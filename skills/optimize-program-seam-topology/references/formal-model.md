# Formal model: constraint-preserving program folding

## 1. Four objects, not one graph

A useful analysis keeps four objects distinct:

1. **Semantic system**: what executions and observations mean.
2. **Evidence hypergraph**: what the analysis has measured about state,
   transitions, contracts, and development.
3. **Architecture candidate**: which representations survive, who owns them,
   and where boundaries lie.
4. **Migration graph**: which behavior-preserving edits connect current and
   candidate architectures.

Conflating them creates false proofs. A good partition of an incomplete graph
does not establish semantic preservation. A semantically equivalent target can
still be an expensive architecture. A good final architecture can still require
an unsafe migration.

## 2. Semantic system and protected observations

Model the source or current implementation as a labeled transition system

```text
S = (X, I, A, ->, Obs)
```

where `X` is state, `I` initial states, `A` transition labels, and `Obs` a set
of observation functions over states, steps, finite traces, or infinite traces.
The target is `T` with a representation relation `R` or abstraction map `alpha`.

A protected observation is not merely an output value. Depending on the domain
it may include:

- exact state or event trace;
- read/write or happens-before order;
- arithmetic overflow and width;
- allocation, identity, generation, or reclamation;
- randomness draw count and order;
- persistence/reset/lifetime behavior;
- failure class and mutation-before-failure behavior;
- ABI bytes and cursor/generation binding;
- resource or timing bounds; or
- security-relevant noninterference.

Choose the weakest relation that is truthful:

- **trace equality** for exact deterministic replay;
- **stuttering or weak equivalence** when named internal steps are deliberately
  unobservable;
- **forward simulation/refinement** when every target behavior implements an
  allowed source behavior;
- **bisimulation** when both systems must match each other's observations;
- **bounded conformance** over an explicit workload set when complete proof is
  not available.

The protected relation defines the feasible set. It is not a score.

## 3. Multilayer temporal attributed directed hypergraph

Represent measured structure as

```text
H = (V, E, K, tau_V, tau_E, w, q, U)
```

where:

- `V` is the set of vertices;
- `E` is a set of directed hyperedges;
- `K` is the set of evidence layers;
- `tau_V` and `tau_E` are vertex and edge types;
- `w` contains measured magnitudes and units;
- `q` records temporal phase, ordering, or validity interval; and
- `U` records uncertainty and provenance.

### 3.1 Vertex types

Prefer semantic atoms over files:

| Type | Meaning |
|---|---|
| `state` | canonical mutable fact or candidate representation |
| `immutable_fact` | validated input that is never mutated in scope |
| `transition` | behavior with a measurable read/write/effect footprint |
| `phase` | scheduler or lifecycle phase |
| `identity` | allocator, slot, generation, handle, or lifetime carrier |
| `derived_view` | cache or projection with an explicit source and validity rule |
| `observation` | hash, trace, output, contract, or acceptance boundary |
| `contract` | ABI, schema, protocol, public type, or persisted layout |
| `file` | physical compilation/edit unit; never assumed semantic by itself |
| `owner` | team, writer lane, service, process, or deployment boundary |

A single source declaration may induce several vertices when it has several
roles. Conversely, multiple declarations may represent one semantic fact. Keep
that distinction until equivalence is proved.

### 3.2 Directed hyperedge form

For a transition `e`, a useful core record is

```text
e = (site_e, key_e, guard_e, R_e, W_e,
     alloc_e, reclaim_e, rng_e, emit_e,
     topology_e, facts_e, continuation_e, validity_e)
```

This records scheduler phase/site, conservative dispatch key, guard, reads,
writes, allocation/reclamation, ordered randomness and emissions, observed or
mutated identity/topology, immutable facts, scheduler continuation
(`next/rescan/retry/fault`), and cache/fact validity interval. Directed tails and
heads are projections of this richer record.

Hyperedges matter because a transition usually couples a set of inputs and
outputs as one semantic relation. Replacing it with all pairwise edges can
invent relationships, erase direction, and lose the fact that the whole set is
jointly required.

### 3.3 Evidence layers

Keep these layers separately queryable:

1. static control and data dependence;
2. dynamic read/write and call traces;
3. source schedule and temporal order;
4. identity, allocation, and lifetime;
5. reset, persistence, and invalidation;
6. ABI, schema, and external observations;
7. runtime frequency, volume, latency, and locality;
8. build invalidation and compile cost;
9. co-change and edit collision; and
10. provenance, tests, and evidence confidence.

Cross-layer edges connect the same semantic unit across layers. Multislice
network methods are useful for exploration, but layer coupling must not be
chosen merely to force a preferred result.

## 4. Candidate architecture

Represent a candidate as

```text
C = (rho, pi, ell, A, Z)
```

where:

- `rho: V -> V` is an idempotent representative map for removed duplicates;
- `pi: rho(V) -> M` assigns surviving representatives to modules/owners;
- `ell` assigns lifetime class where that is a design variable;
- `A` is the set of temporary adapters; and
- `Z` is revision-bound evidence attached to protected constraints.

Require `rho(rho(v)) = rho(v)`. A replacement is not valid merely because two
vertices share a name or type. It needs an observation/refinement argument and
a rewiring plan for all incident hyperedges.

The quotient graph `H/rho` is built only after representative choices pass hard
constraints. The partition is then applied to the quotient.

## 5. Feasibility

Keep three classes separate:

1. semantic constraints protect observations of the source/current system;
2. contract constraints protect ABI, schema, package, error, and persisted
   compatibility; and
3. project-policy constraints encode deliberate non-universal rules such as
   one canonical mutable carrier or one runtime being non-authoritative.

Let `F` be the candidates satisfying all three classes. Examples:

```text
one_owner(fact)             = true
preserve_trace(source, C)   = true
preserve_rng_order(C)       = true
preserve_identity(C)        = true
preserve_reset(C)           = true
preserve_contract(C)        = true
fail_closed(C)              = true
adapter_has_removal(A_i)    = true
```

`one_owner` is a powerful default for a local deterministic engine. It is not a
theorem about all software: replicated state machines, transactional replicas,
event-sourced logs, and derived caches may have several physical
representations when their consistency/derivation protocol is itself part of
the semantics. State that exception in the candidate materialization model.

Do not simulate infinity using a large coefficient. Reject `C` when `C` is not
in `F`. This avoids a sufficiently large improvement in compile time or module
count ever “buying” a semantic violation.

Evidence strength is part of feasibility policy:

```text
proved > exhaustively checked finite model > differential conformance
       > discriminating tests > ordinary regression > assumption
```

The order is contextual, not a universal numeric scale. Report it rather than
pretending a test sample is a proof.

## 6. Objective vector

For feasible `C`, minimize a vector rather than a premature scalar:

```text
f(C) = (
  duplicate_authority,
  sync_work,
  mutable_cut,
  ordered_cut,
  boundary_information_rate,
  special_case_description,
  contract_churn,
  edit_contention,
  runtime_cost,
  memory_cost,
  expected_rebuild_cost,
  migration_cost,
  proof_deficit
)
```

Every component needs a unit and an extraction route. Examples:

- `sync_work`: bytes or state cells copied per representative workload;
- `mutable_cut`: sum of observed write-crossing frequency times state volume;
- `ordered_cut`: count and frequency of cross-boundary ordering protocols;
- `boundary_information_rate`: fixed-codec bits in canonical boundary traces
  per semantic step, stratified by workload;
- `edit_contention`: overlapping writer-minutes or collision incidents;
- `runtime_cost`: measured latency/cycles/allocations caused by the boundary;
- `expected_rebuild_cost`: observed invalidation probability times compile time;
- `migration_cost`: touched semantic units, temporary adapters, and proof reruns;
- `proof_deficit`: named constraints without adequate evidence, not raw test
  count.

Pareto dominance is:

```text
C1 dominates C2 iff all f_i(C1) <= f_i(C2)
                    and some f_j(C1) < f_j(C2).
```

A frontier exposes tradeoffs. Policy can then use priority levels or explicit
bounds without hiding them in a coefficient soup.

## 7. Minimum description length

Use the two-part MDL principle:

```text
L(total) = L(architecture model) + L(observed behavior/data | model)
```

For program seams, a practical approximation can include:

```text
L(model) = shared implementation
         + module/interface declarations
         + dispatch/ownership description
         + exception and adapter description

L(data | model) = per-product or per-level descriptors
                + residual special cases
                + unexplained edge/type exceptions
```

Bytes, tokens, AST nodes, serialized schemas, or a stable compressor may be
used, but compare candidates under the same coding scheme. Raw lines of code
are not MDL. The code for the partition, interface vocabulary, and exceptions
must be included; otherwise overfragmentation looks artificially cheap.

MDL is especially valuable for choosing granularity because it charges both an
overly simple model with many exceptions and an overly complex model with too
many modules.

## 8. Information bottleneck and rate distortion

Let `X` be detailed implementation facts, `Z` a proposed module/interface
representation, and `Y` protected observations or future change tasks. The
information bottleneck asks for a compact `Z` that preserves information about
`Y`:

```text
min I(X; Z) - beta I(Z; Y)
```

This is a useful conceptual test for an interface: discard irrelevant internal
detail while retaining what consumers and proofs require.

Rate-distortion adds an important distinction:

- protected semantic observations have zero allowed distortion;
- approximate diagnostics or performance models may have explicit nonzero
  tolerances.

Never use an average distortion metric to hide a rare catastrophic semantic
error.

## 9. Local/global consistency

Two advanced lenses can help with difficult systems:

- **Wiring diagrams and operads** describe hierarchical composition of typed
  input/output processes. Use them when the question is whether local contracts
  compose, not as a substitute for source semantics.
- **Cellular sheaves** encode local data and restriction maps, making
  local-to-global inconsistency measurable. A nonzero inconsistency can flag
  incompatible ABI/state views around a cycle. This is research-grade tooling,
  not the default path for an ordinary refactor.

## 10. Source-target alignment

When source and target graphs have different vertex identities, ordinary graph
matching may fail. Relational alignment methods such as Gromov-Wasserstein can
propose correspondences based on within-graph distances or roles. Attributes
can be combined with relational structure using a fused objective.

Use such alignments to generate hypotheses. They are nonconvex, sensitive to
the chosen metric, and do not prove semantic correspondence. Confirm every
important match through state/transition evidence.

## 11. Uncertainty

Every measured edge should carry:

- origin: static, dynamic, history, manual, generated, or inferred;
- revision and workload;
- unit and sampling window;
- coverage or confidence bounds where available; and
- whether absence is meaningful.

Do not turn uncertainty into arbitrary fractional edge weights. Preserve the
provenance and run sensitivity analyses: recompute candidates with disputed
layers omitted, bounded, or perturbed. A seam that vanishes under a small
reasonable change is a hypothesis, not a decision.
