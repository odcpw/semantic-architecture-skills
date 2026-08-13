# Convergent campaigns

Use this guide when the user wants one whole-system run, asks whether another
pass is necessary, or wants the strongest result attainable under declared
bounds.

## Contents

1. One invocation, adaptive passes
2. Decision frontier
3. Monotone evidence loop
4. Fixed-point statuses
5. Reopening and information gain
6. Stage packet contract
7. Limits

## One invocation, adaptive passes

Expose one command or agent invocation, but let it contain as many targeted
passes as evidence requires. A static sweep is not a completeness claim. A
campaign alternates discovery, bounded checking, challenge, transformation,
and re-observation until a declared stopping predicate holds.

Pin before discovery:

```text
source/tree/dirty-patch identity
decision, consumer, deliverable, and retirement condition
protected observations and admissible contexts
evidence and corpus manifests
provider versions and finite-search bounds
permitted write roots and external data policy
```

## Decision frontier

Work only on unknown, refuted, or conflicted claims whose outcomes can change
the named deliverable. For each candidate test require:

```text
current claim and nearest competing hypothesis
prospective predicted observation under each hypothesis
executable falsifier and semantic comparator
affected consumer decision for every possible result
```

If no possible result changes the deliverable, do not run the test.

## Monotone evidence loop

1. Discover candidate entities, authorities, histories, and seams at the scale
   implicated by the frontier. Treat provider output as inference or heuristic.
2. Run semantic analysis, coordinate checks, and work derivation.
3. Lower eligible finite questions into Kerf.
4. Challenge distinct behavioral laws: boundary comparisons, state authority,
   history truncation/reseeding, ordering versus set semantics, identity, and
   prerequisite/environment removal.
5. Retain every counterexample. Invalidate all dependent claims before
   refining the implicated neighborhood.
6. Apply only an accepted transformation in an isolated surface; recompute its
   source identity.
7. Rerun exactly the observations, challenges, and squares affected by that
   transformation.

This is counterexample-guided refinement. Call it formal CEGAR only when a
sound abstraction/concretization relation is actually supplied.

## Fixed-point statuses

Compute a canonical state hash over source identity, decision contract,
semantic model, evidence/corpus manifests, claim ledger, coordinate receipts,
counterexamples, Kerf results, candidate frontier, work graph, accepted diff,
and provider versions.

Return `decision_fixed_point` only when:

- the retirement condition is satisfied;
- no blocker affects the decision;
- every decision-driving positive claim has a checker or executable falsifier;
- every mandatory correspondence square has an accepted artifact/execution
  receipt at the required closure;
- all declared finite generators relevant to the decision are exhausted;
- no newly nondominated eligible candidate remains; and
- one complete scheduler sweep changes no canonical state coordinate.

Return `bounded_exhaustive_fixed_point` only when all declared finite universes
and candidate generators were enumerated completely. Return
`budget_exhausted_with_unknowns` when any resource bound fires first. A time or
mutation budget is never a success criterion.

A Program Seam Topology
`bounded_single_move_pareto_stationary` receipt discharges exactly one declared
local generator. It is never, by itself, evidence that the campaign frontier is
exhausted: pending dominating neighbors keep the campaign open, and replacement,
adapter, new-module, rewiring, source-discovery, and multi-move generators remain
open unless the decision contract explicitly excludes them. If the contract
deliberately declares only the single-move universe, any success is bounded to
that universe and must retain that limitation in the result.

## Reopening and information gain

Do not rerun an unchanged deterministic campaign. Reopen when at least one of
these changes:

- subject source/build/environment identity;
- decision contract or protected observations;
- evidence or corpus manifest;
- a decision-relevant falsifier, provider, or finite bound;
- an accepted transformation that changes the subject.

The same revision may merit another campaign when the experiment frontier is
new. The same revision with the same frontier should be a cheap hash/no-op.

## Stage packet contract

Until `kerf.stage-packet.v1` is implemented, every handoff must preserve:

```text
producer/consumer and source identity
decision-contract hash and consumed packet hashes
protected observation IDs
supported/refuted/conflicted/unknown claims with closure
coordinate, square, checker, and counterexample receipts
native artifact schema/hash/locality
open obligations and explicit nonclaims
```

No downstream stage may raise epistemic class or closure without new evidence,
a stronger manifest, or a locally checked result.

## Limits

For arbitrary programs, complete bug finding, unrestricted program
equivalence, universal architecture optimization, and a universally shortest
equivalent implementation are not computable in general and are not even
well-defined without a complete specification and cost model. Exact guarantees
are possible for explicitly finite models, corpora, rewrite grammars, and
candidate sets when enumeration completes. State the universe and closure;
never rename a bounded fixed point “nothing better exists anywhere.”
