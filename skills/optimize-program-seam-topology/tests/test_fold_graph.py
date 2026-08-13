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


class FoldGraphV2Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.graph, self.candidate = fold_graph.self_test_documents()

    def score(self):
        return fold_graph.score_model(
            fold_graph.build_model(self.graph, self.candidate)
        )

    def test_self_test_fixture_is_feasible_and_identity_bound(self) -> None:
        result = self.score()
        self.assertTrue(result["feasible"])
        self.assertEqual(
            result["input_identity"]["graph_sha256"],
            fold_graph.canonical_sha256(self.graph),
        )
        self.assertEqual(
            result["input_identity"]["candidate_sha256"],
            fold_graph.canonical_sha256(self.candidate),
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

    def test_identity_is_stable_across_declaration_order(self) -> None:
        reordered = json.loads(
            json.dumps(self.candidate, sort_keys=True),
        )
        self.assertEqual(
            fold_graph.canonical_sha256(self.candidate),
            fold_graph.canonical_sha256(reordered),
        )

    def test_fixed_point_requires_second_unchanged_evaluation(self) -> None:
        model = fold_graph.build_model(self.graph, self.candidate)
        first = fold_graph.evaluate_fixed_point(model, None)
        self.assertEqual(first["status"], "open")
        self.assertFalse(first["fixed_point"])
        second = fold_graph.evaluate_fixed_point(model, first["state_sha256"])
        self.assertEqual(second["status"], "bounded_single_move_fixed_point")
        self.assertTrue(second["fixed_point"])
        self.assertTrue(second["generator"]["exhausted"])

    def test_fixed_point_rejects_stale_state(self) -> None:
        model = fold_graph.build_model(self.graph, self.candidate)
        result = fold_graph.evaluate_fixed_point(model, "0" * 64)
        self.assertEqual(result["status"], "state_changed")
        self.assertFalse(result["fixed_point"])

    def test_dominating_move_prevents_fixed_point(self) -> None:
        self.graph["vertices"][3]["move_cost"] = 0
        model = fold_graph.build_model(self.graph, self.candidate)
        first = fold_graph.evaluate_fixed_point(model, None)
        second = fold_graph.evaluate_fixed_point(model, first["state_sha256"])
        self.assertEqual(second["status"], "open_dominating_candidate")
        self.assertFalse(second["fixed_point"])
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
            self.assertEqual(first_result["status"], "open")
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
                "bounded_single_move_fixed_point",
            )


if __name__ == "__main__":
    unittest.main()
