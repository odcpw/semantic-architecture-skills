# Feasible search, description length, and migration

## Three different computations

Do not collapse:

1. **semantic quotienting** — which distinctions are interchangeable under the
   declared observation relation;
2. **factorization, ownership, placement, and routing** — where lawful surviving
   state and work should live; and
3. **migration routing** — which verified revisions can reach a target without
   leaving the feasible set.

A good partition of an incomplete graph is not a semantic proof. A good target
can have an unsafe migration. A safe migration can end at an expensive design.

## Admissibility before cost

Partition candidates into:

```text
refuted
unresolved at required evidence level
accepted at evidence level k
```

Only accepted candidates enter cost comparison. Missing proof is usually an
admission status, not a small objective penalty.

Hard constraints can include:

- protected trace/refinement;
- arithmetic, order, RNG, identity, reset, and failure behavior;
- one declared authority protocol per semantic fact;
- ABI, schema, persistence, and security contracts;
- capacity, liveness, and failure-model conditions; and
- project policy such as a non-authoritative presentation layer.

Never approximate a hard constraint with a large coefficient.

## Dimensioned Pareto order

For feasible candidates retain coordinates such as:

```text
boundary bits / semantic step
cross-owner writes / operation
cache-line transfers / operation
messages / second
bytes / second
p50 and p99 seconds / operation
cycles / operation
peak bytes
joules / operation when measured
incremental build seconds / change
developer minutes / change
description bits under a frozen coder
migration carriers touched and person-hours
```

Use product Pareto order by default. A candidate dominates another only when it
is no worse in every comparable coordinate and strictly better in at least one.
Equal vectors leave both candidates nondominated unless their structures are
identical.

Use lexicographic or epsilon-constrained optimization when policy supplies a
priority hierarchy. Use a scalar only with explicit exchange rates, units, and
sensitivity analysis.

## Minimum description length

MDL is a model-selection principle under a declared coding scheme, not a synonym
for fewer source lines. A useful architecture code may include:

```text
L = code(shared behavior)
  + code(boundary and dispatch schemas)
  + code(validated data/configuration)
  + code(unavoidable exceptions)
  + code(temporary adapters)
```

Freeze the language, formatter, macro-expansion rule, and coder. A compressor or
syntax change must not masquerade as architecture improvement.

For a family of content variants, a valuable prospective prediction is that
the exception term declines as shared capabilities saturate. Test that on held-
out future variants.

## Candidate generation

After semantic contractions and mandatory groups:

1. seed the current architecture and obvious baselines;
2. generate typed multilevel candidates without flattening edge layers;
3. enumerate exact bounded instances where possible;
4. use CP-SAT/MILP or heuristics for larger instances;
5. record feasibility, incumbent, lower bound, and solver gap independently;
6. archive all nondominated candidates; and
7. repeat under workload and evidence perturbations.

Spectral methods, graph partitions, embeddings, LLM proposals, natural-network
adaptation, and optimal transport are proposal mechanisms. They do not establish
semantic equivalence.

## Robustness and scale

A preferred candidate should remain nondominated across a declared uncertainty
set of workloads, hardware, evidence weights, history windows, and reasonable
granularity choices. Report:

- always feasible;
- feasible only under some assumptions;
- always nondominated;
- frontier rank or hypervolume stability; and
- which perturbation changes the decision.

A seam found only by one extraction method is an operational fraction, not a
canonical module.

## Minimal interaction alphabet

Self-assembly research motivates a testable engineering hypothesis [C]: sweep
the number of semantically distinct state, effect, action, and port types. Seek
the smallest alphabet at which migration accessibility and final quality cease
trading sharply.

- Too few types produce over-generic interfaces and off-target compositions.
- Too many produce special cases, trait explosion, contract churn, and compile
  cost.

This is an experiment, not a theorem of software.

## Architecture nucleus

Enumerate near-optimal but semantically wrong candidates. Each invariant, type
distinction, or discriminating test rejects a subset. An inclusion-minimal
hitting set of evidence that rejects every wrong candidate is a useful
**architecture nucleus**: no member can be removed without admitting a known
wrong candidate [C].

Validate inclusion-minimality by removing each nucleus member and demonstrating
at least one wrong candidate that becomes admissible. This does not prove
minimum cardinality: a different hitting set may use fewer members. Claim the
smallest nucleus only after exhaustive search on a bounded instance or a checked
solver certificate that no smaller hitting set exists.

## Proof-carrying migration graph

`kerf validate-migration` checks an explicit finite graph, not a repository.
Each node is a complete modeled state over one fixed carrier universe. For each
carrier it contains one finite machine and one writer; the node may also carry
one cleanup obligation with a positive natural rank. Each directed edge changes
exactly one machine, embeds a locally checkable source-to-target equivalence
certificate under the carrier's graph-fixed context, and exactly states any
writer transfer. Canonical machine encodings establish endpoint identity;
asserted hashes and JSON declaration order do not.

Cleanup is node state because an edge-only annotation is ambiguous at joins.
The admitted transitions are:

```text
debt-free -> debt-free       neutral
debt-free -> (o, r)          introduce, r > 0
(o, r)    -> (o, r')         progress, 0 < r' < r
(o, r)    -> debt-free       discharge
```

Every positive node must have an outgoing edge. Consequently, any maximal
continuation that starts at rank `r` reaches debt-free state within at most `r`
edges: continuing positive edges form a strictly descending sequence of
positive natural numbers. Equivalently, no path can remain positive for `r`
edges. Obligation swaps, plateaus, ascents, positive sinks, and wholly positive
cycles are rejected locally. A debt-free-to-positive-to-debt-free cycle remains
legal because it starts a new bounded cleanup episode.

An optional reverse evidence bundle is checked against the endpoints in reverse
and against the reversed writer transfer. It is not an executable rollback and
is not graph adjacency. An explicit reverse edge supplies finite graph
adjacency and must independently satisfy every graph and cleanup law, but even
that does not prove an operation is executable or runtime rollback is
available.

This theorem is exact for the supplied finite machines, contexts, certificates,
writer labels, and ranks. It does not establish source-code equivalence,
operation executability, Git identity, runtime authority enforcement,
cross-carrier composition, carrier creation/removal, or path existence.

After graph validity, a future path optimizer may retain edge costs as a vector:

```text
semantic carriers touched
single-authority compatibility-adapter lifetime
API/ABI churn
proof and test reruns
runtime risk
build disruption
developer time
```

Find a feasible multiobjective path, not merely the best endpoint. This search
is outside `validate-migration`; the current command adds no hidden weights and
does not infer paths.

Temporary adapters require:

```text
one authoritative direction
named consumers
removal predicate
monotonically decreasing removal distance
```

A bidirectional adapter with two writable authorities has changed the state
semantics.

## Migration hysteresis

Approach the same target topology through different edit orders. Compare
remaining adapters, caches, dual representations, compatibility commitments,
and proof burden. Different outcomes show path dependence that endpoint graphs
miss.

Do not call this thermodynamic metastability. It is a discrete revision-path
effect with measurable debt.

## Global-optimum language

General semantic equivalence is undecidable; common partition problems are
NP-hard; ideal Kolmogorov complexity is uncomputable; some computable functions
have no asymptotically best program in the relevant Blum sense. Multiobjective
problems also need not have one best point.

Use “globally optimal” only when all are fixed:

- finite semantics and environment;
- finite admissible candidate language or a complete solver encoding;
- objective definitions and units;
- full enumeration or a solver certificate; and
- reported optimality gap.

`kerf pareto` is exact only over the represented-feasible supplied set; it
claims no candidate-space exhaustiveness and does not search arbitrary Rust
architectures.

## Stop rule

Stop architecture search when:

- every candidate is lawful at the required evidence level;
- no robust feasible neighbor dominates the current candidate;
- material residual costs are close to valid lower bounds or have no justified
  lower-bound model yet;
- further search costs more than the unresolved opportunity under explicit
  decision economics; and
- the next capability can be added through the same stable boundary vocabulary
  without new duplicate authority or special-case code.
