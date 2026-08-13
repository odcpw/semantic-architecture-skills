# Evidence extraction

## Principle

Extract evidence for the decision at hand. Do not build a universal repository
knowledge graph before a consumer exists. Start with the smallest behavior
slice that reaches the named capability and expand only when a crossing edge or
counterexample requires it.

## 1. Establish the observation boundary

Before parsing code, write a table:

| Observation | Exact or approximate | Workloads | Oracle | Failure meaning |
|---|---|---|---|---|
| state hash | exact | named inputs | current/source implementation | any mismatch |
| event order | exact | event-producing inputs | trace capture | permutation or omission |
| latency | bounded | representative benchmark | pinned baseline | distribution regression |

An empty workload set or absent oracle makes a later equivalence claim invalid.
Also pin the decision ID, consumer, retirement condition, structured
source/tree/dirty identity, each observation-manifest SHA-256, evaluator, skill,
normative procedure-manifest identity, and explicit nonclaims. A branch name,
human-readable revision label, or command string is not a content binding.

## 2. Choose semantic granularity

Use the smallest units that can move or own state independently, but no smaller:

- a state field or tightly coupled state group;
- a transition or source dispatch case;
- a scheduler phase;
- an immutable input table;
- a stable ABI operation;
- a derived index with one validity interval;
- a file only when compilation or writer ownership makes the file itself
  relevant.

Avoid one vertex per token, statement, or source callback unless that is the
actual decision surface. Excessive granularity explodes the search space and
creates attractive but unusable cuts.

## 3. Static semantic evidence

### Program and system dependence

Build or approximate a program dependence graph with:

- control dependence;
- def-use and alias-aware data dependence;
- call/return and parameter edges;
- interprocedural summary edges; and
- exceptional and failure control flow.

For stateful systems add:

- ownership and mutation sites;
- allocator/free-list/generation changes;
- persistence and reset paths;
- cache construction, validity, and invalidation;
- protocol/ABI encode-decode paths; and
- integer width, signedness, wrapping, and unsafe/FFI boundaries.

Static absence is meaningful only within the analysis assumptions. Reflection,
dynamic loading, generated code, FFI, pointer aliasing, build scripts, and
runtime configuration must be named blind spots.

### Slicing

Use backward slices from each protected observation to find all possible
influences and forward slices from authoritative state to find consumers. Use
interprocedural slicing for call-context sensitivity. A slice is a candidate
semantic unit or proof obligation, not automatically a module.

Dynamic slices are narrower and workload-specific. Keep them tagged with the
exact input; never substitute them for the static possibility set.

### Structured source facts

Prefer AST, compiler, schema, or binary-layout tooling to regex when extracting:

- field and type layouts;
- match/dispatch keys;
- generated tables;
- public symbols and ABI layouts;
- package manifests and version constraints; and
- build dependency edges.

Manual source archaeology is valid when the relevant code is small and its
evidence is recorded with line/symbol/revision references.

## 4. Dynamic evidence

Instrument the nearest real route. For each transition capture only facts the
decision needs:

```text
phase, transition/site, state reads, state writes,
ordered RNG receipts, identity/allocation operations,
ordered emissions, failure/mutation outcome, duration
```

Use stable IDs rather than memory addresses. Bound trace volume and keep a
schema/version. Capture input identity, revision, build flags, seed, and
environment.

Aggregate without erasing rare edges:

- frequency and total volume;
- maximum burst and tail latency;
- workload coverage count;
- first/last phase;
- ordered sequence signatures; and
- edge presence per workload.

An edge that appears once may be more semantically important than a million
ordinary reads.

## 5. Mutable ownership inventory

For every mutable fact ask:

1. What is its semantic identity?
2. Which representation is canonical?
3. Who can write it?
4. Which copies exist and when do they diverge?
5. What synchronizes them, at what frequency and volume?
6. What observes ordering between writes and copies?
7. What happens on reset, rollback, failure, and reuse?

Classify every representation relative to a named reset/lifecycle operation and
guard; one field can be persistent under a soft reset and resettable under a
campaign restart. Use:

```text
canonical authority
immutable baseline/fact
derived view/cache
external projection
temporary adapter
replica with declared consistency semantics
unclassified (blocks optimization)
```

A clone used for a transaction is not necessarily duplicate authority if only
one candidate can commit and failure is atomic. Its copy cost remains an
objective. A synchronized writable mirror is duplicate authority until proven
otherwise.

For exact topology, use an observational test: if a backward slice from a
protected future observation reaches slot/list/queue order or identity, the
topology is semantic. Otherwise a counterfactual reorder with fixed logical
contents can test whether the representation is safely derivable.

## 6. Temporal and ordering evidence

A call graph cannot express:

- pass order;
- rescan/retry loops;
- allocation into earlier or later iteration positions;
- rebuild points for indexes;
- pre- and postpasses;
- draw/emission order; or
- validity intervals.

Represent these as phase vertices and ordered hyperedges. For an ordered list
`[a,b,c]`, record adjacency or a partial order plus the scope in which it must
hold. Do not overconstrain independent events into a total order.

When a scheduler rebuilds a derived index, record:

```text
invalidating mutations -> rebuild phase -> valid-use interval -> next invalidation
```

## 7. Runtime cost

Measure the candidate boundary mechanism, not a generic abstraction benchmark.
Relevant units include:

- bytes or fields copied per tick;
- calls or messages per transaction;
- allocations and peak memory;
- cache misses or cycles when stable tooling exists;
- latency distributions and throughput;
- code size; and
- failure/rollback work.

Pin workload, revision, hardware, build mode, warmup, sample count, and competing
load. Keep a correctness countermetric.

## 8. Build cost

Build graphs and file boundaries are a distinct layer. Measure:

- clean and incremental build durations;
- which edits invalidate which units;
- artifact size and link cost;
- peak CPU/RAM; and
- cache hit/miss behavior.

An expected rebuild estimate can be computed as:

```text
sum_m build_cost(m) * P(module m invalidated by a representative change)
```

Estimate the probability from a defined history window or active work plan.
Do not treat old history as a stationary law.

## 9. Change and coordination evidence

Version history helps distinguish stable seams from shared hotspots, but it is
noisy. Extract:

- commits touching two semantic units;
- time-decayed co-change;
- authored versus generated/mechanical changes;
- revert/fix-followup incidents;
- concurrent writer overlap; and
- merge/reservation collisions.

Normalize for broad commits. One simple bounded measure for a commit `c` that
touches `n_c` units is to contribute `1/(n_c-1)` to each pair instead of one.
Exclude known formatting, generated, vendored, and mass-move commits when that
classification is reliable.

Co-change is ambiguous:

- high co-change may mean semantic cohesion;
- it may mean a leaky contract;
- it may reflect one cross-cutting migration; or
- it may be an artifact of one large file.

Use it to challenge source/runtime evidence, not to overrule it.

## 10. ABI and external contracts

Treat a cross-language or persisted boundary as one logical compatibility
surface even when it spans many files. Record:

- layout, alignment, width, endianness, version, and symbol identity;
- lifecycle and ownership of buffers/handles;
- error code and fail-closed behavior;
- cursor, generation, and transaction binding;
- idempotence and reset semantics; and
- real consumers and deployment routes.

Contract churn should be counted semantically. Updating five mirrors for one
ABI change is one public decision plus five synchronization obligations, not
five independent features.

## 11. Uncertainty ledger

For each graph layer state:

| Field | Meaning |
|---|---|
| scope | files, symbols, levels, workloads, time window |
| extraction | exact tool/command or manual method |
| coverage | what was and was not observed |
| freshness | revision/date |
| confidence | proved, exhaustive, sampled, inferred, unknown |
| bias | likely direction of error |
| retirement | evidence that resolves the uncertainty |

Keep this inside the decision packet. Do not create a permanent governance
ledger unless another deliverable genuinely consumes it.

## 12. Minimal extraction sequence

For most legacy-port seams:

1. define protected observations;
2. identify canonical mutable facts and duplicate representations;
3. derive static backward slices from those observations;
4. instrument one real positive path and one discriminating negative;
5. extract ABI/reset/failure edges;
6. add recent co-change and build layers only if they can change the decision;
7. propose two or three candidate folds;
8. gather new evidence only for edges that distinguish those candidates.

This active-learning loop is cheaper and more informative than exhaustively
mapping the entire repository first.
