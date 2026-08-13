# Rust systems as explicit behavioral factorizations

## Ownership is a realization, not the semantics itself

Start with one semantic fact and one explicit authority protocol. A single
owner is the simplest protocol for ordinary local mutable state. Replicated
state machines, CRDTs, immutable snapshots, caches, event logs, and read
replicas are lawful only when their derivation, convergence, consistency,
failure, and reset rules are part of the behavior.

Rust maps useful capabilities into code:

```rust
fn inspect(state: &State) -> Observation;
fn update(state: &mut State, command: Command) -> Result<Effects, Error>;
```

`&T` prevents mutation through that reference under safe Rust; `&mut T` grants
a scoped exclusive capability. Neither proves that `State` is the correct
semantic owner or that a module boundary is optimal.

`Arc<Mutex<T>>` chooses shared lifetime, serialization, contention, poisoning,
lock order, and coherence behavior. It is not decoupling. Atomics expose a
memory-ordering seam. `Send` and `Sync` prove allowed transfer/sharing under the
type system, not determinism, linearizability, fairness, deadlock freedom, or
performance.

## Group state by operative geometry

Prefer grouping by:

- writer/authority;
- scheduler phase and protected order;
- lifetime and reset epoch;
- access frequency and physical placement; and
- invariants established atomically.

Grouping only by domain noun often creates broad mutable interfaces. Splitting
only by file size creates coordination without semantic value.

## Make effects explicit ports

Hidden global RNG, time, allocation, filesystem, telemetry, and effect sinks are
unmodeled seams. Use narrow capabilities or explicit receipts:

```rust
pub fn step(
    state: &mut State,
    input: Input,
    rng: &mut impl Random,
    effects: &mut impl EffectSink,
) -> Result<(), StepError>;
```

Do not replace explicit capabilities with one giant `Context`; parameter-count
reduction can increase semantic boundary width.

Where draw count or ordering matters, RNG is an owned ordered state carrier,
not a utility service. Where allocation identity is observed, allocator/free
list/generation state belongs in the protected model.

## Typestate and protocols

Typestate can make illegal finite protocol transitions unrepresentable to
typed callers. It is useful for initialization, transaction, mapping, handshake,
and ownership-transfer phases. Highly dynamic protocols may be clearer as an
enum plus a checked transition table.

Typestate does not remove runtime state or prove external semantics. Validate
the transition relation itself.

## Async cancellation seams

Every `.await` is a possible temporal cut: the future may never resume. Before
each suspension, ask which invariants hold if cancellation occurs immediately.

Prefer one of:

- leave canonical state unchanged until external work commits;
- record a durable intent;
- move intermediate authority into an owner that outlives the future;
- make the operation idempotent and replayable; or
- specify explicit shutdown and cleanup protocols.

Holding a broad mutable borrow or synchronization guard across suspension makes
ownership duration scheduler-dependent. Cancellation tests should inject drops
at every reachable suspension point and verify state, protocol, and resource
invariants.

## Scheduling and trace order

Do not parallelize merely because final sets match. Protected behavior can
include:

```text
phase, cursor, scan direction, continuation/rescan,
RNG draw order, allocation/reuse order, emission order, failure mutation
```

Represent scheduler state explicitly. Prove allowed commutations against the
observation relation. A conservative footprint can establish independence of
disjoint actions; shared footprints do not prove noncommutativity and require a
stronger test.

## Direct calls, generics, and message routers

- A direct borrow/call is appropriate inside one lifetime, scheduling, and
  failure domain.
- Static generics are dedicated wiring: analyzable and often fast, but can
  increase code size and build invalidation.
- Trait objects and buses are routers: valuable for genuine dynamic multiplexing,
  but paid for in erased information, indirection, arbitration, protocol
  surface, and sometimes allocation.
- Owned bounded messages are appropriate when authority crosses a scheduler or
  failure boundary.

Do not install a generic event bus on a stable hot point-to-point path without
measured variability that amortizes it.

## Layout and locality

`repr(Rust)` does not define a stable external layout. Use declared schemas,
`#[repr(C)]`, or `#[repr(transparent)]` where their documented contracts fit.
Never persist or transmit arbitrary Rust memory images as an implicit ABI.

Choose AoS/SoA, padding, sharding, and NUMA placement from access vectors and
measurements. A semantic module move with unchanged physical placement predicts
no locality improvement. False-sharing fixes can improve a logically unchanged
design, showing that the physical seam was below the source-language boundary.

## ABI seams

An FFI contract includes more than layout:

- allocator ownership and deallocation;
- nullability, alignment, aliasing, and lifetimes;
- panic/unwind behavior;
- error representation;
- threading, reentrancy, and cancellation;
- handle identity/generation;
- version and cursor binding; and
- mutation-before-failure behavior.

Opaque handles plus generations are often safer than exporting internal state.
Validate all inputs before entering safe authoritative logic. Compile consumers
independently and test version skew.

## Observability

Prefer:

```text
canonical transition -> immutable receipt -> trace/metric/log
```

Instrumentation must not become a second writable representation. If tracing
changes protected timing or scheduling, that perturbation is part of the
observation contract. For deterministic cores, compare trace-on and trace-off
protected observations.

## Crates, builds, and work surfaces

A crate is a namespace/API and build unit. It is not automatically a semantic
component. Measure public contract churn, incremental invalidation, codegen,
and link effects separately.

Define an agent work surface by:

```text
authoritative carriers
permitted transitions
editable interfaces
invariants
required evidence receipts
```

This is safer than arbitrary file ranges. Parallel agents may consume shared
immutable snapshots and independent proof obligations, but never concurrently
rewrite one canonical state protocol.

## Rust-specific falsification tests

- Replace a broad state argument with the inferred port; hidden access must fail
  to compile or appear as an explicit command/effect.
- Randomly cancel at every await and check invariants.
- Change task scheduling; protected traces must remain equal where commutation
  was claimed.
- Compare same-core, cross-core, NUMA, process, and FFI placement.
- Compile ABI producer and consumer independently and fuzz invalid handles/tags.
- Measure incremental builds before moving crate boundaries.
- Search for clone/synchronize cycles, broad `Arc<Mutex<World>>`, and lifetime-
  erasing indirection as evidence that physical and semantic ownership disagree.
