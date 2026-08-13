# Semantic Architecture Project Instructions

This project inherits `/home/oliver/.codex/AGENTS.md`; this file contains only
project-specific truth.

## Purpose and ownership

- Canonical aggregate development checkout:
  `/home/oliver/Projects/odcpw/semantic-architecture-skills/skills/semantic-architecture`.
- Git identity is `odcpw <odcpw@users.noreply.github.com>`.
- The original private constituent repository remains at
  `git@github-odcpw:odcpw/semantic-architecture.git` for history. New
  cross-skill development lands through the aggregate repository; there is no
  public release authority or release license.
- The project constructs greenfield intent models and recovers bounded
  brownfield models into one evidence-bearing semantic IR.
- The sibling `../behavioral-factorization` supplies the exact finite Kerf
  kernel. Keep the dependency one-way: semantic architecture may lower into
  Kerf; Kerf must not depend on this project.

## Architecture and invariants

- `src/semantic.rs` owns `kerf.semantic-model.v1`, typed analysis, claim and
  closure policy, deterministic work compilation, and exact projection
  manifests.
- `src/observe.rs` owns the explicit open/reset/probe/action/close reality
  harness. It executes declared programs without a sandbox, timeout, or output
  cap; tests must use harmless local fixtures.
- `src/manifest.rs` owns plan-bound executed evidence: source/object pins,
  executable and toolchain hashes, clean/dirty checkout identity, exact
  invocations, typed filesystem topology, validators, strict negative
  controls, authority ceilings, and the lossless compact parity-corpus
  compiler. It does not attest source-to-binary derivation or physical
  execution of a persisted unsigned receipt.
- `src/converge.rs` owns pure decision-contract evaluation. It re-audits every
  package against its exact plan, derives finite coverage only through
  mandatory decision paths, binds state to the evaluator executable, and
  requires an unchanged canonical state on a second complete sweep. It is a
  stopping predicate, not a scheduler.
- `src/bin/kerf-model.rs` and `src/bin/kerf-observe.rs` are machine-facing JSON
  CLIs. `src/bin/kerf-evidence.rs` executes evidence plans and evaluates
  stopping contracts. Success data belongs on stdout and typed errors on
  stderr.
- Formal class and reality contact are independent coordinates. Never promote
  a finite Kerf result, passive source extraction, LLM interpretation, trace,
  or analogy into source-program equivalence.
- Every finite projection must map or explicitly omit every scoped entity and
  incident relation. Projection validity is not source correspondence.
- Work compilation is pure. External tracker mutation is an adapter concern.
- Preserve counterexamples and unknowns; never weaken a closure or acceptance
  gate to obtain a clean report.

## Verification

```bash
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
jsm validate . --json
```

Run `ubs` on changed Rust source before handoff. For CLI behavior, also execute
the checked examples in `examples/` and confirm data-only JSON output.
