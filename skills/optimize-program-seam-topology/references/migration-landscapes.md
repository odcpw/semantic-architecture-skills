# Migration landscapes: folding without semantic excursions

## 1. The second optimization problem

Finding a good final partition is not enough. Let each node in a migration
graph be a concrete repository revision plus its validated inputs and build
configuration. An edge is one independently landable transformation.

```text
G_migration = (revisions, verified transformations)
```

Find a path from current revision `r0` to a target region, subject to every
intermediate remaining feasible.

The path has its own objective vector:

```text
g(path) = (
  maximum semantic risk of any step,
  maximum temporary duplication,
  total proof/gate cost,
  total touched surface,
  adapter lifetime,
  integration/contention delay,
  peak runtime/build regression,
  rollback complexity
)
```

This prevents a globally attractive architecture from justifying a dangerous
“big bang” path.

## 2. Neutral network

Think of revisions that preserve protected observations as a neutral network.
Safe refactors move within it; feature additions deliberately change the
specified phenotype.

This suggests a practical rule:

- separate isomorphic/behavior-preserving moves from semantic additions;
- validate every neutral move against the same observation relation; and
- use the neutral network to reach a representation from which the next feature
  is cheaper.

Neutral does not mean costless. A revision can preserve behavior while
regressing compile time, runtime, comprehensibility, or future reachability.

## 3. Folding funnel

A useful migration funnel has many feasible microscopic routes but a general
bias toward:

- fewer duplicate authorities;
- smaller mutable cut surfaces;
- more explicit source schedule and contracts;
- less product/level-specific code; and
- cheaper proof of the next move.

Do not demand that every metric improve every step. Require hard feasibility
and an explicit reason for temporary regressions. Bound them and give them a
removal milestone.

## 4. Kinetic traps

A kinetic trap is a locally tidy architecture that makes the next required
capability or proof harder. Common examples:

- extracting a generic interface before the behavior inventory is known;
- freezing a wrong canonical owner behind a polished facade;
- using runtime plug-ins where source order is static and exact;
- splitting crates/services before state ownership is clean;
- generating a broad compatibility layer that becomes permanent;
- simultaneously moving files and changing state semantics; or
- optimizing the current level/product into a type that must be copied for the
  next one.

Challenge a candidate with the next two known capabilities, not an imagined
infinite future.

## 5. Contact order and migration difficulty

Protein contact order measures sequence separation between residues that must
contact in the folded structure and correlates with folding kinetics for some
protein classes. A cautious software analog is **migration contact distance**:

```text
MCD = sum_e importance(e) * separation(e) / normalization
```

where separation may be ownership boundaries, build units, language/runtime
boundaries, or number of dependent migration stages between semantic units
that must ultimately coordinate.

Use MCD as a risk hypothesis. Unlike a polymer chain, source-file distance has
no universal physical meaning. Calibrate separation against observed change,
build, and integration delay.

## 6. Chaperones

Biological chaperones reduce aggregation or reshape folding pathways without
determining the final native structure. Temporary software “chaperones” can be:

- a facade preserving old paths during a pure move;
- a dual-observation comparison that exposes old/new state without dual writes;
- an adapter translating a frozen ABI to canonical state;
- a feature flag selecting one authority at a time; or
- a trace bridge used only for validation.

Every chaperone needs:

```text
purpose
single authority rule
admission and failure behavior
performance bound
removal condition
latest removal milestone
```

A writable mirror with indefinite lifetime is not a chaperone; it is duplicate
authority.

## 7. Cotranslational folding

Proteins can begin folding while synthesis continues; the available landscape
depends on chain length and translation rate. For evolving programs:

- establish the canonical ownership and schedule seam as new capabilities are
  added;
- do not wait for all future features before validating a useful local domain;
- do not finalize a global abstraction from the first partial capability; and
- pace refactoring so temporary states do not accumulate faster than they are
  resolved.

This supports incremental generalization: census the next capability, add only
new shared behavior, validate the current positive path, then reassess the fold.

## 8. Local frustration

Not all cross-boundary tension should vanish. Localized high-friction sites can
be functional:

- a public ABI deliberately isolates incompatible representations;
- a scheduler centralizes nonlocal ordering;
- a provenance boundary prevents data/code transfer;
- an unsafe kernel contains a necessary low-level operation; or
- a transaction boundary coordinates several state domains atomically.

The goal is minimum unnecessary frustration, not zero coupling. Localize,
name, observe, and test necessary frustration instead of smearing it through
many modules.

## 9. Allostery and nonlocal effects

In proteins, local changes can alter distant functional behavior. In programs,
RNG consumption, scheduler order, allocation, cache invalidation, and global
configuration are common nonlocal channels.

Before moving a seemingly local transition, inspect its full backward/forward
slice for these channels. A small local code edit can have a large semantic
radius.

## 10. Move taxonomy

Prefer one transformation class per patch:

1. pure physical move with stable facade/re-export;
2. representation relation and dual observation;
3. canonical owner cutover;
4. removal of old representation/adapter;
5. scheduler extraction preserving literal trace;
6. dispatch/data generalization;
7. public contract extension; or
8. new semantic capability.

Combining classes makes failures harder to localize and evidence harder to
attribute.

## 11. Migration edge contract

For each planned edge record:

| Field | Requirement |
|---|---|
| capability | named behavior or architecture improvement |
| ownership | narrow code/state surface |
| precondition | exact prior revision and required inputs |
| transformation | one class from the taxonomy |
| invariants | protected observations affected |
| evidence | focused positive, negative, and nearest real consumer |
| countermetric | runtime/build/resource bound |
| rollback | revert only this coherent change if it loses |
| adapters | owner and removal condition |

The record can live in a task description or implementation note; do not build
a parallel workflow system for it.

The positive and negative evidence must be separately identifiable and bound to
the exact edge endpoints. A control cannot serve two claims. If a candidate
node's source, evaluator, skill procedure, observation closure, or nonclaims
change, its prior receipt and fixed-point state are stale.

## 12. Parallel execution

Parallelize independent leaves, not shared cores. Derive lanes from the
hypergraph:

- no simultaneous writer on the same representative, contract, scheduler, or
  canonical state carrier;
- shared-read source archaeology can proceed in parallel;
- one integrator owns each high-cut compatibility surface;
- workers run focused evidence; one owner runs expensive integration; and
- moving HEAD is normal when edit surfaces are disjoint.

An objective for worker scheduling can minimize overlapping edit hyperedges and
critical-path duration, but orchestration is justified only when the tasks are
genuinely independently landable.

## 13. Choosing the next move

Select a move that:

- resolves an active delivery blocker or removes a measured recurring cost;
- changes one canonical carrier or seam;
- has a discriminating failure test;
- leaves the real positive route working;
- reduces or bounds future migration contact distance; and
- does not pre-decide an unmeasured future abstraction.

If no such move exists, the missing product is evidence, not a larger refactor.
