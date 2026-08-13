# Design synthesis: frontier review to executable architecture

Date: 2026-08-12. Repository baseline reviewed: `d37fba6`.

This document records why the semantic architecture compiler has its present
shape. It synthesizes independent reviews by Fable and Oracle, three specialist
frontier passes (formal mathematics, natural systems/epistemology, and systems
architecture), the existing repository, and current primary literature. These
reviews are design advice, not evidence about a particular target system.

## Convergence

All reviews converged on five decisions:

1. Keep Kerf as the small exact finite checker. Put modeling and evidence in a
   separate compiler/harness surface.
2. Use one canonical evidence-bearing model for both greenfield intent and
   brownfield recovery, while keeping intent, implementation, and candidates as
   distinct subjects.
3. Make closure explicit. Without a statement such as exhaustive finite, sound
   over-approximation, corpus-complete, observed, sampled, or unknown, absence
   cannot become falsehood or impossibility.
4. Make projection total over its declared scope. Every coordinate is mapped or
   omitted, every map pointer resolves, and exactness stays on the emitted
   finite target.
5. Drive work from contradictions and missing proof/evidence obligations, not
   from file size or graph aesthetics.

Oracle's most important contribution was the closure lattice and the composite
claim boundary:

```text
model M + projection P -> finite K
Kerf checks Q of K
binding/evidence V with closure Γ limits correspondence to source revision r
```

Fable's adversarial review identified the missing reality bridge. A content
hash says which bytes were used; it does not say what a model symbol means in a
real process. That forced three implemented changes:

- mandatory subject-bound evidence bindings;
- separate formal-strength and reality-contact coordinates;
- `kerf-observe`, with reset/session protocol, repeated raw observations,
  negative controls, and checked trace certificates.

Fable also corrected a work-graph assumption: migration edges describe whole
system nodes, so disjoint-looking carrier changes are not automatically safe in
parallel. Parallel migration requires an explicitly validated commuting
diamond. Further, rechecking a pre-change model certificate is circular
acceptance; completed implementation work needs observation at the landing
revision or an explicit `model-only` classification. These constraints remain
roadmap requirements for external tracker application and migration synthesis.

## What transferred from the mathematical frontier

| Field | Executable transfer | Boundary |
|---|---|---|
| Coalgebra/automata | observations, transitions, morphisms, quotients, shortest distinguishing words | exact only for supplied finite systems |
| Typed open hypergraphs/cospans | joint typed tails/heads and explicit ports | incidence alone proves no behavior |
| Sheaves/local-to-global methods | future overlap-contract consistency checks | local agreement does not invent missing truth |
| Petri nets/event structures | future true-concurrency projections | partial-order reduction needs checked independence |
| Abstract interpretation/CEGAR | closure kind `sound_overapproximation`; counterexample-guided refinement | CEGAR name requires a real abstraction/concretization relation |
| CEGIS/SyGuS | bounded greenfield candidate synthesis | minimum only inside grammar, bounds, and tie-breaker |
| Information bottleneck/MDL | possible declared ranking coordinates | requires fixed X/Y distribution or coder; no universal minimum |
| Causal inference/control | interventions, invariants, feedback, discriminating experiments | provenance, similarity, and traces are not causation |

Typed open-system work motivates the structural layer, including recent cospan
and polynomial-functor formulations. Equality saturation is relevant to
candidate generation and local equivalence checking, not a global source
semantic oracle. See [sources.md](sources.md).

## What transferred from natural systems

The natural-systems review separated rigorous transfers from hypothesis
generators:

- null-space conservation laws can be exact for explicitly declared
  transitions;
- intervention-preserving abstraction maps can state real causal questions;
- information bottlenecks are meaningful only relative to declared variables;
- predictive equivalence needs stationarity/ergodicity and adequate sampling;
- feedback and invariance are useful when the state/observation model is
  explicit.

Bow-ties, homeostasis, retroactivity, biological modularity, reaction networks,
renormalization, linguistic bottlenecks, multiscale conversation, and neural
compositionality can propose entities, scales, or experiments. They do not
establish software seams. The compiler's heuristic/metaphor guard and the
anti-ornamentality test make this distinction enforceable.

## Architecture chosen

```text
                         ┌──────────────────────┐
greenfield requirements ─┤                      │
                         │ semantic model v1    ├─ findings/contradictions
brownfield bindings +   ─┤ intent / impl / cand├─ deterministic work graph
source/runtime evidence  │                      ├─ exact Kerf projections
                         └──────────┬───────────┘
                                    │
                          kerf-observe certificates
                                    │
                              finite Kerf kernel
                                    │
                        seams -> migration -> refactor
```

The JSON model is the interchange contract; Rust is the strict validator and
compiler. There is no separate app, database, DSL, graph service, or confidence
engine. A database becomes justified only by a measured query/persistence
consumer. An LLM is a proposer and semantic normalizer: its output enters as
inference or heuristic until a deterministic checker or real observation
changes the class.

The compiler is packaged as the sibling `odcpw/semantic-architecture` crate
with a one-way path dependency on `odcpw/behavioral-factorization`. This keeps
Kerf's proof-kernel surface small and lets both JSM skills validate within the
package limit. Kerf exposes only the serializable finite subject types and
canonical trace-subject hash needed by the observation layer.

## Current executable increment

- `kerf-model analyze`: strict typed validation plus semantic/evidence findings
  and per-claim grounding pairs.
- `kerf-model derive-work`: pure deterministic obligation graph, never external
  tracker mutation.
- `kerf-model project`: exact map-or-omit lowering into every existing Kerf
  input schema with a source-scope manifest.
- `kerf-observe`: environment-cleared repeated process observations, executable
  and output hashes, negative controls, trace certificate emission, and local
  verification.

The positive semantic example and nearby adversarial tests cover projection
holes, invalid JSON pointers, duplicate JSON members, unauthorized writers,
missing requirement semantics, finite-to-source laundering, runtime binding
omissions, grounding, real conformance, real contradiction, nondeterminism, and
negative-control failure.

## Accretive roadmap

The canonical Bead epic is `bf-semantic-architecture-compiler-iw8`; do not open
a competing architecture epic. Its remaining high-value path is:

1. bind `kerf-observe` output directly into brownfield semantic evidence;
2. add source adapters that emit proposed implementation subjects with explicit
   coverage and unknowns, beginning with this repository;
3. add greenfield bounded candidate generation within a fixed grammar;
4. implement local-to-global overlap-contract checks;
5. bridge accepted projections/claims into seam, de-monolithization, and
   simplification packets;
6. synthesize proof-carrying migration work, sequential by default and parallel
   only for validated commuting diamonds;
7. add an external Beads application command only after the pure graph has an
   operator-reviewed consumer;
8. activate L*/TTT/W-method learning only when model authoring is the measured
   bottleneck and a reliable reset-capable teacher exists.

The retirement condition is equally important: if the semantic ledger does not
prevent a real contradiction, lost claim, unsafe transformation, or repeated
modeling cost for a named consumer, retain the observation harness and exact
Kerf projections and delete—not elaborate—the unused ledger layer.
