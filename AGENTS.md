# Semantic Architecture Skills repository instructions

This file adds only repository-specific truth to
`/home/oliver/.codex/AGENTS.md`.

## Purpose and authority

- Canonical aggregate checkout:
  `/home/oliver/Projects/odcpw/semantic-architecture-skills`.
- Git identity is `odcpw <odcpw@users.noreply.github.com>`; branch is
  `main`; the private canonical remote is
  `git@github-odcpw:odcpw/semantic-architecture-skills.git`.
- This repository is the unified development and GitHub publication project.
  It does not automatically change installed Codex skill discovery paths.
- Do not make the repository public or publish a release without Oliver's
  explicit instruction.

## Architecture

- `skills/semantic-architecture` owns the global semantic IR, executed
  evidence, decision contracts, and fixed-point stopping predicate.
- `skills/optimize-program-seam-topology` owns typed seam graphs, feasible
  structural partition scoring, exact bounded local Pareto stationarity, and
  migration-candidate generation. It does not own campaign fixed points.
- `skills/behavioral-factorization` owns exact finite behavioral checks.
- Dependencies must remain conceptually acyclic: semantic architecture may
  compile finite claims into behavioral factorization; seam topology consumes
  semantic/evidence packets and finite receipts; neither finite checker nor
  seam scorer may promote its bounded result into source-program equivalence.

## Verification

```bash
jsm validate skills/semantic-architecture --json
jsm validate skills/optimize-program-seam-topology --json
jsm validate skills/behavioral-factorization --json

cargo test --manifest-path skills/behavioral-factorization/Cargo.toml --all-targets --locked
cargo test --manifest-path skills/semantic-architecture/Cargo.toml --all-targets --locked
python3 skills/optimize-program-seam-topology/scripts/fold_graph.py self-test
```

Run strict formatting and linting for both Rust crates and focused tests for
every changed deterministic evaluator. Preserve project-specific case evidence
in the target repository or a named research reference, never in generated
cache directories.
