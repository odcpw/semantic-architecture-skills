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
Translate a selected operation into `kerf.metabolism-plan.v1` through the
matching specialist and run `kerf-metabolize`. It mutates only an isolated
candidate, executes bound checks and negative controls, measures every metric,
rejects forbidden regressions, and rebuilds the semantic model before the
campaign continues. See
the metabolism contract below.

## Transformation metabolism

`kerf-metabolize` executes one typed, content-bound materialization plan and
returns a receipt that can be refuted without losing the counterexample:

```bash
kerf-metabolize execute REQUEST.json BATCH.json PLAN.json --require-accepted
```

The request owns purpose, constraints, metrics, and the brownfield subject. The
batch owns the replayable generated candidate and predictions. The metabolism
plan owns exact source bindings, mutations, commands, negative controls,
probes, isolation bounds, and model rebuild.

The workspace root must be new, absolute, and outside the source repository.
The executor creates baseline and candidate copies, rejects links and special
files, clears inherited command environments, invokes no shell, and rechecks
absolute program hashes immediately before spawn. It has no sandbox, timeout,
or output cap, so inspect and authorize the commands. Existing mutation targets
carry preimage hashes and exact replacement counts; created files carry their
content and postimage hash. The post-change revision is computed from ordered
source paths and hashes rather than caller asserted.

Interpreter scripts, model templates, fixtures, and other absolute files read
by a command belong in its `external_files` list. Their hashes are rechecked
with the executable before every spawn; an interpreter hash alone does not bind
the script it executes.

Acceptance requires effective source changes; exact coverage of protected
observations, hard constraints, and metric probes; passing behavior checks and
negative controls; at least one measured improvement; no forbidden regression;
a closed predicted-versus-measured ledger; and a structurally valid rebuilt
model naming the computed revision. Without `--require-accepted`, a completed
rejection remains exit-zero JSON. With it, rejection exits 3. Invalid bindings
exit 2, internal failures 4, and operating-system failures 5.

The receipt is not a publication receipt. Land the accepted mutations through
the target repository's normal Git workflow, then bind the landed revision,
rebuild the model, and generate again under the same envelope.
