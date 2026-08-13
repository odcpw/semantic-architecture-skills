# Semantic Architecture

Semantic Architecture is the model/compiler layer above Kerf. It gives new
systems and existing codebases the same destination: a strict, versioned model
whose claims name their subject, scope, evidence, closure, epistemic strength,
reality contact, and falsifier.

It is deliberately not a universal optimizer. “Smallest” is meaningful only
after the observations, contexts, candidate language, resource bounds, and
cost order are explicit. Within a finite projection, Kerf can establish exact
quotients, factors, morphisms, and bounded minima. Correspondence to real code
remains a separate evidence-bearing claim.

## The stack

```text
greenfield requirements ─┐
                         ├─> kerf.semantic-model.v1
brownfield evidence ────┘            │
                                      ├─> typed findings
                                      ├─> deterministic work graph
                                      ├─> observation certificates
                                      └─> explicit finite projection
                                                    │
                                                    v
                                                  Kerf
```

The model includes intent, implementation, and candidate subjects; requirements,
observations, state carriers, authorities, derived views, transitions, effects,
ports, contracts, implementation locators, invariants, and resources; typed
hyper-relations; hashed bindings; evidence; closure claims; scoped claims;
work; and Kerf projections.

Two axes prevent proof laundering:

- formal class: theorem instance, exact checked model, bounded check, evidence
  inference, heuristic, or metaphor;
- reality contact: none, declared artifact, observed, or intervened.

Closure is separate again: exhaustive finite, sound over-approximation,
complete for a declared corpus, observed only, sampled, or unknown. Therefore
“not observed” cannot silently become “impossible.”

## Run it

```bash
cargo run --quiet --bin kerf-model -- analyze examples/semantic-toggle.model.json
cargo run --quiet --bin kerf-model -- derive-work examples/semantic-toggle.model.json
cargo run --quiet --bin kerf-model -- project examples/semantic-toggle.model.json toggle-lts
cargo run --quiet --bin kerf-model -- check-coordinates coordinates.json
cargo run --quiet --bin kerf-observe -- examples/toggle.observe.json
cargo run --quiet --bin kerf-pipeline -- verify-square coordinates.json receipt.json
cargo run --quiet --bin kerf-evidence -- identity
cargo run --quiet --bin kerf-evidence -- expand-parity-corpus parity-spec.json
cargo run --quiet --bin kerf-evidence -- execute plan.json --require-accepted
cargo run --quiet --bin kerf-evidence -- evaluate contract.json package.json --require-fixed-point
```

All successful output is versioned JSON on stdout. Errors are typed JSON on
stderr. `kerf-observe` executes the plan's real commands. Review every plan;
the harness clears inherited environment variables and binds executable bytes,
but it does not sandbox commands, limit runtime, cap output, or constrain their
external effects.

## Coordinate correspondence checks

`kerf.coordinate-model.v1` describes facts by semantic kind, unit, chart,
effective/observation clock, authority, and identity. A correspondence claim over
different coordinates must name a directional conversion with a hash-bound
checker. Every claim must also name a verification square: two executable command
manifests, their observations, a comparator, a coverage ceiling, and the decisive
falsifier.

The pure checker emits a blocker when a claim silently crosses coordinates, presents
multiple writers as one authority without a transfer protocol, or asks for a
stronger closure than the weakest fact, conversion, or square can support. It
only checks the manifest. It does not execute commands, authenticate artifacts,
or prove that the declared facts completely describe the source system. It now
reports structural square readiness separately and refuses to promote a
positive correspondence by itself. `kerf-pipeline verify-square` re-hashes the
resolved artifacts and checks every named output pair byte-for-byte, retaining
the first mismatch offset; this is artifact binding, not execution or build
attestation.

## Executed evidence and honest stopping

`kerf-evidence` closes the gap between a declared command and an executed,
identity-bound observation. Its plan pins Git objects and checkout dirtiness,
executables, dependencies, toolchain output, exact argv/cwd/env/stdin, and typed
filesystem topology including empty directories, symlinks, and dangling links.
Validators name their authority, claims name their closure, and every positive
claim needs a separate strict negative control.

An executed claim's machine-supported subject is a typed set of bound
executables. Its free description is explanatory metadata, not the supported
proposition. A source-revision claim needs a separate source↔binary attestation,
which v1 intentionally cannot supply. An `independent_fixture` must be present
at a distinct, strict Git ancestor of the observed subject; a candidate-side
golden cannot become independent by changing its label.

An execution package proves no more than its declared corpus or observations.
It does not prove that source produced the binary, cannot accept
`source_correspondence`, and a persisted unsigned package is internally
auditable rather than cryptographic proof that execution physically occurred.
For hostile provenance, supply an external attestation or execute and consume
the package within the same trusted process boundary.

The convergence evaluator is plan-aware, evidence-aware, and bound to the
executable and embedded skill-manual SHA-256 identities emitted by
`kerf-evidence identity`. It derives finite
coverage only from invocations reachable through mandatory claims and their
strict controls, and requires a second complete sweep with an unchanged
canonical state before returning a fixed point. Budget exhaustion is always an
unknown, never success. See [Executed evidence and convergence](references/executed-evidence.md).

For the common baseline/candidate corpus shape,
`kerf.parity-corpus-spec.v1` compiles each case into both invocations, an
immutable-fixture check, and a distinct-implementation parity check, plus one
strict semantic negative control. The compiler adds no authority: it emits the
same validated execution-plan schema and its complete result envelope can be
passed directly to `execute`.

## Read next

- [Architecture and workflows](references/semantic-architecture.md)
- [Observation harness](references/observation-harness.md)
- [Design synthesis and roadmap](references/design-synthesis.md)
- [Dispersion pilot accretion record](references/dispersion-pilot-accretion.md)
- [PowerBI resolver pilot accretion record](references/powerbi-resolver-pilot-accretion.md)
- [Operational schema guide](references/workflow-and-schema.md)
- [Convergent campaigns and honest stopping](references/convergent-campaigns.md)
- [Executed evidence and convergence](references/executed-evidence.md)
- [Primary-source ledger](references/sources.md)
