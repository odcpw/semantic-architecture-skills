#!/usr/bin/python3
"""Deterministic OOXML pilot probes and source-bound semantic rebuild."""

from __future__ import annotations

import hashlib
import json
import os
from collections import defaultdict
from pathlib import Path
import sys


def rust_files(root: Path) -> list[Path]:
    return sorted(
        path
        for source_root in (root / "src", root / "tests")
        for path in source_root.rglob("*.rs")
        if path.is_file()
    )


def normalized_lines(path: Path) -> list[str]:
    result = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if line and line not in {"{", "}", "(", ")", "[", "]"} and not line.startswith("//"):
            result.append(line)
    return result


def source_lines(root: Path) -> int:
    return sum(len(normalized_lines(path)) for path in rust_files(root))


def duplicate_excess(root: Path, width: int = 8) -> int:
    locations: dict[str, set[str]] = defaultdict(set)
    for path in rust_files(root):
        relative = path.relative_to(root).as_posix()
        lines = normalized_lines(path)
        for start in range(len(lines) - width + 1):
            block = "\n".join(lines[start : start + width])
            if len(block) >= 64 and sum(ch.isalnum() and ch.isascii() for ch in block) >= 32:
                locations[hashlib.sha256(block.encode()).hexdigest()].add(relative)
    return sum(len(files) - 1 for files in locations.values() if len(files) >= 2)


def rebuild(root: Path, output: Path) -> None:
    revision = os.environ["KERF_POST_REVISION"]
    files = rust_files(root)
    entities = []
    bindings = []
    for index, path in enumerate(files):
        relative = path.relative_to(root).as_posix()
        artifact_hash = hashlib.sha256(path.read_bytes()).hexdigest()
        entity_id = f"source-{index:04d}"
        entities.append(
            {
                "id": entity_id,
                "subject": "implementation",
                "label": relative,
                "kind": "implementation",
                "locator": relative,
            }
        )
        bindings.append(
            {
                "id": f"binding-{index:04d}",
                "subject": "implementation",
                "kind": "source_artifact",
                "artifact_sha256": artifact_hash,
                "locator": relative,
                "revision": revision,
            }
        )
    model = {
        "schema_version": "kerf.semantic-model.v1",
        "model_id": "ooxml-source-architecture",
        "subjects": [
            {
                "id": "implementation",
                "kind": "implementation",
                "revision": revision,
                "runtime_environment": "linux-rust-1.96",
            }
        ],
        "scopes": [
            {
                "id": "rust-source",
                "subject": "implementation",
                "contexts": ["src/**/*.rs"],
                "workloads": ["OOXML CLI and library"],
                "horizon": "current content revision",
            }
        ],
        "entities": entities,
        "bindings": bindings,
    }
    output.write_text(json.dumps(model, sort_keys=True, separators=(",", ":")), encoding="utf-8")


def main() -> int:
    if len(sys.argv) < 2:
        return 2
    root = Path.cwd()
    mode = sys.argv[1]
    if mode == "source-lines":
        print(source_lines(root))
        return 0
    if mode == "duplicate-excess":
        print(duplicate_excess(root))
        return 0
    if mode == "block" and len(sys.argv) == 5:
        path = root / sys.argv[2]
        start = int(sys.argv[3]) - 1
        width = int(sys.argv[4])
        print(json.dumps(normalized_lines(path)[start : start + width], indent=2))
        return 0
    if mode == "rank" and len(sys.argv) == 3:
        batch = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
        ranked = []
        for candidate in batch["candidates"]:
            witness = candidate["witnesses"][0]
            location = witness["locations"][0]
            relative, raw_start = location.rsplit(":", 1)
            start = int(raw_start) - 1
            width = witness["normalized_line_count"]
            block = normalized_lines(root / relative)[start : start + width]
            if block and (block[0].startswith("fn ") or block[0].startswith("pub fn ")):
                ranked.append(
                    {
                        "id": candidate["id"],
                        "locations": witness["locations"],
                        "first": block[0],
                        "last": block[-1],
                    }
                )
        print(json.dumps(ranked, indent=2))
        return 0
    if mode == "rebuild" and len(sys.argv) == 3:
        rebuild(root, root / sys.argv[2])
        return 0
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
