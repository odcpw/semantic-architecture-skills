# Living optimization cycle

Use this reference to author or consume `kerf.optimization-campaign.v1`.

## What the state means

The campaign is one closed-loop search state, not a report about a pass. It has
one of two initial subject bindings:

- `greenfield`: an intent model exists before an implementation does;
- `brownfield`: a repository revision and source-tree digest already exist.

Everything after that binding is shared. Both modes use the same purpose and
environment envelope, semantic model, generator obligations, metric vector,
candidate evidence, Pareto calculation, resource accounting, and stopping
logic. Once a greenfield implementation becomes current, the next state is
ordinary brownfield improvement over the produced artifact.

## Identities

Run:

```bash
cargo run --quiet --bin kerf-cycle -- identity
cargo run --quiet --bin kerf-cycle -- hashes CAMPAIGN.json
```

Bind the returned evaluator identity into the campaign. `hashes` emits:

- `search_basis_sha256`: evaluator, subject, envelope, semantic model,
  generator versions/scopes, metric definitions, current candidate, and any
  finite universe. Generated candidates and every sweep consume this identity.
- `decision_state_sha256`: the search basis plus the candidate/evidence ledger,
  blockers, and unknowns. A quiescent sweep consumes this identity.

The full evaluation additionally emits `canonical_state_sha256`, which includes
sweep history, budget use, parent-state identity, and nonclaims. Vector order
does not change the canonical identities of set-like fields.

Any producer must recompute hashes after editing a bound coordinate. Never
copy hashes from an earlier subject, model, envelope, generator version, or
candidate ledger merely to make a receipt match.

## Generator obligations and sweeps

Each obligation is the tuple:

```text
generator_id
generator_version_sha256
scope_id
scope_sha256
```

A sweep is complete only when it contains exactly one `complete` receipt for
every current tuple. `blocked` receipts preserve the reason but do not count as
coverage. A complete receipt lists newly produced candidate IDs and any
explicit finite members examined. Candidate matching is centralized: each
reported candidate must exist, name the producing sweep, and consume the same
search basis.

A receipt digest binds prospective execution evidence. The v1 campaign
evaluator validates identity and state logic; it does not yet attest that the
generator physically ran. The apply/verify stage must supply that provenance
before a candidate may be marked `verified`.

## Metrics and frontier

Every verified current-basis candidate supplies exactly one unsigned integer
value for every declared metric. Directions are independent `minimize` or
`maximize` coordinates. The evaluator calls Kerf's exact product-order kernel:

- a candidate dominates another only when it is no worse on every coordinate
  and strictly better on at least one;
- incomparable candidates remain on the frontier;
- no weighted sum, confidence blend, or universal elegance score is invented.

The current implementation must itself remain on the represented frontier.
If a verified candidate dominates it, the campaign stays `open` so the apply
loop can act.

Metric values are still claims until their evidence digest is independently
validated. Add ratios, signed quantities, uncertainty intervals, and
distributional comparisons only with a real consumer and exact comparison
semantics; do not encode them by arbitrary scaling.

## Computed statuses

- `open`: coverage, verification, current implementation, or stability work
  remains.
- `budget_exhausted_with_unknowns`: `consumed >= limit` before a terminal
  predicate holds. This is never success.
- `envelope_stationary`: the current candidate is the sole represented
  nondominated candidate and the last two current-decision sweeps are complete
  and quiescent.
- `frontier_set`: the same two-sweep predicate holds, but multiple verified
  incomparable candidates remain.
- `bounded_exhaustive_fixed_point`: one complete sweep exactly covers an
  explicitly declared finite universe and the current candidate is on the
  exact represented frontier with no blockers, unknowns, or pending evidence.

These statuses never mean “universally optimal.” They are relative to the
bound envelope, semantic model, metrics, generator grammar, scopes, evaluator,
and evidence. A new environment, falsifier, generator, metric, or implementation
identity starts a new search basis.

## Minimal lifecycle

1. Author the purpose/environment envelope and semantic model.
2. Declare versioned generator/scope obligations and non-fungible metrics.
3. Bind the current implementation candidate when one exists.
4. Run `hashes`; execute generators against that search basis.
5. Add candidate and sweep receipts. Verify candidate evidence independently.
6. Apply an accepted candidate, rebuild the semantic model, and create the next
   campaign state with a new subject/search identity.
7. When no accepted change remains, run complete quiescent confirmation sweeps
   until the computed status is terminal or the budget ends with unknowns.

The apply/verify/rebuild controller is the next implementation layer. Until it
exists, `kerf-cycle` is an executable state and stopping kernel, not an
autonomous optimizer.
