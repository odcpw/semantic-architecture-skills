#!/usr/bin/python3
"""Build a content-bound metabolism plan from an accepted staged diff."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def bound_command(program: Path, arguments: list[str], external: list[Path] | None = None) -> dict:
    return {
        "program": {"path": str(program), "sha256": sha(program)},
        "external_files": [
            {"path": str(path), "sha256": sha(path)} for path in (external or [])
        ],
        "arguments": arguments,
        "cwd": ".",
        "environment": {},
        "stdin": "",
    }


def cargo_command(arguments: list[str]) -> dict:
    command = bound_command(Path("/usr/bin/cargo"), arguments)
    command["environment"] = {
        "CARGO_HOME": "/home/oliver/.cargo",
        "CARGO_TARGET_DIR": "target-kerf",
        "HOME": "/home/oliver",
        "PATH": "/usr/bin:/bin",
    }
    return command


def main() -> int:
    if len(sys.argv) != 9:
        return 2
    request_path = Path(sys.argv[1]).resolve()
    batch_path = Path(sys.argv[2]).resolve()
    candidate_id = sys.argv[3]
    baseline = Path(sys.argv[4]).resolve()
    staged = Path(sys.argv[5]).resolve()
    workspace = Path(sys.argv[6]).resolve()
    output = Path(sys.argv[7]).resolve()
    plan_id = sys.argv[8]
    request = json.loads(request_path.read_text(encoding="utf-8"))
    batch = json.loads(batch_path.read_text(encoding="utf-8"))
    if not any(item["id"] == candidate_id for item in batch["candidates"]):
        raise SystemExit("candidate is absent from batch")

    relative_files = [Path("Cargo.toml"), Path("Cargo.lock")]
    for directory in ("src", "tests", "testdata"):
        relative_files.extend(
            sorted(
                path.relative_to(baseline)
                for path in (baseline / directory).rglob("*")
                if path.is_file()
            )
        )
    changed = []
    for relative in relative_files:
        before = baseline / relative
        after = staged / relative
        if before.read_bytes() != after.read_bytes():
            changed.append((relative, before, after))
    if not changed:
        raise SystemExit("staged tree has no changed bound source")

    mutations = []
    for relative, before, after in changed:
        mutations.append(
            {
                "kind": "replace_text",
                "path": relative.as_posix(),
                "preimage_sha256": sha(before),
                "replacements": [
                    {
                        "old": before.read_text(encoding="utf-8"),
                        "new": after.read_text(encoding="utf-8"),
                        "expected_occurrences": 1,
                    }
                ],
            }
        )

    probe = Path(__file__).with_name("pilot_probe.py").resolve()
    python = Path("/usr/bin/python3")
    probe_command = lambda mode: bound_command(  # noqa: E731 - compact plan schema mapping
        python, [str(probe), mode], [probe]
    )
    model_command = bound_command(
        python, [str(probe), "rebuild", ".kerf-rebuilt.model.json"], [probe]
    )
    plan = {
        "schema_version": "kerf.metabolism-plan.v1",
        "plan_id": plan_id,
        "candidate_id": candidate_id,
        "source_revision": request["subject"]["revision"],
        "source_manifest": [
            {"path": relative.as_posix(), "sha256": sha(baseline / relative)}
            for relative in relative_files
        ],
        "isolation": {
            "workspace_root": str(workspace),
            "copy_paths": ["Cargo.toml", "Cargo.lock", "src", "tests", "testdata"],
            "excluded_names": [".git", ".ntm", "target", "target-kerf"],
            "max_files": 2000,
            "max_bytes": 30000000,
        },
        "mutations": mutations,
        "checks": [
            {
                "id": "rust-library-contracts",
                "kind": "success",
                "command": cargo_command(["test", "--quiet", "--lib", "--locked"]),
                "protected_observation_ids": ["library_contracts"],
                "hard_constraint_ids": ["rust_library_green"],
            },
            {
                "id": "range-capability-contract",
                "kind": "stdout_parity",
                "command": cargo_command(
                    ["run", "--quiet", "--locked", "--", "--json", "capabilities", "--for", "range"]
                ),
                "protected_observation_ids": ["capabilities_json"],
                "hard_constraint_ids": ["capability_contract_stable"],
            },
        ],
        "negative_controls": [
            {
                "id": "unknown-command-rejected",
                "command": cargo_command(
                    ["run", "--quiet", "--locked", "--", "--json", "definitely-not-a-command"]
                ),
                "expected_exit_code": 2,
            }
        ],
        "metrics": [
            {"metric_id": "rust_nonblank_lines", "command": probe_command("source-lines")},
            {
                "metric_id": "duplicate_window_excess",
                "command": probe_command("duplicate-excess"),
            },
        ],
        "forbidden_regression_metrics": ["rust_nonblank_lines", "duplicate_window_excess"],
        "model_rebuild": {
            "command": model_command,
            "model_path": ".kerf-rebuilt.model.json",
            "implementation_subject_id": "implementation",
        },
        "nonclaims": [
            "Linux library and capability evidence does not establish desktop Office compatibility",
            "the duplicate-window metric is a bounded lexical coordinate, not semantic equivalence",
        ],
    }
    output.write_text(json.dumps(plan, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
