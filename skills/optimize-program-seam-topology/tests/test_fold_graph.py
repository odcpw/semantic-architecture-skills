from __future__ import annotations

import copy
import importlib.util
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).parents[1] / "scripts" / "fold_graph.py"
SPEC = importlib.util.spec_from_file_location("fold_graph", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
fold_graph = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = fold_graph
SPEC.loader.exec_module(fold_graph)


class FoldGraphV3Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.graph, self.candidate = fold_graph.self_test_documents()

    def score(self):
        return fold_graph.score_model(
            fold_graph.build_model(self.graph, self.candidate)
        )

    def rebind_evidence(self) -> None:
        semantics_sha256 = fold_graph.candidate_semantics_sha256(
            self.graph,
            self.candidate,
        )
        for binding in self.candidate["evidence"].values():
            binding["candidate_semantics_sha256"] = semantics_sha256

    def test_self_test_fixture_is_feasible_and_identity_bound(self) -> None:
        result = self.score()
        self.assertTrue(result["feasible"])
        self.assertEqual(
            result["input_identity"]["graph_sha256"],
            fold_graph.canonical_sha256(
                fold_graph.normalized_graph_identity(self.graph)
            ),
        )
        self.assertEqual(
            result["input_identity"]["candidate_sha256"],
            fold_graph.canonical_sha256(
                fold_graph.normalized_candidate_identity(self.candidate)
            ),
        )

    def test_duplicate_json_members_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text(
                '{"schema_version":2,"schema_version":2}',
                encoding="utf-8",
            )
            with self.assertRaisesRegex(
                fold_graph.InputError,
                "duplicate JSON object key",
            ):
                fold_graph.load_json(str(path))

    def test_unknown_candidate_field_is_rejected(self) -> None:
        self.candidate["proof"] = "trust me"
        with self.assertRaisesRegex(fold_graph.InputError, "unknown fields"):
            fold_graph.build_model(self.graph, self.candidate)

    def test_orphan_evidence_is_rejected(self) -> None:
        self.candidate["evidence"]["orphan"] = copy.deepcopy(
            self.candidate["evidence"]["trace"]
        )
        with self.assertRaisesRegex(
            fold_graph.InputError, "no semantic constraint owner"
        ):
            self.score()

    def test_evidence_must_bind_candidate_subject(self) -> None:
        self.candidate["evidence"]["trace"]["subject_identity"] = "stale-candidate"
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "must equal candidate.subject_identity",
        ):
            self.score()

    def test_required_closure_must_match_exactly(self) -> None:
        self.candidate["evidence"]["trace"]["closure"] = "observed_only"
        with self.assertRaisesRegex(fold_graph.InputError, "must exactly equal"):
            self.score()

    def test_negative_control_cannot_be_shared(self) -> None:
        second_constraint = copy.deepcopy(self.graph["semantic_constraints"][0])
        second_constraint["id"] = "trace-two"
        self.graph["semantic_constraints"].append(second_constraint)
        second_evidence = copy.deepcopy(self.candidate["evidence"]["trace"])
        second_evidence["semantic_constraint_id"] = "trace-two"
        second_evidence["claim_id"] = "trace-two-claim"
        self.candidate["evidence"]["trace-two"] = second_evidence
        self.rebind_evidence()
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "declared by multiple semantic constraints",
        ):
            self.score()

    def test_negative_control_cannot_reuse_positive_receipt(self) -> None:
        evidence = self.candidate["evidence"]["trace"]
        evidence["negative_control"]["receipt_sha256"] = evidence["receipt_sha256"]
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "must differ from the positive claim receipt",
        ):
            self.score()

    def test_prior_rejected_decision_is_identity_bound(self) -> None:
        before = self.score()
        self.graph["decision"]["prior_decision_receipts"].append(
            {
                "id": "typed-routing-expansion",
                "sha256": "f" * 64,
                "disposition": "rejected",
            }
        )
        changed_semantics = fold_graph.candidate_semantics_sha256(
            self.graph,
            self.candidate,
        )
        self.assertNotEqual(
            before["input_identity"]["candidate_semantics_sha256"],
            changed_semantics,
        )
        self.rebind_evidence()
        after = self.score()
        self.assertNotEqual(
            before["input_identity"]["graph_sha256"],
            after["input_identity"]["graph_sha256"],
        )
        self.assertEqual(before["objectives"], after["objectives"])

    def test_nonclaim_changes_identity_not_objectives(self) -> None:
        before = self.score()
        self.candidate["nonclaims"].append("another explicit limitation")
        after = self.score()
        self.assertNotEqual(
            before["input_identity"]["candidate_sha256"],
            after["input_identity"]["candidate_sha256"],
        )
        self.assertEqual(before["objectives"], after["objectives"])

    def test_identity_is_stable_across_object_key_order(self) -> None:
        reordered = json.loads(
            json.dumps(self.candidate, sort_keys=True),
        )
        self.assertEqual(
            fold_graph.canonical_sha256(self.candidate),
            fold_graph.canonical_sha256(reordered),
        )

    def test_input_identity_is_stable_across_set_declaration_order(self) -> None:
        before = self.score()
        self.graph["vertices"].reverse()
        self.graph["hyperedges"].reverse()
        self.graph["constraints"].reverse()
        self.graph["semantic_constraints"].reverse()
        self.graph["decision"]["protected_observations"].reverse()
        self.candidate["nonclaims"].reverse()
        after = self.score()
        self.assertEqual(before["input_identity"], after["input_identity"])
        self.assertEqual(before["objectives"], after["objectives"])

    def test_stationarity_is_one_pass_and_second_pass_is_confirmation(self) -> None:
        model = fold_graph.build_model(self.graph, self.candidate)
        first = fold_graph.evaluate_stationarity(model, None)
        self.assertEqual(first["status"], "bounded_single_move_pareto_stationary")
        self.assertTrue(first["pareto_stationary"])
        self.assertFalse(first["idempotently_confirmed"])
        second = fold_graph.evaluate_stationarity(model, first["state_sha256"])
        self.assertEqual(
            second["status"],
            "bounded_single_move_pareto_stationary_confirmed",
        )
        self.assertTrue(second["pareto_stationary"])
        self.assertTrue(second["idempotently_confirmed"])
        self.assertTrue(second["generator"]["exhausted"])

    def test_stationarity_confirmation_rejects_stale_state(self) -> None:
        model = fold_graph.build_model(self.graph, self.candidate)
        result = fold_graph.evaluate_stationarity(model, "0" * 64)
        self.assertEqual(result["status"], "state_changed")
        self.assertFalse(result["pareto_stationary"])

    def test_dominating_move_prevents_stationarity(self) -> None:
        self.graph["vertices"][3]["move_cost"] = 0
        self.rebind_evidence()
        model = fold_graph.build_model(self.graph, self.candidate)
        first = fold_graph.evaluate_stationarity(model, None)
        second = fold_graph.evaluate_stationarity(model, first["state_sha256"])
        self.assertEqual(
            second["status"],
            "open_structural_dominator_pending_evidence",
        )
        self.assertFalse(second["pareto_stationary"])
        self.assertGreater(len(second["dominating_candidates"]), 0)

    def test_converge_cli_closes_only_with_exact_prior_state(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            graph_path = root / "graph.json"
            candidate_path = root / "candidate.json"
            graph_path.write_text(json.dumps(self.graph), encoding="utf-8")
            candidate_path.write_text(json.dumps(self.candidate), encoding="utf-8")
            base_command = [
                sys.executable,
                str(SCRIPT),
                "converge",
                "--graph",
                str(graph_path),
                "--candidate",
                str(candidate_path),
            ]
            first = subprocess.run(
                base_command,
                check=True,
                capture_output=True,
                text=True,
                timeout=10,
            )
            first_result = json.loads(first.stdout)
            self.assertEqual(
                first_result["status"],
                "bounded_single_move_pareto_stationary",
            )
            second = subprocess.run(
                [
                    *base_command,
                    "--prior-state-sha256",
                    first_result["state_sha256"],
                ],
                check=True,
                capture_output=True,
                text=True,
                timeout=10,
            )
            second_result = json.loads(second.stdout)
            self.assertEqual(
                second_result["status"],
                "bounded_single_move_pareto_stationary_confirmed",
            )

    def test_every_protected_observation_requires_evidence_coverage(self) -> None:
        self.graph["semantic_constraints"] = []
        self.candidate["evidence"] = {}
        result = self.score()
        self.assertFalse(result["feasible"])
        self.assertTrue(
            any(
                violation["code"] == "uncovered_protected_observation"
                for violation in result["violations"]
            )
        )
        convergence = fold_graph.evaluate_stationarity(
            fold_graph.build_model(self.graph, self.candidate),
            None,
        )
        self.assertEqual(convergence["status"], "blocked_evidence")
        self.assertFalse(convergence["pareto_stationary"])

    def test_presentation_rounding_cannot_hide_a_true_dominator(self) -> None:
        self.graph["module_overhead_bits"] = 0
        self.graph["vertices"] = [
            {
                "id": "left",
                "kind": "state",
                "current_module": "left",
                "move_cost": 0,
            },
            {
                "id": "right",
                "kind": "state",
                "current_module": "right",
                "move_cost": 0,
            },
        ]
        self.graph["constraints"] = []
        self.graph["hyperedges"] = [
            {
                "id": "tiny-cut",
                "kind": "call",
                "members": ["left", "right"],
                "frequency": 1,
                "runtime_cost": 4e-10,
            }
        ]
        self.candidate["assignments"] = {"left": "left", "right": "right"}
        self.candidate["replacements"] = {}
        self.rebind_evidence()
        model = fold_graph.build_model(self.graph, self.candidate)
        result = fold_graph.evaluate_stationarity(model, None)
        self.assertEqual(
            result["status"],
            "open_structural_dominator_pending_evidence",
        )
        self.assertGreater(len(result["dominating_candidates"]), 0)
        self.assertEqual(
            result["baseline_objective_rationals"]["runtime_cut_cost"],
            "1/2500000000",
        )
        self.graph["hyperedges"][0]["runtime_cost"] = 9_007_199_254_740_993
        self.rebind_evidence()
        large = self.score()
        self.assertEqual(
            large["objective_rationals"]["runtime_cut_cost"],
            "9007199254740993",
        )

    def test_semantic_evidence_cannot_survive_topology_mutation(self) -> None:
        self.candidate["assignments"]["spell"] = "core"
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "exact candidate semantics",
        ):
            self.score()

    def test_semantic_evidence_cannot_survive_graph_edge_mutation(self) -> None:
        self.graph["hyperedges"][0]["runtime_cost"] = 99
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "exact candidate semantics",
        ):
            self.score()

    def test_semantic_evidence_cannot_survive_observation_manifest_drift(self) -> None:
        self.graph["decision"]["protected_observations"][0]["manifest_sha256"] = (
            "7" * 64
        )
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "exact candidate semantics",
        ):
            self.score()

    def test_distinct_control_aliases_cannot_share_one_receipt(self) -> None:
        second_constraint = copy.deepcopy(self.graph["semantic_constraints"][0])
        second_constraint["id"] = "trace-two"
        second_constraint["negative_control_id"] = "trace-two-mutant"
        self.graph["semantic_constraints"].append(second_constraint)
        second_evidence = copy.deepcopy(self.candidate["evidence"]["trace"])
        second_evidence["semantic_constraint_id"] = "trace-two"
        second_evidence["claim_id"] = "trace-two-claim"
        second_evidence["receipt_sha256"] = "f" * 64
        second_evidence["negative_control"]["id"] = "trace-two-mutant"
        self.candidate["evidence"]["trace-two"] = second_evidence
        self.rebind_evidence()
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "control receipt",
        ):
            self.score()

    def test_generated_moves_do_not_reuse_baseline_evidence(self) -> None:
        moves = fold_graph.enumerate_moves(
            fold_graph.build_model(self.graph, self.candidate),
            10,
        )
        self.assertGreater(moves["frontier_total"], 0)
        self.assertTrue(
            all(item["evidence_status"] == "pending" for item in moves["frontier"])
        )
        self.assertTrue(
            all(
                item["candidate_semantics_sha256"]
                != moves["input_identity"]["candidate_semantics_sha256"]
                for item in moves["frontier"]
            )
        )

    def test_checked_certificate_does_not_require_empirical_control(self) -> None:
        constraint = self.graph["semantic_constraints"][0]
        constraint["evidence_kind"] = "checked_certificate"
        del constraint["negative_control_id"]
        evidence = self.candidate["evidence"]["trace"]
        evidence["kind"] = "checked_certificate"
        evidence["commands"] = []
        evidence["artifacts"] = ["kerf-certificate.json"]
        del evidence["negative_control"]
        self.rebind_evidence()
        self.assertTrue(self.score()["feasible"])

    def test_source_identity_must_be_content_bound(self) -> None:
        self.graph["decision"]["source_identity"] = "main"
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "content-bound object",
        ):
            self.score()

    def test_source_identity_requires_explicit_working_tree_state(self) -> None:
        source = self.graph["decision"]["source_identity"]
        del source["working_tree_state"]
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "missing required fields",
        ):
            self.score()

    def test_dirty_source_identity_requires_patch_hash(self) -> None:
        source = self.graph["decision"]["source_identity"]
        source["kind"] = "git_tree"
        source["working_tree_state"] = "dirty"
        with self.assertRaises(fold_graph.InputError) as raised:
            self.score()
        self.assertEqual(
            raised.exception.path,
            "graph.decision.source_identity.dirty_patch_sha256",
        )

    def test_cut_receipt_preserves_directed_endpoints(self) -> None:
        edge = self.graph["hyperedges"][0]
        edge["tails"] = ["schedule", "terrain"]
        edge["heads"] = ["spell"]
        self.rebind_evidence()
        cut = self.score()["cuts"][0]
        self.assertEqual(cut["tails"], ["schedule", "terrain"])
        self.assertEqual(cut["heads"], ["spell"])
        self.assertEqual(cut["resolved_tails"], ["schedule", "terrain"])
        self.assertEqual(cut["resolved_heads"], ["spell"])

    def test_unsupported_selection_rule_is_rejected(self) -> None:
        self.graph["decision"]["selection_rule"] = (
            "lexicographic mutable_cut then ordered_cut"
        )
        with self.assertRaisesRegex(
            fold_graph.InputError,
            "selection_rule",
        ):
            self.score()


if __name__ == "__main__":
    unittest.main()
