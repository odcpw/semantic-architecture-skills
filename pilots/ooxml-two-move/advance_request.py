#!/usr/bin/python3
"""Advance a brownfield request from a landed commit and prior rebuild receipt."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import sys


def git(repo: Path, *arguments: str) -> bytes:
    return subprocess.run(
        ["/usr/bin/git", "-C", str(repo), *arguments],
        check=True,
        stdout=subprocess.PIPE,
    ).stdout


def main() -> int:
    if len(sys.argv) != 5:
        return 2
    prior_path = Path(sys.argv[1])
    receipt_path = Path(sys.argv[2])
    repository = Path(sys.argv[3]).resolve()
    output_path = Path(sys.argv[4])
    request = json.loads(prior_path.read_text(encoding="utf-8"))
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))["result"]
    if not receipt["accepted"]:
        raise SystemExit("prior move was not accepted")
    revision = git(repository, "rev-parse", "HEAD").decode().strip()
    tree_hash = hashlib.sha256(git(repository, "archive", "HEAD")).hexdigest()
    model_hash = receipt["model_receipt"]["sha256"]
    search_basis = hashlib.sha256(
        f"{revision}:{model_hash}:{request['envelope']['envelope_id']}".encode()
    ).hexdigest()
    request["request_id"] = "ooxml-two-move-pilot-v2"
    request["search_basis_sha256"] = search_basis
    request["subject"] = {
        "mode": "brownfield",
        "repository_root": str(repository),
        "revision": revision,
        "source_tree_sha256": tree_hash,
    }
    output_path.write_text(json.dumps(request, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
