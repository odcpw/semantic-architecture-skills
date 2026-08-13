# Behavioral Factorization repository instructions

This file adds only repository-specific rules to
`/home/oliver/.codex/AGENTS.md`.

## Purpose and authority

- The canonical aggregate development checkout is
  `/home/oliver/Projects/odcpw/semantic-architecture-skills/skills/behavioral-factorization`.
  Git identity is `odcpw <odcpw@users.noreply.github.com>` and the branch is
  `main`. The original private constituent repository remains at
  `git@github-odcpw:odcpw/behavioral-factorization.git` for history.
- This is a private research tool and Codex skill. There is no public release
  authority or release license; do not make it public or publish a release
  without Oliver's instruction.
- The repository is independently worded from public literature and contains
  no remc2 source, owner data, private traces, or MagicCarpet implementation
  details.

## Product boundary

- `SKILL.md` is the concise routing and operating manual. Detailed theory lives
  under `references/`; do not turn the skill entry point into a literature dump.
- `kerf` proves properties only of supplied finite deterministic models and
  represented-feasible supplied candidate sets. It never claims equivalence of an original
  Rust program, causal identification, or a globally optimal real architecture.
- Hard semantic, contract, and policy constraints are feasibility conditions.
  They are never finite penalties. Unknown evidence is never equivalent to
  positive proof.
- Cost coordinates retain their units and use Pareto order. Do not introduce
  hidden weights, floating-point tolerances, or an "architecture energy."
- Primary `kerf` outputs are deterministic, versioned, and bound to the input
  hash. LTS exploration is explicitly bounded; a resource limit returns
  `unknown`, never success. The separate self-host envelope checks Git HEAD and
  executable hash but does not prove source-to-binary derivation.

## Architecture

- `src/lts.rs` owns finite machines, context monitors, validation, products,
  equivalence witnesses, and observational quotienting.
- `src/factor.rs` checks supplied context-relative factor maps and emits their
  induced partial boundary representations or shortest conflict witnesses.
- `src/factor_search.rs` enumerates bounded restricted-growth partitions and
  admits them through the same factor laws; candidate exhaustion is unknown.
- `src/recompose.rs` synchronizes certified factor contexts, constructs only
  the proved-reachable synchronous product, and compares it with a reference.
- `src/morphism.rs` checks one supplied total finite state function, global
  carrier claims, and independently supplied section claims without inferring
  a relation or map.
- `src/cert.rs` locally checks hash-bound finite relations, inequivalence
  witnesses, and recorded traces; provenance remains orthogonal to validity.
- `src/candidates.rs` owns explicit architecture assignments, executable hard
  constraints, exact metric validation, Pareto dominance, and finite
  proof-carrying migration-graph validation.
- `src/transport.rs` owns arbitrary-precision exact rational arithmetic for
  declared static routes.
- `src/report.rs` owns typed domain errors and input hashing.
- `src/main.rs` owns argument routing, file I/O, and versioned result/error
  envelopes with execution context.
- `src/self_host.rs`, `src/bin/kerf-selfhost.rs`, and `self-host/` own the
  revision-checked raw-process corpus falsifier. It is not a source analyzer.
- The sibling `../semantic-architecture` is the global semantic compiler and
  reality-contact layer. It depends on this crate as its exact finite kernel;
  keep the dependency one-way and never broaden a Kerf result into a
  source-program claim.
- `examples/` are discriminating microcases; `references/` separates theorem,
  engineering inference, conjecture, and broken metaphor.

## Toolchain and verification

- Rust stable is the only runtime dependency. Keep the dependency set small and
  commit `Cargo.lock` for reproducibility.
- Focused checks:

  ```bash
  cargo fmt --all -- --check
  cargo test --all-targets --locked
  cargo clippy --all-targets --all-features --locked -- -D warnings
  jsm validate . --json
  ```

  A clean self-host evidence run additionally builds both binaries and invokes
  `kerf-selfhost` with the exact 40-hex `HEAD` and `self-host/corpus.json`.

- For each semantic algorithm, test a nearby positive case, a shortest negative
  witness, invalid input, unreachable state handling, deterministic ordering,
  and a configured resource limit.
- New claims require either an executable finite certificate or a primary-source
  citation plus an explicit evidence label. Natural-system analogies never count
  as software evidence.
