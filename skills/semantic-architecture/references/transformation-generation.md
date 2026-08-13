# Whole-system transformation generation

`kerf-generate` turns a bound purpose model and optional source inventory into
proof-obligation-carrying candidates. It removes the need for the operator to
guess a promising seam, but it does not decide that a candidate is correct or
beneficial.

## Request

Author `kerf.transformation-request.v1` with:

- the campaign `search_basis_sha256`;
- `greenfield`, or `brownfield` plus repository root, revision, and source-tree
  identity;
- the same `PurposeEnvelope` used by the campaign;
- capabilities, semantic facts and their writers/readers, protected
  observations, contexts, assumptions, and hard constraints;
- every non-fungible metric with unit, direction, and semantic role;
- allowed generator families, source extensions, planned source root, and hard
  discovery bounds.

While drafting, the envelope digests may be placeholders. Run:

```bash
kerf-generate bind-envelope REQUEST.json
```

Copy the emitted five content identities into the request without changing the
bound content afterward. Generation rejects drift. The campaign search basis
still remains a campaign-controller identity; `bind-envelope` does not invent
or replace it.

## Families and admissible witnesses

| Family | Minimum semantic witness | Required downstream check |
|---|---|---|
| capability construction | declared capability plus protected observation | build and executed observation |
| behavioral factor | shared observation or semantic fact across capabilities | Kerf factor and recomposition |
| authority consolidation | one fact with at least two writers | semantic authority analysis plus executed observations |
| isomorphic simplification | identical normalized multi-line source at distinct files | independent parity corpus and negative control |
| split/fuse/relocate | capability anchors scattered across files or distinct capabilities co-located | seam feasibility plus executed observations |

File length, directory shape, embedding similarity, an LLM judgment, or graph
centrality alone is not an admissible semantic witness. Those signals may rank
already admissible candidates later; they cannot create one.

## Metrics

Candidates must cover every declared metric. A prediction is one of `better`,
`worse`, `unchanged`, or `unknown`, with a causal mechanism and falsifier.
Unknown is preferable to an invented direction. These predictions are not
measurements and do not enter the Pareto ledger until independent evidence
supplies exact values.

Built-in roles are capability coverage, description cost, duplicate authority,
boundary information, runtime cost, and memory cost. `other` preserves a real
project metric without pretending the generic generator knows its direction.

## Coverage and replay

Brownfield discovery walks only declared source extensions, excludes common
generated/vendor roots, rejects symlinks, and enforces file-count, byte, and
depth bounds. It emits a digest of every consumed source file. Skipped files or
candidate truncation make `coverage.complete=false`.

Persist the complete batch and run:

```bash
kerf-generate validate REQUEST.json BATCH.json
```

Validation checks every candidate and replays the generator against the
current request, registry bytes, and source inventory. Any source, purpose,
policy, metric, witness, operation, proof obligation, or nonclaim change
invalidates the batch.

## Next boundary

The generator deliberately emits typed operations rather than source patches.
The apply/verify/rebuild layer must translate an operation through the matching
skill, mutate only an isolated target, execute every proof obligation, measure
every metric, reject regressions or incomparable policy violations, and rebuild
the semantic model before the campaign can continue.
