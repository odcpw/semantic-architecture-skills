# Workflow and schema field guide

Use `examples/semantic-toggle.model.json` in this project as the executable
minimal example. Use `examples/toggle.observe.json` for the real-command
observation loop.

## Construction order

1. Add distinct `subjects` for intent, implementation, and each candidate.
   Pin implementation subjects to a revision.
2. Add one or more `scopes` per subject. Context, workload, and horizon are
   claim coordinates, not free-form commentary.
3. Add typed `entities`. A requirement needs modality and acceptance claim IDs.
   A carrier needs the semantic fact it holds and normally an authority. An
   effect should expose durable intent, receipt observation, and reconciliation.
4. Add typed hyper-`relations`. Endpoint kinds are checked. Multiple tails or
   heads mean the relation is joint rather than a bag of pairwise edges.
5. Add `bindings` from artifacts/model symbols to concrete locators and
   predicates. Runtime bindings need an alphabet, open/reset/close protocol,
   and nearby negative controls.
6. Add content-addressed `evidence` with subject, binding, acquisition, source, revision,
   derivation, coverage, and limitations.
7. Add `closures`. Keep closure separate from confidence and from claim verdict.
8. Add scoped `claims`, including support/refutation, conflicts, assumptions,
   nonclaims, and checker receipt when exact.
9. Add `work_items` for implementation/evidence actions. The compiler will add
   unresolved obligations but never apply them externally.
10. Add a `projection` only when a concrete existing Kerf input is available.

## Artifact-bound verification squares

`kerf-model check-coordinates` validates the declaration only. It reports
`ready_for_square_execution`, `execution_bound`, and
`ready_for_correspondence_claims` separately. A model containing a positive
correspondence claim is never execution-bound merely because its command arrays
and hashes are well formed.

Create `kerf.execution-square-receipt.v1` after both paths have produced their
artifacts. Bind the left/right executable bytes, exact argv, every declared
common input, both output sets, the comparator, and the closure. Each artifact
entry contains a semantic coordinate ID, absolute path, and SHA-256. Then run:

```bash
cargo run --quiet --bin kerf-pipeline -- \
  verify-square COORDINATES.json RECEIPT.json
```

The verifier re-hashes files using bounded-memory streaming, rejects argv,
coordinate, comparator, closure, or artifact drift, and emits the first byte
offset for unequal output pairs. Acceptance covers only the named current
artifacts. It does not execute the commands, establish source-to-binary
derivation, or authenticate the machine. A future executed-manifest layer must
close those stronger obligations rather than broadening this result.

## Projection invariant

`scope_entities` defines the local universe. Every member appears exactly once
in either `entity_map` or `omitted_entities`. Every relation incident on that
universe appears exactly once in either `relation_map` or
`omitted_relations`. Each map target is an RFC 6901 JSON pointer that must
resolve in `target_input`. Omission reasons must be non-empty.

Claims and closure IDs attached to a projection must have the same subject and
scope. `target_input` is deserialized by the current strict type for the named
Kerf operation, so a projection cannot emit a merely plausible look-alike.

## Common findings

- `requirement_observation_missing`: no exterior acceptance/refutation signal.
- `requirement_realization_missing`: prose requirement reaches no semantic
  mechanism.
- `requirement_implementation_binding_missing`: an intent realization has no
  implementation relation when an implementation subject exists.
- `carrier_authority_missing` / `fact_authority_conflict` /
  `unauthorized_writer`: ownership of a fact is absent or contradictory.
- `derived_view_validity_missing` / `derived_view_invalidation_missing`:
  cached or materialized state has no coherence rule.
- `effect_recovery_protocol_incomplete`: missing intent, receipt, or
  reconciliation.
- `closure_evidence_missing`: a non-unknown coverage assertion has no evidence.
- `exact_claim_checker_missing` / `checker_verdict_mismatch`: exact language is
  unsupported or contradicts the receipt.
- `exact_claim_checker_identity_mismatch`: an exact receipt names a tool or
  operation different from its bound Kerf projection.
- `heuristic_claim_laundering`: a heuristic/metaphor is used as feasibility
  proof.
- `source_claim_laundering`: finite exactness is promoted to exact source
  correspondence, whether or not the claim is attached to a projection.
- `source_correspondence_subject_evidence_missing`: one side of a claimed
  correspondence has no supporting evidence.

## LLM use

An LLM may draft entities, normalize vocabulary, propose correspondence,
identify candidate conflicts, or turn prose into questions. Assign those
outputs `evidence_inference` or `heuristic`; include the inputs used and
limitations. Promote only after deterministic checks, direct evidence, or a
locally checkable proof receipt. Model disagreement as competing claims rather
than averaging it away.
