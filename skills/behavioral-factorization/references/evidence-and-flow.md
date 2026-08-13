# Evidence, transport, and real resource flow

## Keep the layers typed

Extract evidence into separately queryable layers:

1. static control and data dependence;
2. dynamic state access and transition traces;
3. temporal order, scheduler phase, and continuation;
4. identity, allocation, lifetime, reset, and invalidation;
5. ABI, schema, failure, and external observations;
6. runtime rate, volume, latency, locality, and resource placement;
7. build invalidation and compile cost;
8. change history, edit collision, and ownership; and
9. provenance, coverage, uncertainty, and evidence strength.

Static analysis conservatively proposes possible influence. Dynamic absence is
not unreachability unless coverage is complete for the declared model. File
adjacency, imports, and co-change are hypotheses about architecture, not
semantic facts.

## Architecture includes placement, schedule, and routes

After semantic quotienting, model a candidate as:

```text
A = (representatives, ownership, logical partition,
     schedule, placement, routes, interface protocols, evidence)
```

The resource topology can include register, core/cache, coherence domain, NUMA
node, process, machine, network, and durable storage. Each crossing may have
capacity, latency, physical energy, fixed overhead, ordering, and failure
semantics.

Two identical module decompositions can have opposite performance because of
schedule, cache, NUMA, or route placement. Logical and physical seams are
different layers joined by an explicit materialization map.

## Multiscale seam profile

A single cut score hides scale. For workload `w`, inspect a typed isoperimetric
profile across feasible subset sizes:

```text
Phi_w(a) = minimum boundary flow / smaller-side useful work
           over feasible subsets at scale a
```

- Stable valleys across workloads suggest candidate boundaries.
- A uniformly high profile suggests an inseparable core under that evidence.
- A valley that vanishes when order, RNG, reset, or rare-path edges return was
  an artifact.

Rent-like terminal scaling can be measured per edge type. It is empirical,
scale- and partition-dependent. Reject the exponent when the power-law fit is
poor; never assume software obeys Rent's rule.

## Lower-bound accounting

For every comparable coordinate retain:

```text
L <= OPT <= U <= C
```

- `L`: proved floor under explicit assumptions.
- `U`: best feasible candidate actually demonstrated.
- `C`: current measured cost.
- `C-U`: demonstrated removable cost.
- `U-L`: unresolved opportunity.

Do not call `C-L` waste unless optimality is certified.

Applicable floors include:

- contextual state cardinality and communication complexity;
- Hong-Kung/red-blue-pebble I/O bounds for an explicit computation DAG and
  memory model;
- VLSI bisection/area-time constraints under the stated circuit model;
- capacity cuts and max-flow for a declared single-commodity network;
- congestion and dilation lower bounds under a packet-routing model;
- live-frontier/pathwidth pressure under a declared schedule; and
- arrival/service-curve backlog and delay bounds.

Each theorem has a model. A floor for bytes across one memory boundary is not a
floor for messages, byte-hops over a full route, or wall-clock latency.

## Movement regret

When an applicable lower bound and measured movement use identical units,
scope, workload, and counting convention:

```text
gap = measured movement / certified lower bound
```

A ratio near one says layout-only changes have little headroom under that
model. A large ratio identifies potential waste, not its cause. `kerf` v0.1
does not accept or compare lower-bound claims; that comparison remains external
until a mechanically checked certificate language exists.

## Schedule geometry

The live frontier of a schedule contains values already produced but still
needed by an unexecuted transition. Peak frontier bytes predict working-set,
register/cache pressure, borrow duration, and staged buffer demand [E].

A partition with a poor schedule can thrash memory. Optimize partition and
schedule together, while preserving protected order. In deterministic engines,
dense source-shaped phase passes can be better than task-per-entity scheduling
when state and order are homogeneous.

## Async seams and steady state

A live program is an open driven system; its literal minimum-power state is
halted. Use steady-state operational variables, not thermodynamic equilibrium.

At every asynchronous seam name:

```text
arrival process
service capacity
buffer capacity
burst assumption
backpressure/drop/coalescing rule
maximum delay
disconnect/reset/cancellation behavior
```

Average utilization `rho = lambda / mu` is not enough for burst guarantees.
Sustained `rho >= 1` implies overload under ordinary stable-queue assumptions.
Little's law relates average occupancy, rate, and residence time only under its
stationarity and finite-mean conditions. Deterministic network calculus needs
arrival and service curves before it can bound backlog or delay.

An unbounded queue does not remove backpressure; it converts overload into
memory growth.

## Dataflow specializations

- Synchronous dataflow can produce static periodic schedules and bounded
  buffers when production/consumption rates are fixed [T].
- Kahn process networks provide determinism under their blocking-read and
  conceptually unbounded-channel model [T].
- Latency-insensitive designs require explicit elastic protocols, ordering,
  bounded buffering, and liveness arguments [T/E].

Arbitrary Rust tasks, finite queues, shared memory, nondeterministic selection,
cancellation, and side effects do not inherit these theorems.

## Physical and cache boundaries

A crate boundary may inline away. Two adjacent fields can still communicate on
every write through one coherence line. Measure actual cache-line ownership,
remote-NUMA bytes, fences, atomics, allocations, syscalls, and serialization.

Safe hypotheses:

- colocate state with the same writer, phase, lifetime, and access frequency;
- separate unrelated hot writers only after false-sharing evidence;
- share immutable facts freely;
- stage parallel results and merge in canonical order where order is protected;
- use direct calls/borrows inside one scheduler and failure domain;
- use owned bounded messages when authority truly crosses domains.

Rust `Send`, `Sync`, atomics, or a lock prove neither good placement nor cheap
sharing.

## Literal energy and passivity

Use energy only for joules, a valid Hamiltonian/storage function, a Dirichlet
quadratic form, or another defined physical/mathematical quantity.

Passivity/dissipativity can compose when a real storage function and supply rate
are named. Software candidates include queued bytes, outstanding work credits,
retry debt, in-flight ownership transfers, and allocated-but-not-retired memory.
These are resource potentials, not thermodynamic energy.

Maximum-power impedance matching is generally the wrong software analogy: it
can sacrifice efficiency, while queue utilization near one destroys latency
headroom.

## Evidence extraction warnings

- A call graph proves a possible syntactic relation, not measured flow.
- A trace proves observed flow, not intentional routing or benefit.
- An undirected spectral graph discards order, guards, ownership, and failure.
- Graph sparsification preserves only the forms covered by its theorem.
- A proxy can correlate with outcomes while measuring a heterogeneous
  extraction fraction rather than the named semantic property.
