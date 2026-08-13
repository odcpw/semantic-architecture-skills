# Applications and limits

## 1. Semantics-sensitive legacy ports

### Decision

Map source state, schedule, identity, arithmetic, and externally visible effects
into a new language/runtime while improving ownership and modularity.

### High-value layers

- system/program dependence;
- exact source schedule and trace;
- mutable authority and reset;
- ABI/presentation boundary;
- source-target correspondence; and
- migration proof cost.

### Typical fold

Immutable source facts + one canonical live state + explicit scheduler + static
behavior dispatch + external projections. Product/level variants become
validated descriptors plus genuinely new shared behavior.

### Failure mode

Grouping by source files or target object types while erasing nonlocal schedule,
RNG, allocation, and reset semantics.

## 2. Monolith decomposition

### Decision

Choose internal modules or service boundaries without turning local calls into
distributed consistency protocols unnecessarily.

### High-value layers

- mutable transaction hyperedges;
- data ownership and persistence;
- runtime request flow;
- failure/rollback semantics;
- deployment scaling requirements;
- co-change and team ownership; and
- latency/availability budgets.

### Rule

Partition after data/transaction ownership is explicit. A dense mutable
hyperedge is often a reason to remain one process or one transactional owner.

### Failure mode

Optimizing call-graph modularity, balanced service size, or team count while
ignoring data consistency and failure amplification.

## 3. State-machine and protocol extraction

### Decision

Separate protocol phases or handlers while preserving legal traces and failure
behavior.

### High-value methods

- transition-system partition refinement;
- simulation/bisimulation;
- temporal hyperedges;
- translation validation; and
- model checking on the reduced state space.

### Failure mode

Treating messages as independent methods and losing order, retransmission,
idempotence, cursor, or generation semantics.

## 4. Compiler/interpreter refactoring

### Decision

Split passes, IRs, and analyses or replace one transformation while preserving
program semantics.

### High-value methods

- dependence graphs;
- pass/analysis invalidation edges;
- refinement and translation validation;
- compile-time/build objective; and
- differential/fuzz conformance.

### Failure mode

Using golden output alone when multiple correct outputs exist, or claiming a
pass is independent while it consumes hidden analysis state.

## 5. Database and persistence ownership

### Decision

Normalize duplicated state, split schemas/services, or migrate stores.

### High-value layers

- transaction read/write hyperedges;
- constraints and indexes;
- isolation/order and failure;
- replication/change-data-capture;
- schema/ABI compatibility; and
- migration/rollback path.

### Failure mode

Calling replicas or denormalized projections duplicate authority without
modeling consensus/commit semantics, or treating independently writable mirrors
as harmless caches.

## 6. Distributed systems and actor boundaries

### Decision

Assign state and messages to processes/actors while minimizing coordination and
preserving safety/liveness.

### High-value methods

- open-system/refinement models;
- message-order and happens-before layers;
- failure domains;
- ownership/lease/consensus state; and
- latency, availability, and cancellation objectives.

### Failure mode

Using a static cohesion score that ignores retries, partitions, backpressure,
cancellation, or partial failure.

## 7. Real-time and game simulation engines

### Decision

Separate deterministic simulation, presentation, asset facts, and tools while
preserving tick order and replay.

### High-value layers

- scheduler phases;
- RNG and allocation;
- canonical state/hash/reset;
- simulation-relevant asset facts;
- presentation journals/projections; and
- frame/runtime budgets.

### Failure mode

Letting rendering/Unity/UI become gameplay authority, using unordered ECS
iteration where stable source order is semantic, or making one world per level.

## 8. Safety-critical and embedded systems

### Decision

Minimize verification surface and resource cost while preserving certified
behavior.

### High-value methods

- hard resource and timing constraints;
- refinement/translation validation;
- hazard-trace observations;
- ownership and concurrency isolation;
- worst-case rather than average runtime; and
- explicit trusted computing base.

### Failure mode

Trading a safety invariant against average performance or treating sampled
tests as evidence of worst-case timing.

## 9. GPU kernels and data pipelines

### Decision

Fuse or split stages to reduce memory traffic and launch overhead without
changing numeric or ordering semantics.

### High-value layers

- producer/consumer hyperedges;
- tensor/buffer lifetime and aliasing;
- precision, associativity, and reduction order;
- device transfer and memory capacity;
- kernel compile and cache behavior; and
- workload-shape distributions.

### Failure mode

Assuming algebraic equivalence under floating point, or optimizing one tensor
shape and regressing the distribution.

## 10. API and ABI design

### Decision

Find the smallest stable contract that preserves consumer-relevant information
while hiding implementation detail.

### High-value methods

- information bottleneck/sufficient statistic lens;
- contract/consumer hypergraph;
- MDL for interface plus exceptions;
- compatibility/refinement; and
- version/migration cost.

### Failure mode

Minimizing field count while forcing consumers to reconstruct hidden state, or
exposing a giant context to avoid deliberate seams.

## 11. Product-line and level/campaign architectures

### Decision

Separate shared behavior capabilities from product-specific initial data and
objectives.

### High-value measures

- per-variant behavior inventory;
- capability closure and spawn/reachability;
- descriptor versus special-case MDL;
- package identity/fail-closed behavior; and
- marginal code/data delta per variant.

### Failure mode

One subclass/session/route/ABI operation per variant, or a universal DSL before
the real behavior vocabulary is measured.

## 12. Team and agent work partitioning

### Decision

Assign parallel implementation lanes with minimal file/logical-surface overlap.

### High-value layers

- edit collision and co-change;
- canonical semantic ownership;
- dependency critical path;
- expensive gate ownership; and
- integration boundary.

### Failure mode

Forcing balanced task sizes, splitting one mutable core among workers, or
creating tracker leaves for integration/verification that own no distinct
artifact.

## 13. Build-system and package topology

### Decision

Choose crates/packages/targets to reduce expected rebuild cost without bloating
public APIs or runtime boundaries.

### High-value layers

- actual build invalidation;
- compile/link time and peak resources;
- edit-frequency distribution;
- inlining/codegen needs;
- public API churn; and
- deployment ownership.

### Failure mode

Crate-per-domain based on aesthetics, clean-build-only measurements, or module
count optimization.

## 14. Security compartmentalization

### Decision

Partition privileges, secrets, parsers, and untrusted inputs to minimize attack
paths and blast radius.

### High-value layers

- data/privilege flow;
- trust boundaries;
- parser and deserialization transitions;
- identity/authentication state;
- failure and recovery; and
- runtime/operational cost.

### Rule

Security constraints remain hard. A lower coupling score never justifies a new
secret flow or privilege expansion.

## 15. Scientific workflow reconstruction

### Decision

Separate immutable inputs, algorithms, mutable experiment state, provenance,
and result projections while preserving reproducibility.

### High-value layers

- data lineage hyperedges;
- environment/version identity;
- random seeds and numeric semantics;
- cache derivation/invalidation;
- workflow scheduling; and
- artifact schemas.

### Failure mode

Treating cached or plotted data as authority, or claiming reproducibility from
source alone without inputs/environment.

## 16. When not to use the full method

Do not construct the full graph/solver apparatus when:

- one local owner and boundary are already obvious;
- the refactor is a small compiler-checked rename or extraction;
- no semantic/state/order risk exists;
- measurement cost exceeds the plausible design difference; or
- there is no implementation consumer.

Use the smallest relevant subset: often an ownership table, protected
observation list, two candidate cuts, and one focused experiment are enough.

## 17. Universal red flags

- one scalar “architecture energy” with unexplained coefficients;
- “infinite” constraints implemented as merely large weights;
- pairwise call graphs standing in for ordered state transitions;
- inferred absence treated as proof;
- balanced partitions without a capacity requirement;
- duplicate state hidden behind synchronization;
- traces without workload/revision identity;
- history without normalization for broad changes;
- an endpoint design without a feasible migration path;
- adapters without removal conditions;
- protein terms used without measurable software counterparts; and
- a framework whose analysis/process cost delays the named capability.
