#!/usr/bin/env python3
"""Validate and score declared program-seam graph candidates.

This is a deterministic decision aid. It validates the declared model and
checks that required evidence records exist; it does not prove semantics.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import sys
from collections.abc import Iterable
from dataclasses import dataclass
from pathlib import Path
from typing import Any

TOOL_VERSION = "2.0.0"
SCHEMA_VERSION = 2
OBJECTIVE_NAMES = (
    "duplicate_authority",
    "sync_work",
    "mutable_cut",
    "ordered_cut",
    "boundary_information_bits_per_step",
    "mdl_bits",
    "special_case_bits",
    "contract_churn",
    "edit_contention",
    "runtime_cut_cost",
    "memory_bytes",
    "expected_rebuild_ms",
    "migration_cost",
    "proof_deficit",
)
ALLOWED_CLOSURES = {
    "observed_only",
    "sampled",
    "complete_for_declared_corpus",
    "exhaustive_finite",
    "sound_overapproximation",
}


class InputError(Exception):
    """A typed user/model input error."""

    def __init__(self, code: str, message: str, path: str | None = None):
        super().__init__(message)
        self.code = code
        self.message = message
        self.path = path


@dataclass(frozen=True)
class Model:
    graph: dict[str, Any]
    candidate: dict[str, Any]
    vertices: dict[str, dict[str, Any]]
    representatives: dict[str, str]
    assignments: dict[str, str]
    active: tuple[str, ...]


def reject_unknown_fields(value: dict[str, Any], allowed: set[str], path: str) -> None:
    unknown = sorted(set(value) - allowed)
    if unknown:
        raise InputError(
            "unknown_field",
            f"unknown fields: {unknown}",
            path,
        )


def emit_error(error: InputError) -> int:
    payload: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "tool_version": TOOL_VERSION,
        "ok": False,
        "error": {"code": error.code, "message": error.message},
    }
    if error.path:
        payload["error"]["path"] = error.path
    print(json.dumps(payload, sort_keys=True), file=sys.stderr)
    return 2


def load_json(path: str) -> dict[str, Any]:
    try:

        def no_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
            result: dict[str, Any] = {}
            for key, value in pairs:
                if key in result:
                    raise InputError(
                        "duplicate_json_key",
                        f"duplicate JSON object key {key!r}",
                        path,
                    )
                result[key] = value
            return result

        data = json.loads(
            Path(path).read_text(encoding="utf-8"),
            object_pairs_hook=no_duplicates,
        )
    except OSError as exc:
        raise InputError("read_failed", str(exc), path) from exc
    except json.JSONDecodeError as exc:
        raise InputError(
            "invalid_json", f"{exc.msg} at line {exc.lineno}, column {exc.colno}", path
        ) from exc
    if not isinstance(data, dict):
        raise InputError("invalid_root", "JSON root must be an object", path)
    return data


def canonical_sha256(value: Any) -> str:
    encoded = json.dumps(
        value,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def evaluator_identity() -> dict[str, str]:
    script = Path(__file__).resolve()
    skill = script.parent.parent / "SKILL.md"
    try:
        script_sha256 = hashlib.sha256(script.read_bytes()).hexdigest()
        skill_sha256 = hashlib.sha256(skill.read_bytes()).hexdigest()
    except OSError as exc:
        raise InputError("identity_failed", str(exc), str(exc.filename)) from exc
    return {
        "evaluator_sha256": script_sha256,
        "skill_sha256": skill_sha256,
    }


def input_identity(graph: dict[str, Any], candidate: dict[str, Any]) -> dict[str, str]:
    return {
        "graph_sha256": canonical_sha256(graph),
        "candidate_sha256": canonical_sha256(candidate),
        **evaluator_identity(),
    }


def require_version(document: dict[str, Any], name: str) -> None:
    if document.get("schema_version") != SCHEMA_VERSION:
        raise InputError(
            "unsupported_schema",
            f"{name}.schema_version must equal {SCHEMA_VERSION}",
            f"{name}.schema_version",
        )


def number(value: Any, path: str, default: float = 0.0) -> float:
    if value is None:
        return default
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise InputError(
            "invalid_number", "expected a finite non-negative number", path
        )
    result = float(value)
    if not math.isfinite(result) or result < 0:
        raise InputError(
            "invalid_number", "expected a finite non-negative number", path
        )
    return result


def probability(value: Any, path: str) -> float:
    result = number(value, path)
    if result > 1:
        raise InputError("invalid_probability", "expected a number from 0 to 1", path)
    return result


def string_list(value: Any, path: str, *, nonempty: bool = False) -> list[str]:
    if not isinstance(value, list) or any(
        not isinstance(item, str) or not item for item in value
    ):
        raise InputError(
            "invalid_string_list", "expected a list of non-empty strings", path
        )
    if nonempty and not value:
        raise InputError("empty_list", "list must not be empty", path)
    if len(set(value)) != len(value):
        raise InputError("duplicate_item", "list items must be unique", path)
    return value


def nonempty_string(value: Any, path: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise InputError("invalid_string", "expected a non-empty string", path)
    return value


def sha256_string(value: Any, path: str) -> str:
    result = nonempty_string(value, path)
    if len(result) != 64 or any(
        character not in "0123456789abcdef" for character in result
    ):
        raise InputError(
            "invalid_sha256",
            "expected 64 lowercase hexadecimal SHA-256 characters",
            path,
        )
    return result


def validate_decision(graph: dict[str, Any]) -> None:
    decision = graph.get("decision")
    if not isinstance(decision, dict):
        raise InputError(
            "invalid_decision",
            "graph.decision must be an object",
            "graph.decision",
        )
    reject_unknown_fields(
        decision,
        {
            "id",
            "consumer",
            "retirement_condition",
            "source_identity",
            "protected_observations",
            "selection_rule",
            "prior_decision_receipts",
        },
        "graph.decision",
    )
    for field in (
        "id",
        "consumer",
        "retirement_condition",
        "source_identity",
        "selection_rule",
    ):
        nonempty_string(decision.get(field), f"graph.decision.{field}")
    string_list(
        decision.get("protected_observations"),
        "graph.decision.protected_observations",
        nonempty=True,
    )
    receipts = decision.get("prior_decision_receipts")
    if not isinstance(receipts, list):
        raise InputError(
            "invalid_prior_decision_receipts",
            "prior_decision_receipts must be a list",
            "graph.decision.prior_decision_receipts",
        )
    seen: set[str] = set()
    for index, receipt in enumerate(receipts):
        path = f"graph.decision.prior_decision_receipts[{index}]"
        if not isinstance(receipt, dict):
            raise InputError(
                "invalid_prior_decision_receipt",
                "prior decision receipt must be an object",
                path,
            )
        reject_unknown_fields(receipt, {"id", "sha256", "disposition"}, path)
        receipt_id = nonempty_string(receipt.get("id"), f"{path}.id")
        if receipt_id in seen:
            raise InputError(
                "duplicate_prior_decision_receipt",
                f"duplicate prior decision receipt {receipt_id!r}",
                f"{path}.id",
            )
        seen.add(receipt_id)
        sha256_string(receipt.get("sha256"), f"{path}.sha256")
        if receipt.get("disposition") not in {"accepted", "rejected", "superseded"}:
            raise InputError(
                "invalid_prior_decision_disposition",
                "disposition must be accepted, rejected, or superseded",
                f"{path}.disposition",
            )


def index_vertices(graph: dict[str, Any]) -> dict[str, dict[str, Any]]:
    raw = graph.get("vertices")
    if not isinstance(raw, list) or not raw:
        raise InputError(
            "invalid_vertices",
            "graph.vertices must be a non-empty list",
            "graph.vertices",
        )
    result: dict[str, dict[str, Any]] = {}
    for index, vertex in enumerate(raw):
        path = f"graph.vertices[{index}]"
        if not isinstance(vertex, dict):
            raise InputError("invalid_vertex", "vertex must be an object", path)
        reject_unknown_fields(
            vertex,
            {
                "id",
                "kind",
                "current_module",
                "mutable",
                "authority_key",
                "replacement_allowed",
                "pinned_module",
                "description_bits",
                "special_case_bits",
                "compile_cost_ms",
                "change_probability",
                "memory_bytes",
                "move_cost",
                "public_contract",
                "contract_churn_cost",
            },
            path,
        )
        vertex_id = vertex.get("id")
        if not isinstance(vertex_id, str) or not vertex_id:
            raise InputError(
                "invalid_vertex_id",
                "vertex.id must be a non-empty string",
                f"{path}.id",
            )
        if vertex_id in result:
            raise InputError(
                "duplicate_vertex", f"duplicate vertex id {vertex_id!r}", f"{path}.id"
            )
        kind = vertex.get("kind")
        if not isinstance(kind, str) or not kind:
            raise InputError(
                "invalid_vertex_kind",
                "vertex.kind must be a non-empty string",
                f"{path}.kind",
            )
        for field in (
            "description_bits",
            "special_case_bits",
            "compile_cost_ms",
            "contract_churn_cost",
            "memory_bytes",
            "move_cost",
        ):
            number(vertex.get(field), f"{path}.{field}")
        probability(vertex.get("change_probability"), f"{path}.change_probability")
        if "replacement_allowed" in vertex and not isinstance(
            vertex["replacement_allowed"], bool
        ):
            raise InputError(
                "invalid_boolean",
                "replacement_allowed must be a boolean",
                f"{path}.replacement_allowed",
            )
        for field in ("mutable", "public_contract"):
            if field in vertex and not isinstance(vertex[field], bool):
                raise InputError(
                    "invalid_boolean",
                    f"{field} must be a boolean",
                    f"{path}.{field}",
                )
        for field in (
            "current_module",
            "authority_key",
            "pinned_module",
        ):
            if field in vertex:
                nonempty_string(vertex[field], f"{path}.{field}")
        result[vertex_id] = vertex
    return result


def resolve_representatives(
    vertices: dict[str, dict[str, Any]], replacements: dict[str, Any]
) -> dict[str, str]:
    for old, new in replacements.items():
        if old not in vertices:
            raise InputError(
                "unknown_replacement_source",
                f"unknown vertex {old!r}",
                f"candidate.replacements.{old}",
            )
        if not isinstance(new, str) or new not in vertices:
            raise InputError(
                "unknown_replacement_target",
                f"unknown replacement target {new!r}",
                f"candidate.replacements.{old}",
            )
        if not vertices[old].get("replacement_allowed", True):
            raise InputError(
                "replacement_forbidden",
                f"vertex {old!r} forbids replacement",
                f"candidate.replacements.{old}",
            )

    resolved: dict[str, str] = {}

    def visit(vertex_id: str, trail: tuple[str, ...]) -> str:
        if vertex_id in resolved:
            return resolved[vertex_id]
        if vertex_id in trail:
            cycle = " -> ".join((*trail, vertex_id))
            raise InputError(
                "replacement_cycle",
                f"replacement cycle: {cycle}",
                "candidate.replacements",
            )
        target = replacements.get(vertex_id)
        representative = (
            vertex_id if target is None else visit(target, (*trail, vertex_id))
        )
        resolved[vertex_id] = representative
        return representative

    for vertex_id in vertices:
        visit(vertex_id, ())
    return resolved


def build_model(graph: dict[str, Any], candidate: dict[str, Any]) -> Model:
    require_version(graph, "graph")
    require_version(candidate, "candidate")
    reject_unknown_fields(
        graph,
        {
            "schema_version",
            "decision",
            "module_overhead_bits",
            "semantic_steps",
            "vertices",
            "hyperedges",
            "constraints",
            "semantic_constraints",
            "nonclaims",
        },
        "graph",
    )
    reject_unknown_fields(
        candidate,
        {
            "schema_version",
            "id",
            "subject_identity",
            "assignments",
            "replacements",
            "evidence",
            "adapters",
            "nonclaims",
        },
        "candidate",
    )
    validate_decision(graph)
    nonempty_string(candidate.get("id"), "candidate.id")
    nonempty_string(candidate.get("subject_identity"), "candidate.subject_identity")
    string_list(graph.get("nonclaims"), "graph.nonclaims", nonempty=True)
    string_list(candidate.get("nonclaims"), "candidate.nonclaims", nonempty=True)
    vertices = index_vertices(graph)
    replacements = candidate.get("replacements", {})
    assignments = candidate.get("assignments")
    if not isinstance(replacements, dict):
        raise InputError(
            "invalid_replacements",
            "candidate.replacements must be an object",
            "candidate.replacements",
        )
    if not isinstance(assignments, dict):
        raise InputError(
            "invalid_assignments",
            "candidate.assignments must be an object",
            "candidate.assignments",
        )
    representatives = resolve_representatives(vertices, replacements)
    active = tuple(sorted({representatives[vertex_id] for vertex_id in vertices}))

    normalized: dict[str, str] = {}
    for representative in active:
        module = assignments.get(representative)
        if not isinstance(module, str) or not module:
            raise InputError(
                "missing_assignment",
                f"active representative {representative!r} needs a non-empty module assignment",
                f"candidate.assignments.{representative}",
            )
        normalized[representative] = module
    unknown = sorted(set(assignments) - set(vertices))
    if unknown:
        raise InputError(
            "unknown_assignment",
            f"assignments contain unknown vertices: {unknown}",
            "candidate.assignments",
        )
    return Model(graph, candidate, vertices, representatives, normalized, active)


def resolved_vertex(model: Model, vertex_id: str, path: str) -> str:
    if vertex_id not in model.vertices:
        raise InputError("unknown_vertex", f"unknown vertex {vertex_id!r}", path)
    return model.representatives[vertex_id]


def module_of(model: Model, vertex_id: str, path: str) -> str:
    return model.assignments[resolved_vertex(model, vertex_id, path)]


def validate_hyperedges(model: Model) -> list[dict[str, Any]]:
    raw = model.graph.get("hyperedges", [])
    if not isinstance(raw, list):
        raise InputError(
            "invalid_hyperedges", "graph.hyperedges must be a list", "graph.hyperedges"
        )
    seen: set[str] = set()
    edges: list[dict[str, Any]] = []
    for index, edge in enumerate(raw):
        path = f"graph.hyperedges[{index}]"
        if not isinstance(edge, dict):
            raise InputError("invalid_hyperedge", "hyperedge must be an object", path)
        reject_unknown_fields(
            edge,
            {
                "id",
                "kind",
                "members",
                "tails",
                "heads",
                "weight",
                "frequency",
                "volume",
                "mutable",
                "ordered",
                "runtime_cost",
                "protocol_bits",
                "boundary_information_bits",
                "boundary_memory_bytes",
                "cut_policy",
                "penalty_mode",
                "layers",
            },
            path,
        )
        edge_id = edge.get("id")
        kind = edge.get("kind")
        if not isinstance(edge_id, str) or not edge_id:
            raise InputError(
                "invalid_hyperedge_id",
                "hyperedge.id must be a non-empty string",
                f"{path}.id",
            )
        if edge_id in seen:
            raise InputError(
                "duplicate_hyperedge",
                f"duplicate hyperedge id {edge_id!r}",
                f"{path}.id",
            )
        seen.add(edge_id)
        if not isinstance(kind, str) or not kind:
            raise InputError(
                "invalid_hyperedge_kind",
                "hyperedge.kind must be a non-empty string",
                f"{path}.kind",
            )
        members = string_list(edge.get("members"), f"{path}.members", nonempty=True)
        for member in members:
            resolved_vertex(model, member, f"{path}.members")
        for field in ("tails", "heads"):
            if field in edge:
                values = string_list(edge[field], f"{path}.{field}")
                if not set(values).issubset(members):
                    raise InputError(
                        "edge_endpoint_not_member",
                        f"{field} must be a subset of members",
                        f"{path}.{field}",
                    )
        for field in ("mutable", "ordered"):
            if field in edge and not isinstance(edge[field], bool):
                raise InputError(
                    "invalid_boolean",
                    f"{field} must be a boolean",
                    f"{path}.{field}",
                )
        if "layers" in edge:
            string_list(edge["layers"], f"{path}.layers")
        for field in (
            "weight",
            "frequency",
            "volume",
            "runtime_cost",
            "protocol_bits",
            "boundary_information_bits",
            "boundary_memory_bytes",
        ):
            number(
                edge.get(field), f"{path}.{field}", 1.0 if field == "weight" else 0.0
            )
        policy = edge.get("cut_policy", "allow")
        if policy not in ("allow", "forbid"):
            raise InputError(
                "invalid_cut_policy",
                "cut_policy must be 'allow' or 'forbid'",
                f"{path}.cut_policy",
            )
        penalty_mode = edge.get("penalty_mode", "cut")
        if penalty_mode not in ("cut", "co_location"):
            raise InputError(
                "invalid_penalty_mode",
                "penalty_mode must be 'cut' or 'co_location'",
                f"{path}.penalty_mode",
            )
        edges.append(edge)
    return edges


def validate_evidence_binding(
    value: Any,
    path: str,
    *,
    semantic_id: str,
    subject_identity: str,
    required_closure: str,
    negative_control_id: str,
) -> tuple[str, str]:
    if not isinstance(value, dict):
        raise InputError(
            "invalid_evidence_binding",
            "evidence binding must be an object",
            path,
        )
    reject_unknown_fields(
        value,
        {
            "semantic_constraint_id",
            "subject_identity",
            "claim_id",
            "closure",
            "status",
            "plan_sha256",
            "receipt_sha256",
            "evaluator_sha256",
            "skill_sha256",
            "commands",
            "artifacts",
            "negative_control",
        },
        path,
    )
    if value.get("semantic_constraint_id") != semantic_id:
        raise InputError(
            "evidence_subject_mismatch",
            f"semantic_constraint_id must equal {semantic_id!r}",
            f"{path}.semantic_constraint_id",
        )
    if value.get("subject_identity") != subject_identity:
        raise InputError(
            "evidence_subject_mismatch",
            "evidence subject_identity must equal candidate.subject_identity",
            f"{path}.subject_identity",
        )
    claim_id = nonempty_string(value.get("claim_id"), f"{path}.claim_id")
    closure = nonempty_string(value.get("closure"), f"{path}.closure")
    if closure not in ALLOWED_CLOSURES:
        raise InputError(
            "invalid_closure",
            f"closure must be one of {sorted(ALLOWED_CLOSURES)}",
            f"{path}.closure",
        )
    if closure != required_closure:
        raise InputError(
            "closure_mismatch",
            f"evidence closure {closure!r} must exactly equal required closure {required_closure!r}",
            f"{path}.closure",
        )
    if value.get("status") != "accepted":
        raise InputError(
            "evidence_not_accepted",
            "evidence status must be 'accepted'",
            f"{path}.status",
        )
    for field in (
        "plan_sha256",
        "receipt_sha256",
        "evaluator_sha256",
        "skill_sha256",
    ):
        sha256_string(value.get(field), f"{path}.{field}")
    commands = string_list(value.get("commands"), f"{path}.commands")
    artifacts = string_list(value.get("artifacts"), f"{path}.artifacts")
    if not commands and not artifacts:
        raise InputError(
            "zero_information_evidence",
            "evidence needs at least one command or artifact",
            path,
        )
    control = value.get("negative_control")
    control_path = f"{path}.negative_control"
    if not isinstance(control, dict):
        raise InputError(
            "invalid_negative_control",
            "negative_control must be an object",
            control_path,
        )
    reject_unknown_fields(
        control,
        {"id", "status", "receipt_sha256"},
        control_path,
    )
    control_id = nonempty_string(control.get("id"), f"{control_path}.id")
    if control_id != negative_control_id:
        raise InputError(
            "negative_control_mismatch",
            f"negative control must equal {negative_control_id!r}",
            f"{control_path}.id",
        )
    if control.get("status") != "passed":
        raise InputError(
            "negative_control_failed",
            "negative control status must be 'passed'",
            f"{control_path}.status",
        )
    control_receipt = sha256_string(
        control.get("receipt_sha256"),
        f"{control_path}.receipt_sha256",
    )
    if control_receipt == value["receipt_sha256"]:
        raise InputError(
            "negative_control_reuses_claim_receipt",
            "negative control receipt must differ from the positive claim receipt",
            f"{control_path}.receipt_sha256",
        )
    return claim_id, control_id


def constraint_violations(
    model: Model, edges: list[dict[str, Any]]
) -> tuple[list[dict[str, Any]], int]:
    violations: list[dict[str, Any]] = []

    def add(code: str, subject: str, message: str) -> None:
        violations.append({"code": code, "subject": subject, "message": message})

    for vertex_id, vertex in model.vertices.items():
        representative = model.representatives[vertex_id]
        if not vertex.get("replacement_allowed", True) and representative != vertex_id:
            add("replacement_forbidden", vertex_id, "vertex was replaced")
        pinned = vertex.get("pinned_module")
        if pinned is not None:
            if not isinstance(pinned, str) or not pinned:
                raise InputError(
                    "invalid_pinned_module",
                    "pinned_module must be a non-empty string",
                    f"vertex.{vertex_id}.pinned_module",
                )
            actual = model.assignments[representative]
            if (
                actual != pinned
            ):  # ubs:ignore — module labels are public architecture data, not secrets.
                add(
                    "pin_violated",
                    vertex_id,
                    f"expected module {pinned!r}, found {actual!r}",
                )

    authority_groups: dict[str, set[str]] = {}
    for vertex_id, vertex in model.vertices.items():
        key = vertex.get("authority_key")
        if key is not None:
            if not isinstance(key, str) or not key:
                raise InputError(
                    "invalid_authority_key",
                    "authority_key must be a non-empty string",
                    f"vertex.{vertex_id}.authority_key",
                )
            authority_groups.setdefault(key, set()).add(
                model.representatives[vertex_id]
            )
    duplicate_authority = 0
    for key, representatives in sorted(authority_groups.items()):
        excess = max(0, len(representatives) - 1)
        duplicate_authority += excess
        if excess:
            add(
                "duplicate_authority",
                key,
                f"authority resolves to {sorted(representatives)}",
            )

    raw_constraints = model.graph.get("constraints", [])
    if not isinstance(raw_constraints, list):
        raise InputError(
            "invalid_constraints",
            "graph.constraints must be a list",
            "graph.constraints",
        )
    seen_constraints: set[str] = set()
    for index, constraint in enumerate(raw_constraints):
        path = f"graph.constraints[{index}]"
        if not isinstance(constraint, dict):
            raise InputError("invalid_constraint", "constraint must be an object", path)
        reject_unknown_fields(
            constraint,
            {"id", "kind", "vertices", "vertex", "module", "modules"},
            path,
        )
        constraint_id = constraint.get("id", f"constraint-{index}")
        kind = constraint.get("kind")
        if not isinstance(constraint_id, str) or not constraint_id:
            raise InputError(
                "invalid_constraint_id",
                "constraint.id must be a non-empty string",
                f"{path}.id",
            )
        if constraint_id in seen_constraints:
            raise InputError(
                "duplicate_constraint",
                f"duplicate constraint id {constraint_id!r}",
                f"{path}.id",
            )
        seen_constraints.add(constraint_id)
        if kind in ("co_locate", "separate"):
            vertices = string_list(
                constraint.get("vertices"), f"{path}.vertices", nonempty=True
            )
            modules = [
                module_of(model, vertex, f"{path}.vertices") for vertex in vertices
            ]
            if kind == "co_locate" and len(set(modules)) > 1:
                add("co_location_violated", constraint_id, f"modules are {modules}")
            if kind == "separate" and len(set(modules)) != len(modules):
                add("separation_violated", constraint_id, f"modules are {modules}")
        elif kind == "pin":
            vertex = constraint.get("vertex")
            module = constraint.get("module")
            if not isinstance(vertex, str) or not isinstance(module, str) or not module:
                raise InputError(
                    "invalid_pin", "pin needs non-empty vertex and module strings", path
                )
            actual = module_of(model, vertex, f"{path}.vertex")
            if actual != module:
                add(
                    "pin_violated",
                    constraint_id,
                    f"expected {module!r}, found {actual!r}",
                )
        elif kind == "allowed_modules":
            vertex = constraint.get("vertex")
            if not isinstance(vertex, str):
                raise InputError(
                    "invalid_allowed_modules",
                    "allowed_modules needs a vertex string",
                    path,
                )
            modules = string_list(
                constraint.get("modules"), f"{path}.modules", nonempty=True
            )
            actual = module_of(model, vertex, f"{path}.vertex")
            if actual not in modules:
                add(
                    "module_not_allowed",
                    constraint_id,
                    f"module {actual!r} not in {modules}",
                )
        elif kind == "forbid_replacement":
            vertex = constraint.get("vertex")
            if not isinstance(vertex, str):
                raise InputError(
                    "invalid_forbid_replacement",
                    "forbid_replacement needs a vertex string",
                    path,
                )
            representative = resolved_vertex(model, vertex, f"{path}.vertex")
            if representative != vertex:
                add(
                    "replacement_forbidden",
                    constraint_id,
                    f"{vertex!r} resolves to {representative!r}",
                )
        else:
            raise InputError(
                "unknown_constraint_kind",
                f"unsupported constraint kind {kind!r}",
                f"{path}.kind",
            )

    for edge in edges:
        modules = {
            module_of(model, member, f"hyperedge.{edge['id']}.members")
            for member in edge["members"]
        }
        if edge.get("cut_policy", "allow") == "forbid" and len(modules) > 1:
            add("forbidden_cut", edge["id"], f"edge spans modules {sorted(modules)}")

    required = model.graph.get("semantic_constraints", [])
    evidence = model.candidate.get("evidence", {})
    if not isinstance(required, list):
        raise InputError(
            "invalid_semantic_constraints",
            "semantic_constraints must be a list",
            "graph.semantic_constraints",
        )
    if not isinstance(evidence, dict):
        raise InputError(
            "invalid_evidence",
            "candidate.evidence must be an object",
            "candidate.evidence",
        )
    proof_deficit = 0
    seen_semantic: set[str] = set()
    seen_claims: set[str] = set()
    seen_controls: set[str] = set()
    declared_controls: set[str] = set()
    for index, item in enumerate(required):
        path = f"graph.semantic_constraints[{index}]"
        if (
            not isinstance(item, dict)
            or not isinstance(item.get("id"), str)
            or not item["id"]
        ):
            raise InputError(
                "invalid_semantic_constraint",
                "semantic constraint needs a non-empty id",
                path,
            )
        reject_unknown_fields(
            item,
            {
                "id",
                "required",
                "observation",
                "required_closure",
                "negative_control_id",
            },
            path,
        )
        semantic_id = item["id"]
        if semantic_id in seen_semantic:
            raise InputError(
                "duplicate_semantic_constraint",
                f"duplicate semantic constraint {semantic_id!r}",
                path,
            )
        seen_semantic.add(semantic_id)
        observation = nonempty_string(item.get("observation"), f"{path}.observation")
        if observation not in model.graph["decision"]["protected_observations"]:
            raise InputError(
                "unknown_protected_observation",
                f"semantic constraint references undeclared observation {observation!r}",
                f"{path}.observation",
            )
        required_closure = nonempty_string(
            item.get("required_closure"),
            f"{path}.required_closure",
        )
        if required_closure not in ALLOWED_CLOSURES:
            raise InputError(
                "invalid_closure",
                f"required_closure must be one of {sorted(ALLOWED_CLOSURES)}",
                f"{path}.required_closure",
            )
        negative_control_id = nonempty_string(
            item.get("negative_control_id"),
            f"{path}.negative_control_id",
        )
        if negative_control_id in declared_controls:
            raise InputError(
                "shared_negative_control",
                f"negative control {negative_control_id!r} is declared by multiple semantic constraints",
                f"{path}.negative_control_id",
            )
        declared_controls.add(negative_control_id)
        required_item = item.get("required", True)
        if not isinstance(required_item, bool):
            raise InputError(
                "invalid_boolean",
                "required must be a boolean",
                f"{path}.required",
            )
        binding = evidence.get(semantic_id)
        if required_item and binding is None:
            proof_deficit += 1
            add(
                "evidence_missing",
                semantic_id,
                "needs an accepted identity-bound claim and passing strict negative control",
            )
            continue
        if binding is not None:
            claim_id, control_id = validate_evidence_binding(
                binding,
                f"candidate.evidence.{semantic_id}",
                semantic_id=semantic_id,
                subject_identity=model.candidate["subject_identity"],
                required_closure=required_closure,
                negative_control_id=negative_control_id,
            )
            if claim_id in seen_claims:
                raise InputError(
                    "shared_claim",
                    f"claim {claim_id!r} is shared across semantic constraints",
                    f"candidate.evidence.{semantic_id}.claim_id",
                )
            if control_id in seen_controls:
                raise InputError(
                    "shared_negative_control",
                    f"negative control {control_id!r} is shared across semantic constraints",
                    f"candidate.evidence.{semantic_id}.negative_control.id",
                )
            seen_claims.add(claim_id)
            seen_controls.add(control_id)
    orphan_evidence = sorted(set(evidence) - seen_semantic)
    if orphan_evidence:
        raise InputError(
            "orphan_evidence",
            f"candidate evidence has no semantic constraint owner: {orphan_evidence}",
            "candidate.evidence",
        )

    adapters = model.candidate.get("adapters", [])
    if not isinstance(adapters, list):
        raise InputError(
            "invalid_adapters",
            "candidate.adapters must be a list",
            "candidate.adapters",
        )
    seen_adapters: set[str] = set()
    for index, adapter in enumerate(adapters):
        path = f"candidate.adapters[{index}]"
        if (
            not isinstance(adapter, dict)
            or not isinstance(adapter.get("id"), str)
            or not adapter["id"]
        ):
            raise InputError("invalid_adapter", "adapter needs a non-empty id", path)
        reject_unknown_fields(adapter, {"id", "cost", "removal_condition"}, path)
        if adapter["id"] in seen_adapters:
            raise InputError(
                "duplicate_adapter",
                f"duplicate adapter id {adapter['id']!r}",
                f"{path}.id",
            )
        seen_adapters.add(adapter["id"])
        removal = adapter.get("removal_condition")
        if not isinstance(removal, str) or not removal.strip():
            add(
                "adapter_without_removal",
                adapter["id"],
                "temporary adapter needs a removal condition",
            )

    return violations, proof_deficit


def score_model(model: Model) -> dict[str, Any]:
    edges = validate_hyperedges(model)
    violations, proof_deficit = constraint_violations(model, edges)
    modules = sorted({model.assignments[vertex_id] for vertex_id in model.active})
    module_overhead = number(
        model.graph.get("module_overhead_bits"), "graph.module_overhead_bits"
    )
    semantic_steps = number(model.graph.get("semantic_steps"), "graph.semantic_steps")
    objectives = {name: 0.0 for name in OBJECTIVE_NAMES}
    objectives["proof_deficit"] = float(proof_deficit)

    authority_groups: dict[str, set[str]] = {}
    for vertex_id, vertex in model.vertices.items():
        key = vertex.get("authority_key")
        if isinstance(key, str):
            authority_groups.setdefault(key, set()).add(
                model.representatives[vertex_id]
            )
    objectives["duplicate_authority"] = float(
        sum(
            max(0, len(representatives) - 1)
            for representatives in authority_groups.values()
        )
    )

    objectives["mdl_bits"] = module_overhead * len(modules)
    module_build: dict[str, list[dict[str, Any]]] = {module: [] for module in modules}
    for vertex_id in model.active:
        vertex = model.vertices[vertex_id]
        module = model.assignments[vertex_id]
        objectives["mdl_bits"] += number(
            vertex.get("description_bits"), f"vertex.{vertex_id}.description_bits"
        )
        objectives["special_case_bits"] += number(
            vertex.get("special_case_bits"), f"vertex.{vertex_id}.special_case_bits"
        )
        objectives["memory_bytes"] += number(
            vertex.get("memory_bytes"), f"vertex.{vertex_id}.memory_bytes"
        )
        module_build[module].append(vertex)

    for vertex_id, vertex in model.vertices.items():
        representative = model.representatives[vertex_id]
        current = vertex.get("current_module")
        changed = (
            representative != vertex_id
            or (
                isinstance(current, str)
                and current
                and model.assignments[representative]
                != current  # ubs:ignore — module labels are public architecture data, not secrets.
            )
        )
        if changed:
            objectives["migration_cost"] += number(
                vertex.get("move_cost"), f"vertex.{vertex_id}.move_cost", 1.0
            )
            if vertex.get("public_contract", False):
                objectives["contract_churn"] += number(
                    vertex.get("contract_churn_cost"),
                    f"vertex.{vertex_id}.contract_churn_cost",
                    1.0,
                )

    for adapter in model.candidate.get("adapters", []):
        objectives["migration_cost"] += number(
            adapter.get("cost"), f"adapter.{adapter.get('id')}.cost", 1.0
        )

    cuts: list[dict[str, Any]] = []
    for edge in edges:
        resolved = sorted({model.representatives[member] for member in edge["members"]})
        edge_modules = sorted({model.assignments[vertex_id] for vertex_id in resolved})
        span = len(edge_modules)
        crossings = max(0, span - 1)
        weight = number(edge.get("weight"), f"edge.{edge['id']}.weight", 1.0)
        frequency = number(edge.get("frequency"), f"edge.{edge['id']}.frequency")
        volume = number(edge.get("volume"), f"edge.{edge['id']}.volume")
        runtime_cost = number(
            edge.get("runtime_cost"), f"edge.{edge['id']}.runtime_cost"
        )
        protocol_bits = number(
            edge.get("protocol_bits"), f"edge.{edge['id']}.protocol_bits"
        )
        boundary_information = number(
            edge.get("boundary_information_bits"),
            f"edge.{edge['id']}.boundary_information_bits",
        )
        boundary_memory = number(
            edge.get("boundary_memory_bytes"),
            f"edge.{edge['id']}.boundary_memory_bytes",
        )
        cut_factor = float(crossings)

        if crossings:
            objectives["mdl_bits"] += protocol_bits * cut_factor
            objectives["boundary_information_bits_per_step"] += (
                boundary_information * cut_factor
            )
            objectives["runtime_cut_cost"] += runtime_cost * frequency * cut_factor
            objectives["memory_bytes"] += boundary_memory * cut_factor
            if edge.get("mutable", False) or edge["kind"] in {
                "read_write",
                "write",
                "sync",
            }:
                objectives["mutable_cut"] += (
                    weight * frequency * max(volume, 1.0) * cut_factor
                )
            if edge.get("ordered", False):
                objectives["ordered_cut"] += weight * max(frequency, 1.0) * cut_factor
            if edge["kind"] == "sync":
                objectives["sync_work"] += (
                    weight * frequency * max(volume, 1.0) * cut_factor
                )

        penalty_mode = edge.get("penalty_mode", "cut")
        penalty_active = crossings > 0 if penalty_mode == "cut" else span == 1
        if penalty_active and edge["kind"] in {"cochange", "edit_collision"}:
            objectives["edit_contention"] += weight * max(frequency, 1.0)

        if crossings:
            cuts.append(
                {
                    "edge": edge["id"],
                    "kind": edge["kind"],
                    "modules": edge_modules,
                    "resolved_members": resolved,
                    "crossings": crossings,
                    "cut_policy": edge.get("cut_policy", "allow"),
                }
            )

    if objectives["boundary_information_bits_per_step"]:
        if semantic_steps <= 0:
            raise InputError(
                "missing_semantic_steps",
                "graph.semantic_steps must be positive when boundary information bits are supplied",
                "graph.semantic_steps",
            )
        objectives["boundary_information_bits_per_step"] /= semantic_steps

    for module, vertices in module_build.items():
        compile_cost = sum(
            number(v.get("compile_cost_ms"), f"module.{module}.compile_cost_ms")
            for v in vertices
        )
        no_change_probability = 1.0
        for vertex in vertices:
            no_change_probability *= 1.0 - probability(
                vertex.get("change_probability"), f"module.{module}.change_probability"
            )
        objectives["expected_rebuild_ms"] += compile_cost * (
            1.0 - no_change_probability
        )

    cleaned = {name: clean_number(objectives[name]) for name in OBJECTIVE_NAMES}
    return {
        "schema_version": SCHEMA_VERSION,
        "tool_version": TOOL_VERSION,
        "command": "score",
        "ok": True,
        "decision_id": model.graph["decision"]["id"],
        "candidate_id": model.candidate["id"],
        "input_identity": input_identity(model.graph, model.candidate),
        "feasible": not violations,
        "violations": violations,
        "objectives": cleaned,
        "modules": modules,
        "active_representatives": list(model.active),
        "resolved_partition": {
            vertex_id: model.assignments[model.representatives[vertex_id]]
            for vertex_id in sorted(model.vertices)
        },
        "representatives": {
            vertex_id: model.representatives[vertex_id]
            for vertex_id in sorted(model.vertices)
        },
        "cuts": sorted(cuts, key=lambda item: item["edge"]),
        "limitations": [
            "Evidence identity, closure, ownership, and strict-control shape are checked; receipt truth and physical execution are not proved.",
            "Objective values are meaningful only under the supplied units, scope, and coding scheme.",
        ],
    }


def clean_number(value: float) -> int | float:
    if abs(value - round(value)) < 1e-9:
        return round(value)
    return round(value, 9)


def vector(result: dict[str, Any]) -> tuple[float, ...]:
    return tuple(float(result["objectives"][name]) for name in OBJECTIVE_NAMES)


def dominates(left: dict[str, Any], right: dict[str, Any]) -> bool:
    a = vector(left)
    b = vector(right)
    return all(x <= y for x, y in zip(a, b)) and any(x < y for x, y in zip(a, b))


def enumerate_moves(model: Model, limit: int) -> dict[str, Any]:
    baseline = score_model(model)
    modules = sorted(set(model.assignments.values()))
    candidates: list[dict[str, Any]] = []
    considered = 0
    for vertex_id in model.active:
        old_module = model.assignments[vertex_id]
        for module in modules:
            if (
                module == old_module
            ):  # ubs:ignore — module labels are public architecture data, not secrets.
                continue
            considered += 1
            candidate = copy.deepcopy(model.candidate)
            candidate["assignments"][vertex_id] = module
            moved_model = build_model(model.graph, candidate)
            result = score_model(moved_model)
            if not result["feasible"] or dominates(baseline, result):
                continue
            candidates.append(
                {
                    "move": {"vertex": vertex_id, "from": old_module, "to": module},
                    "candidate_sha256": result["input_identity"]["candidate_sha256"],
                    "relation_to_baseline": "dominates"
                    if dominates(result, baseline)
                    else "tradeoff",
                    "objectives": result["objectives"],
                    "cuts": result["cuts"],
                }
            )

    frontier: list[dict[str, Any]] = []
    for index, candidate in enumerate(candidates):
        wrapped = {"objectives": candidate["objectives"]}
        if any(
            index != other_index
            and dominates({"objectives": other["objectives"]}, wrapped)
            for other_index, other in enumerate(candidates)
        ):
            continue
        frontier.append(candidate)
    frontier.sort(
        key=lambda item: (
            tuple(float(item["objectives"][name]) for name in OBJECTIVE_NAMES),
            item["move"]["vertex"],
            item["move"]["to"],
        )
    )
    truncated = len(frontier) > limit
    return {
        "schema_version": SCHEMA_VERSION,
        "tool_version": TOOL_VERSION,
        "command": "moves",
        "ok": True,
        "decision_id": model.graph["decision"]["id"],
        "candidate_id": model.candidate["id"],
        "input_identity": baseline["input_identity"],
        "baseline_feasible": baseline["feasible"],
        "baseline_objectives": baseline["objectives"],
        "generator": {
            "kind": "single_representative_existing_module",
            "candidates_considered": considered,
            "exhausted": True,
        },
        "frontier": frontier[:limit],
        "frontier_total": len(frontier),
        "truncated": truncated,
        "limitations": [
            "Only one active representative is moved at a time.",
            "No replacement, adapter, new-module, or semantic rewiring move is synthesized.",
            "Frontier membership is exhaustive only for the declared single-move generator.",
        ],
    }


def evaluate_fixed_point(
    model: Model,
    prior_state_sha256: str | None,
) -> dict[str, Any]:
    moves = enumerate_moves(model, sys.maxsize)
    state_projection = {
        "schema_version": SCHEMA_VERSION,
        "tool_version": TOOL_VERSION,
        "decision_id": moves["decision_id"],
        "candidate_id": moves["candidate_id"],
        "input_identity": moves["input_identity"],
        "baseline_feasible": moves["baseline_feasible"],
        "baseline_objectives": moves["baseline_objectives"],
        "generator": moves["generator"],
        "frontier": moves["frontier"],
        "frontier_total": moves["frontier_total"],
        "graph_nonclaims": model.graph["nonclaims"],
        "candidate_nonclaims": model.candidate["nonclaims"],
    }
    state_sha256 = canonical_sha256(state_projection)
    dominating = [
        candidate
        for candidate in moves["frontier"]
        if candidate["relation_to_baseline"] == "dominates"
    ]
    if not moves["baseline_feasible"]:
        status = "blocked_infeasible"
    elif dominating:
        status = "open_dominating_candidate"
    elif prior_state_sha256 is None:
        status = "open"
    elif prior_state_sha256 != state_sha256:
        status = "state_changed"
    else:
        status = "bounded_single_move_fixed_point"
    return {
        "schema_version": SCHEMA_VERSION,
        "tool_version": TOOL_VERSION,
        "command": "converge",
        "ok": True,
        "status": status,
        "fixed_point": status == "bounded_single_move_fixed_point",
        "state_sha256": state_sha256,
        "prior_state_sha256": prior_state_sha256,
        "baseline_feasible": moves["baseline_feasible"],
        "baseline_objectives": moves["baseline_objectives"],
        "generator": moves["generator"],
        "dominating_candidates": dominating,
        "tradeoff_frontier_total": sum(
            candidate["relation_to_baseline"] == "tradeoff"
            for candidate in moves["frontier"]
        ),
        "input_identity": moves["input_identity"],
        "limitations": [
            "The fixed point is exact only for one active representative moved among already-declared modules.",
            "It does not exhaust replacements, adapters, new modules, rewiring, source-model discovery, or multi-move paths.",
            "A second unchanged evaluation proves evaluator-state stability, not that an external discovery sweep occurred.",
        ],
    }


def self_test_documents() -> tuple[dict[str, Any], dict[str, Any]]:
    graph = {
        "schema_version": 2,
        "decision": {
            "id": "self-test-seam",
            "consumer": "self-test",
            "retirement_condition": "score one lawful fold and its local frontier",
            "source_identity": "self-test-source",
            "protected_observations": ["trace"],
            "selection_rule": "Pareto order after hard feasibility",
            "prior_decision_receipts": [],
        },
        "module_overhead_bits": 5,
        "semantic_steps": 10,
        "vertices": [
            {
                "id": "schedule",
                "kind": "phase",
                "current_module": "core",
                "pinned_module": "core",
                "description_bits": 10,
            },
            {
                "id": "terrain",
                "kind": "state",
                "current_module": "core",
                "authority_key": "terrain",
                "description_bits": 20,
                "memory_bytes": 64,
            },
            {
                "id": "terrain_copy",
                "kind": "state",
                "current_module": "spell",
                "authority_key": "terrain",
                "description_bits": 20,
                "memory_bytes": 64,
            },
            {
                "id": "spell",
                "kind": "transition",
                "current_module": "spell",
                "description_bits": 15,
            },
        ],
        "hyperedges": [
            {
                "id": "ordered_step",
                "kind": "read_write",
                "members": ["schedule", "terrain", "spell"],
                "mutable": True,
                "ordered": True,
                "weight": 2,
                "frequency": 3,
                "volume": 4,
                "protocol_bits": 7,
                "boundary_information_bits": 50,
            }
        ],
        "constraints": [
            {"id": "backbone", "kind": "co_locate", "vertices": ["schedule", "terrain"]}
        ],
        "semantic_constraints": [
            {
                "id": "trace",
                "required": True,
                "observation": "trace",
                "required_closure": "complete_for_declared_corpus",
                "negative_control_id": "trace-mutant",
            }
        ],
        "nonclaims": ["self-test evidence does not prove source-program equivalence"],
    }
    candidate = {
        "schema_version": 2,
        "id": "self-test-fold",
        "subject_identity": "self-test-candidate",
        "assignments": {"schedule": "core", "terrain": "core", "spell": "spell"},
        "replacements": {"terrain_copy": "terrain"},
        "evidence": {
            "trace": {
                "semantic_constraint_id": "trace",
                "subject_identity": "self-test-candidate",
                "claim_id": "self-test-trace-claim",
                "closure": "complete_for_declared_corpus",
                "status": "accepted",
                "plan_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "receipt_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "evaluator_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "skill_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "commands": ["self-test"],
                "artifacts": [],
                "negative_control": {
                    "id": "trace-mutant",
                    "status": "passed",
                    "receipt_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                },
            }
        },
        "adapters": [],
        "nonclaims": ["self-test fold is bounded to the declared graph"],
    }
    return graph, candidate


def self_test() -> dict[str, Any]:
    def check(condition: bool, message: str) -> None:
        if not condition:
            raise RuntimeError(f"self-test failed: {message}")

    graph, candidate = self_test_documents()
    result = score_model(build_model(graph, candidate))
    check(result["feasible"], "valid representative fold must be feasible")
    check(
        result["objectives"]["duplicate_authority"] == 0,
        "replacement must remove duplicate authority",
    )
    check(result["objectives"]["mutable_cut"] == 24, "mutable-cut score changed")
    check(result["objectives"]["ordered_cut"] == 6, "ordered-cut score changed")
    check(
        result["objectives"]["boundary_information_bits_per_step"] == 5,
        "boundary information rate changed",
    )
    check(result["objectives"]["mdl_bits"] == 62, "MDL score changed")
    check(
        result["input_identity"]["graph_sha256"] == canonical_sha256(graph),
        "graph identity must bind canonical input",
    )
    check(
        result["input_identity"]["candidate_sha256"] == canonical_sha256(candidate),
        "candidate identity must bind canonical input",
    )
    bad = copy.deepcopy(candidate)
    bad["replacements"] = {}
    bad["assignments"]["terrain_copy"] = "spell"
    bad_result = score_model(build_model(graph, bad))
    check(not bad_result["feasible"], "duplicate authority must be infeasible")
    check(
        any(item["code"] == "duplicate_authority" for item in bad_result["violations"]),
        "duplicate-authority violation must be reported",
    )
    moves = enumerate_moves(build_model(graph, candidate), 10)
    check(moves["frontier_total"] == 1, "unexpected Pareto frontier size")
    check(
        moves["frontier"][0]["move"]
        == {"vertex": "spell", "from": "spell", "to": "core"},
        "unexpected Pareto move",
    )
    check(
        moves["frontier"][0]["relation_to_baseline"] == "tradeoff",
        "move relation must preserve its migration tradeoff",
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "tool_version": TOOL_VERSION,
        "command": "self-test",
        "ok": True,
        "checks": 13,
        "identity": evaluator_identity(),
    }


def make_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--version", action="version", version=TOOL_VERSION)
    subparsers = parser.add_subparsers(dest="command", required=True)
    for command in ("validate", "score"):
        child = subparsers.add_parser(command)
        child.add_argument("--graph", required=True)
        child.add_argument("--candidate", required=True)
    moves = subparsers.add_parser("moves")
    moves.add_argument("--graph", required=True)
    moves.add_argument("--candidate", required=True)
    moves.add_argument("--limit", type=int, default=50)
    converge = subparsers.add_parser("converge")
    converge.add_argument("--graph", required=True)
    converge.add_argument("--candidate", required=True)
    converge.add_argument("--prior-state-sha256")
    subparsers.add_parser("identity")
    subparsers.add_parser("self-test")
    return parser


def main(argv: Iterable[str] | None = None) -> int:
    args = make_parser().parse_args(argv)
    try:
        if args.command == "self-test":
            output = self_test()
        elif args.command == "identity":
            output = {
                "schema_version": SCHEMA_VERSION,
                "tool_version": TOOL_VERSION,
                "command": "identity",
                "ok": True,
                **evaluator_identity(),
            }
        else:
            graph = load_json(args.graph)
            candidate = load_json(args.candidate)
            model = build_model(graph, candidate)
            if args.command == "moves":
                if args.limit <= 0:
                    raise InputError(
                        "invalid_limit", "--limit must be positive", "--limit"
                    )
                output = enumerate_moves(model, args.limit)
            elif args.command == "converge":
                prior = args.prior_state_sha256
                if prior is not None:
                    sha256_string(prior, "--prior-state-sha256")
                output = evaluate_fixed_point(model, prior)
            else:
                output = score_model(model)
                output["command"] = args.command
        print(json.dumps(output, sort_keys=True, separators=(",", ":")))
        if args.command == "validate" and not output["feasible"]:
            return 1
        return 0
    except InputError as error:
        return emit_error(error)


if __name__ == "__main__":
    raise SystemExit(main())
