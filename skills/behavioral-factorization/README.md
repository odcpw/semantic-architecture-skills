# Behavioral Factorization

Behavioral Factorization is a research method and Rust checker for finding
lawful system boundaries. Its root claim is deliberately narrow:

> Factor an open system through the smallest boundary that preserves its
> declared behavior, then optimize ownership, transport, placement, and
> migration only inside that feasible set.

The `kerf` binary provides exact checks for supplied finite deterministic
models. LTS exploration has explicit state, pair, and witness-depth limits:

- validate a total transition system and finite context monitor;
- compute the reachable observational quotient;
- compare two machines and return a shortest distinguishing action word;
- locally verify a hash-bound finite relation, witness, or recorded trace;
- check whether a supplied context-relative map is a lawful behavioral factor;
- exhaustively find a minimum lawful factor map within explicit finite bounds;
- recompose named certified factors synchronously and compare the reachable
  behavior with a total reference;
- check a supplied total state function as an all-state or context-reachable
  finite machine morphism, with separately named carrier and section claims;
- validate explicit assignment, separation, pin, and authority constraints;
- validate proof-carrying one-carrier migration edges and well-founded cleanup;
- compute the exact Pareto frontier of the represented-feasible supplied set; and
- account checked rational load on declared static routes.

It does not infer semantics from Rust source or claim a globally optimal real
architecture. The surrounding skill explains how to build evidence, derive
applicable lower bounds, and keep the theorem/inference/metaphor boundary
honest. The sibling `semantic-architecture` project is the global model and
reality-contact layer that lowers explicit finite questions into this kernel.

The name is literal. The irreducible operation found by the research swarm was
not energy minimization or graph clustering, but factorization of an open
behavior through an interface that preserves declared observations. The binary
is called `kerf`: the material a lawful cut must consume. Its measured width is
contextual state, protocol, and routed cost—not a universal scalar.

## Quick start

```bash
cargo test --all-targets --locked
cargo run --quiet --bin kerf -- quotient examples/toggle.lts.json | jq
cargo run --quiet --bin kerf -- distinguish examples/rng-order.compare.json | jq
cargo run --quiet --bin kerf -- verify-certificate examples/simple-equivalence.cert.json | jq
cargo run --quiet --bin kerf -- verify-certificate examples/rng-order.witness.cert.json | jq
cargo run --quiet --bin kerf -- verify-certificate examples/toggle.trace.cert.json | jq
cargo run --quiet --bin kerf -- factor examples/context-side-information.factor.json | jq
cargo run --quiet --bin kerf -- search-factors self-host/process.factor-search.json | jq
cargo run --quiet --bin kerf -- recompose examples/context-side-information.recompose.json | jq
cargo run --quiet --bin kerf -- recompose examples/two-factor.recompose.json | jq
cargo run --quiet --bin kerf -- pareto examples/architecture.candidates.json | jq
cargo run --quiet --bin kerf -- score-routes examples/memory.transport.json | jq
```

All analysis output is versioned JSON on stdout. Errors are typed JSON on
stderr. Exact observations and emissions are opaque strings, avoiding hidden
JSON numeric or canonicalization semantics.

## Executable self-host falsifier

`kerf-selfhost` runs the real `kerf` executable against the fixed corpus in
`self-host/`. It checks raw exit code, stdout bytes, and stderr bytes for eight
roles: successful validation, typed invalid input, resource-limit unknown,
distinguishing witness, lawful and nearby unlawful factors, and equivalent and
nearby inequivalent recompositions. It also fails closed unless the requested
Git HEAD matches and the repository is clean:

```bash
cargo build --bins --locked
target/debug/kerf-selfhost \
  --kerf "$PWD/target/debug/kerf" \
  --repo-root "$PWD" \
  --expect-head <40-lowercase-hex-Git-HEAD> \
  --corpus "$PWD/self-host/corpus.json"
```

The adapter binds an observed executable hash to one repository revision and
one corpus run. It is a regression falsifier and frontend seed, not a proof of
source-to-binary reproducibility, source-program equivalence, or corpus
completeness. `--allow-dirty` exists for development and reports the dirty
state; it is not revision-bound evidence.

## Why there is no simulator dependency

The v0.1 proof kernel is finite and discrete. FrankenSim was evaluated as a
possible foundation for continuum and physical-system experiments, but adopting
it here would add a large simulation surface without strengthening finite trace
equivalence, exact Pareto dominance, or static route arithmetic. A future
continuum benchmark may integrate it when a named model and falsification test
require it.

## Status

This is a private `odcpw` repository with no public release authority or
release license. The repository contains no MagicCarpet source, remc2
material, owner data, or private traces.
