---
name: optimize-program-seam-topology
description: Derive and test constraint-preserving program seams from source, runtime, state ownership, history, ABI, and build evidence. Use when rearchitecting a semantics-sensitive system, breaking up a monolith without duplicate authority, aligning structurally different implementations, choosing module/state/team boundaries, or planning behavior-preserving migrations. Also use for program folding, seam geometry, hypergraph partitioning, or minimum-description-length architecture.
---

# Optimize Program Seam Topology

## Purpose

Turn architecture recovery into an identity-bound constrained-search problem.
First define what behavior may never change. Then infer the smallest useful
semantic units, partition their typed dependency hypergraph, and find a
low-risk sequence of behavior-preserving moves toward a lower-entropy design.

This skill produces a decision aid and migration contract, not an automatic
proof. Runtime evidence and source semantics remain authoritative.

## Core rule

Never encode protected semantics as a large finite penalty. A solution is
feasible or it is not. Optimize engineering objectives only inside the feasible
region.

Separate three problems that are often mistakenly collapsed:

1. **Semantic quotienting:** which states, transitions, and observations are
   proven equivalent or safely replaceable?
2. **Ownership partitioning:** which surviving units should share one owner and
   boundary?
3. **Migration routing:** which sequence of feasible revisions reaches a better
   partition without crossing an unverified state?

## Workflow

### 1. State the decision and stopping condition

Name the implementation or migration decision, its consumer, and the evidence
that retires the analysis. Do not create a permanent architecture dashboard.

Record a stable decision ID, consumer, retirement condition, source identity,
scope, supported workloads, compatibility posture, and whether the work is
diagnosis, design, or implementation. Give every protected observation a stable
ID. Read the repository's instructions before extracting evidence.
Bind prior accepted, rejected, or superseded architecture decisions when they
bear on the same seam. A large file, old plan, or attractive diagram is not a
reason to reopen a previously falsified migration without new evidence.

### 2. Define protected observations

List the externally or semantically observable facts that must remain invariant:
state traces, arithmetic width, ordering, identity, allocation, randomness,
reset, persistence, failure behavior, ABI, timing bounds, or security rules.

Give each invariant a named observation function, required closure, exclusive
strict negative control, and revision-bound evidence route. Distinguish:

- exact trace equivalence;
- refinement, where the target may contain unobservable internal steps;
- bounded conformance over named workloads; and
- an unproved assumption.

Read [semantic-preservation.md](references/semantic-preservation.md) before
claiming equivalence.

### 3. Build the typed temporal hypergraph

Extract vertices for canonical state atoms, immutable facts, transitions,
scheduler phases, observations/contracts, derived views, files, and ownership
surfaces. Extract directed typed hyperedges for read/write sets, control and
happens-before order, RNG draws, identity/lifetime, emissions, reset, ABI,
runtime flow, co-change, edit collision, and build invalidation.

Do not flatten different edge types into one adjacency matrix before deciding
what information can be lost. Read [formal-model.md](references/formal-model.md)
and [evidence-extraction.md](references/evidence-extraction.md).

### 4. Quotient only proven equivalences

Merge duplicated representations only when a representative and observation
map are explicit. Use partition refinement, program slicing, dependence graphs,
or source-directed proof to find the coarsest safe semantic units. Unknown is
not equivalent.

Keep one canonical mutable representative per fact unless the declared system
semantics intentionally includes replication or event-sourced authority. A
cache, projection, or replica must state its source/protocol, validity interval,
and invalidation or consistency rule.

Evidence for a fold must bind the exact candidate subject, claim, closure,
plan/receipt, evaluator, skill procedure, and one exclusively owned passing
negative control. Reject orphan evidence and shared controls. A command string
or artifact path by itself is not proof; use Semantic Architecture executed
evidence when the decision needs physical execution or parity.

### 5. Generate candidate folds

Start from mandatory groups and pinned boundaries. Coarsen strong, compatible
contacts; partition; then uncoarsen with local moves. Preserve typed edge
receipts so every proposed cut is inspectable.

Use hierarchical candidates, not one forced granularity. Avoid balance
constraints unless deployment, memory, build, or team capacity genuinely
requires them.

### 6. Rank feasible candidates without magic weights

Reject hard-constraint violations first. For remaining candidates report the
objective vector and Pareto dominance:

- duplicate mutable authority;
- synchronization frequency and volume;
- mutable and ordered boundary cuts;
- fixed-codec boundary information rate;
- level/product-specific description length;
- public contract churn;
- edit contention;
- runtime and memory cost;
- expected rebuild cost;
- migration cost; and
- proof or observability deficit.

Use lexicographic priority levels or epsilon constraints for policy. Use a
weighted scalar only when weights have empirical units, provenance, and
sensitivity analysis. Read [optimization.md](references/optimization.md).

### 7. Design the migration funnel

Treat revisions as nodes and permitted transformations as edges. Prefer a path
whose every node is buildable, whose protected observations remain proved, and
whose architecture vector is nondominated or improves a stated priority.

Use small reversible moves, temporary adapters with removal conditions, and
translation validation at each step. Bind every migration node to its exact
source/candidate identity and every edge to focused positive evidence, a
discriminating control, and a countermetric. A lower final score does not
justify an unverified intermediate. Read
[migration-landscapes.md](references/migration-landscapes.md).

### 8. Challenge the result

Test at least these counter-hypotheses:

- the apparent module is only a file-layout artifact;
- co-change reflects a broad mechanical edit rather than semantic coupling;
- runtime traces missed a dormant but valid path;
- an attractive cut moves authority rather than removes duplication;
- a generic interface erases ordering or identity;
- the proposed fold improves one workload while regressing another;
- file size is being mistaken for a semantic or ownership boundary;
- a prior rejected architecture is being repeated without a changed falsifier;
- the result is sensitive to one arbitrary coefficient; and
- the migration contains a kinetic trap: a locally tidy state that blocks the
  next required capability.

### 9. Emit a compact decision packet

Report:

1. protected observations and proof strength;
2. graph scope and known blind spots;
3. canonical ownership table;
4. feasible candidate partitions and Pareto vector;
5. decisive seam cuts with crossing-edge receipts;
6. migration sequence with per-step validation;
7. rejected alternatives and why;
8. mysteries requiring new evidence; and
9. the condition that retires the analysis.

Do not report a single mystical “energy.” Preserve the vector and the hard
constraints that made it meaningful.

When the user wants one exhaustive run, use an adaptive campaign. Recompute
only affected evidence after an accepted move, retain counterexamples, and stop
only at a declared fixed point. A second identical static scan is not new
knowledge. Read [pilot-accretion.md](references/pilot-accretion.md) for the
failure modes mined from Dispersion and PowerBI.

## Deterministic evaluator

Use `scripts/fold_graph.py` to validate and score a graph/candidate pair or to
enumerate nondominated single-representative moves:

```bash
python3 scripts/fold_graph.py validate --graph graph.json --candidate candidate.json
python3 scripts/fold_graph.py score --graph graph.json --candidate candidate.json
python3 scripts/fold_graph.py moves --graph graph.json --candidate candidate.json
python3 scripts/fold_graph.py identity
python3 scripts/fold_graph.py converge --graph graph.json --candidate candidate.json
python3 scripts/fold_graph.py self-test
```

The output is versioned JSON on stdout. Diagnostics and typed failures go to
stderr with nonzero exit status. Read
[graph-schema.md](references/graph-schema.md) before constructing inputs. The
v2 evaluator rejects duplicate JSON members and unowned evidence, binds output
to canonical graph/candidate plus evaluator/skill hashes, and requires exact
claim/control metadata. It does not execute or authenticate the referenced
receipts and cannot establish semantic truth by itself.

`converge` enumerates the complete declared one-representative move generator
over existing modules. First run it without `--prior-state-sha256`; if no
strictly dominating move exists, run it again with the emitted state hash. Only
an unchanged second result may return `bounded_single_move_fixed_point`. This
is a local Pareto result for that finite generator—not repository-wide
exhaustion, a global optimum, or evidence that an external discovery sweep
happened.

## Protein-folding lens

Use the biological analogy to generate hypotheses, never as proof. Contacts,
domains, funnels, local frustration, allostery, chaperones, cotranslational
folding, and neutral networks each have a precise but limited software analog.
The analogy's safe mappings and breaking points are in
[protein-folding.md](references/protein-folding.md).

## Research routing

- Read [formal-model.md](references/formal-model.md) for notation and the
  multilayer directed-hypergraph representation.
- Read [evidence-extraction.md](references/evidence-extraction.md) to derive
  vertices, edges, measurements, and uncertainty from real repositories.
- Read [optimization.md](references/optimization.md) for exact and heuristic
  solvers, MDL, information bottleneck, Pareto search, and sensitivity checks.
- Read [semantic-preservation.md](references/semantic-preservation.md) for
  slicing, refinement, simulation/bisimulation, and translation validation.
- Read [migration-landscapes.md](references/migration-landscapes.md) for
  incremental, parallel-safe refactoring paths.
- Read [protein-folding.md](references/protein-folding.md) for the biological
  research and disciplined analogy.
- Read [applications.md](references/applications.md) for domains beyond legacy
  ports and for failure modes.
- Read [pilot-accretion.md](references/pilot-accretion.md) before designing a
  repeated whole-repository campaign or claiming a stopping point.
- Read [bibliography.md](references/bibliography.md) for primary research and
  DOI links.

## Guardrails

- Never infer behavior preservation from a green compile, call graph, file
  move, or clustering score.
- Never let generated architecture replace repository truth or domain review.
- Never optimize module count, symmetry, line count, or “cleanliness” without a
  named delivery effect.
- Never use history, tests, or traces as proof of paths they did not exercise.
- Never place proprietary or source-restricted facts in a reusable global
  skill; keep case-specific analysis inside its authorized repository.
- Stop when the named decision is made and the next implementation step is
  independently landable.
