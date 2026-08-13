# Semantic preservation and proof obligations

## 1. Architecture moves are translations

A semantics-sensitive refactor translates one representation and transition
structure into another. Treat each move like a small compiler pass:

```text
source state/step --migration--> target state/step
           \                         /
            \---- observation ------/
```

The key question is not “do the tests still pass?” but “which relation connects
the old and new states, and which observations does it preserve?”

## 2. Observation functions

Define observation functions explicitly. Examples:

```text
O_state(x)   = canonical externally relevant state
O_trace(t)   = ordered phases, identities, draws, writes, emissions
O_reset(x)   = state after reset plus persistent exceptions
O_fail(s)    = error class, output, and mutation receipt
O_abi(b)     = public bytes/layout/symbol/lifecycle behavior
O_perf(t)    = named resource bounds
```

Different work can protect different functions. Do not claim whole-program
equivalence from one hash or screenshot.

## 3. Representation relation

For a state normalization, write a relation `R(source, target)`. It should map:

- every canonical mutable fact;
- identity and lifetime carriers;
- immutable source facts;
- derived views and their validity status;
- journals/cursors and pending effects;
- persistence and reset baselines; and
- error/transaction state.

If one source representation becomes several target components, state how they
compose. If several source copies become one target authority, specify which
copy was canonical and how inconsistency is rejected or resolved.

## 4. Simulation and refinement

### Forward simulation

For each related state pair `(s,t)` and source-observable target step, show that
the target step corresponds to an allowed source step or sequence and produces
related successor states. This supports an implementation/refinement claim.

### Backward simulation

Backward simulation can be needed with nondeterminism when forward simulation
is too strong or does not compose. State which direction the refinement uses.

### Bisimulation

Require both systems to match each other's observable transitions when exact
behavioral equivalence is the contract. Weak or stuttering variants may hide
named internal transitions, but the hidden alphabet must be explicit.

### Trace equivalence

For deterministic legacy ports, equality of a rich trace over a bounded
workload is often the most practical evidence. It remains bounded conformance,
not a universal bisimulation proof.

## 5. Refinement mappings and auxiliary state

A refinement mapping maps concrete target states to abstract source states.
History variables can remember past abstract information; prophecy variables
can make future-dependent correspondence expressible in difficult concurrent
systems. Their existence is a formal insight, not permission to introduce a
second production authority.

In ordinary migrations, keep proof-only trace/history state separate from
production state and remove or gate it after the relation is established.

## 6. Program dependence and slicing

Program dependence graphs combine control and data dependence. System
dependence graphs extend slicing interprocedurally with call/return context.
Use them to:

- bound all possible influences on a protected observation;
- discover noncontiguous computations that form one behavior unit;
- expose hidden inputs before extraction; and
- prove that a moved computation does not alter unrelated outputs.

Dynamic slicing records only influences for one execution. It is excellent for
counterexample localization and workload-specific contact maps, but cannot
exclude dormant static paths.

The “wedge, split, fold” idea from program tucking is particularly relevant:
gather a semantically related slice, separate it with behavior-preserving
transformations, then fold it into a function/module. Modern tooling still
needs to account for globals, FFI, concurrency, and richer language semantics.

## 7. Partition refinement and quotienting

Begin with blocks distinguished by protected observations. Repeatedly split a
block when its members have observably different successor behavior. The result
is the coarsest stable partition under the selected semantics.

This supplies a principled lower bound on semantic distinctions: architecture
optimization must not merge across those blocks. It does not dictate physical
modules; several distinct blocks may share one owner safely.

## 8. Translation validation

Instead of proving a general refactoring engine, validate each produced
translation:

1. capture the source and target semantic summaries;
2. construct or check a state/step correspondence;
3. compare protected traces/contracts;
4. reject the individual transformation when validation fails.

This is often the right assurance/cost tradeoff for manual legacy migrations.
It mirrors translation validation in compilers: validate the output of a
possibly unverified transformation.

## 9. Proof ladder

Choose evidence proportional to consequence:

1. mechanized proof or model checking for small critical kernels;
2. exhaustive finite-state equivalence where the state space is bounded;
3. translation validation or symbolic relational checking;
4. differential tests over generated/adversarial workloads;
5. discriminating trace/golden tests for known semantic traps;
6. ordinary regression tests; and
7. manual review for residual assumptions.

Higher does not make lower unnecessary when the formal model omits real I/O,
ABI, toolchain, or resource behavior. Combine layers deliberately.

## 10. Reset and transaction proof

Reset and rollback are common sources of false equivalence. Check:

```text
R(initial_source, initial_target)
R(s, t) => R(reset_source(s), reset_target(t))
R(s, t) => observations(step_source(s)) = observations(step_target(t))
failure leaves both systems in related states
committed candidate state is the only new authority
```

If the implementation clones state transactionally, distinguish temporary
candidate state from concurrently writable duplicate authority.

## 11. Identity and allocation proof

Value equality is insufficient when identity is observable. Preserve:

- slot/handle value;
- generation or reuse count;
- allocation/free order;
- linked-index topology when observed;
- stale-handle rejection; and
- mutation ordering around death/reuse.

A new container with the same live objects can still be semantically wrong.

## 12. Randomness proof

Treat randomness as an ordered effect:

```text
(old_rng_state, site, draw_index) -> (value, new_rng_state)
```

Compare draw count, order, consuming site, width, transformation, and failure
paths. Moving a guard before a draw or coalescing two handlers can change all
future behavior even when the immediate output matches.

## 13. ABI and fail-closed proof

At a public boundary test both admitted and rejected cases:

- exact supported version/layout/identity succeeds;
- unknown version, missing fact, stale handle, invalid cursor, wrong generation,
  and unsupported capability fail before forbidden mutation;
- errors are stable and bounded; and
- target language wrappers remain projections, not alternate authority.

## 14. Revision-bound evidence

Each claim should identify:

- source and target revision;
- build configuration and input identity;
- exact command or proof artifact;
- selected workload/test count;
- outcome and exit status; and
- unverified surfaces.

An agent report, stale artifact, zero-test filter, generated golden, or compile
success does not establish preservation.

Bind each candidate's accepted evidence to one structured candidate subject,
the exact required closure, plan and receipt hashes, evaluator and skill
identities, and one exclusively owned strict negative control. Reject orphan
receipts, shared controls, and descriptions that imply source correspondence
when only executable behavior was observed.
