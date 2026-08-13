# Fold graph JSON schema

`scripts/fold_graph.py` consumes two JSON documents: an evidence graph and one
architecture candidate. The schema is deliberately small and declarative. It
does not parse source or certify attached evidence.

## Contract

- UTF-8 JSON objects.
- `schema_version` must be `3` in both documents.
- Duplicate JSON object members and unknown fields are invalid.
- IDs are case-sensitive non-empty strings.
- Costs are finite non-negative numbers.
- Probabilities are in `[0,1]`.
- Omitted numeric values default to zero except hyperedge `weight`, which
  defaults to one.
- All objective values are minimized. `objective_rationals` is authoritative;
  `objectives` is a presentation projection.
- Identity canonicalization is type-tagged and exact for JSON numbers. Object
  key order and set-like declaration order (vertices, edges, constraints,
  observations, adapters, commands, artifacts, and nonclaims) do not change an
  input identity; behaviorally ordered arrays are not silently reordered.
- Output is versioned JSON on stdout. Invalid input is typed JSON on stderr
  with exit code `2`. `validate` returns `1` for a valid but infeasible
  candidate.

## Graph document

```json
{
  "schema_version": 3,
  "decision": {
    "id": "resolver-seam",
    "consumer": "project resolution refactor",
    "retirement_condition": "one accepted extraction with no dominating local move",
    "source_identity": {
      "kind": "git_tree",
      "repository": "powerbi-cli",
      "revision": "0123456789abcdef0123456789abcdef01234567",
      "tree_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "working_tree_state": "clean",
      "dirty_patch_sha256": null
    },
    "protected_observations": [
      {
        "id": "resolve-corpus",
        "manifest_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
      }
    ],
    "selection_rule": "pareto_after_hard_feasibility",
    "prior_decision_receipts": []
  },
  "change_model": "independent_bernoulli",
  "module_overhead_bits": 0,
  "semantic_steps": 0,
  "vertices": [
    {"id": "schedule", "kind": "phase", "current_module": "core"},
    {"id": "terrain", "kind": "state", "current_module": "world"}
  ],
  "hyperedges": [],
  "constraints": [],
  "semantic_constraints": [
    {
      "id": "resolver-parity",
      "required": true,
      "observation": "resolve-corpus",
      "evidence_kind": "executed_observation",
      "required_closure": "complete_for_declared_corpus",
      "negative_control_id": "escaping-reference-mutant"
    }
  ],
  "nonclaims": ["the graph is not a complete source-program model"]
}
```

The decision object and non-empty nonclaim list are mandatory. Source identity
is a strict object for a `git_tree` or `content_manifest`; it binds repository,
revision, SHA-256 tree/content manifest, explicit working-tree state, and a
mandatory nullable dirty-patch field. A dirty Git tree requires the patch hash;
a clean Git tree requires null; a content manifest uses `not_applicable` and
null. A branch name is invalid. Every protected observation binds an exact observation
manifest. Every protected observation must have at least one required semantic
constraint or the candidate is infeasible. The only implemented local
selection rule is `pareto_after_hard_feasibility`. Prior decision receipts make
accepted, rejected, or superseded decisions visible and identity-bound; the
consumer owns any reopening policy.

`change_model` is mandatory and currently must be `independent_bernoulli`.
That assumption is part of the meaning of `expected_rebuild_ms`; correlated
change requires a different declared estimator, not silent reuse of this one.

## Vertex

Required fields:

```json
{"id": "rng", "kind": "state"}
```

Optional fields:

| Field | Type/default | Meaning |
|---|---|---|
| `current_module` | string/absent | current physical or ownership assignment |
| `mutable` | bool/false | descriptive marker; edge mutability drives cut score |
| `authority_key` | string/absent | vertices with the same key are writable representations of one semantic fact and must resolve to one representative |
| `replacement_allowed` | bool/true | whether this representation may be replaced |
| `pinned_module` | string/absent | hard module assignment |
| `description_bits` | number/0 | cost to encode this surviving implementation/fact |
| `special_case_bits` | number/0 | subset of description attributable to product/level-specific code |
| `compile_cost_ms` | number/0 | measured compile contribution under the selected build model |
| `change_probability` | number/0 | probability this unit changes in the selected window |
| `memory_bytes` | number/0 | retained bytes for this surviving representation |
| `move_cost` | number/1 when moved | cost of replacing or moving this unit |
| `public_contract` | bool/false | whether replacement/movement changes a public contract surface |
| `contract_churn_cost` | number/1 | cost charged when a public-contract vertex changes |

`description_bits` need not be literal compressed bits, but all candidates must
use the same declared coding scheme. Keep `special_case_bits <=
description_bits` by convention; the evaluator does not enforce it because the
two may be produced by different estimators.

## Directed typed hyperedge

Required:

```json
{
  "id": "step_spell",
  "kind": "read_write",
  "members": ["schedule", "rng", "terrain", "spell"]
}
```

Optional:

| Field | Type/default | Meaning |
|---|---|---|
| `tails`, `heads` | lists/subsets of members | directed endpoints retained in cut receipts |
| `weight` | number/1 | importance under the declared extraction scheme |
| `frequency` | number/0 | occurrences in the representative window |
| `volume` | number/0 | bytes/state cells per occurrence |
| `mutable` | bool/false | cutting incurs `mutable_cut` |
| `ordered` | bool/false | cutting incurs `ordered_cut` |
| `runtime_cost` | number/0 | per-occurrence runtime boundary cost |
| `protocol_bits` | number/0 | MDL cost per boundary crossing |
| `boundary_information_bits` | number/0 | fixed-codec bits observed for this relation over the declared workload window |
| `boundary_memory_bytes` | number/0 | memory introduced per boundary crossing |
| `cut_policy` | `allow` | `forbid` makes any cut infeasible |
| `penalty_mode` | `cut` | `cut` penalizes separated members; `co_location` penalizes co-located members for cochange/edit edges |
| `layers` | list/absent | evidence provenance such as static, dynamic, ABI, history |

Recognized scoring semantics:

- kinds `read_write`, `write`, and `sync`, or `mutable: true`, contribute to
  `mutable_cut` when cut;
- kind `sync` also contributes to `sync_work`;
- `ordered: true` contributes to `ordered_cut`;
- kinds `cochange` and `edit_collision` contribute to `edit_contention`
  according to `penalty_mode`;
- every cut can contribute protocol MDL, runtime, and boundary memory.
- boundary information is summed across cut relations and divided by graph-level
  `semantic_steps`; supplying bits without a positive step count is an error.

Unknown kinds remain valid typed receipts but receive only generic cut costs.
This is intentional: the caller must choose scoring semantics rather than the
tool guessing.

For a hyperedge spanning `s` modules, the evaluator uses `s-1` crossings. This
is a simple transparent estimator, not a network routing theorem.

## Structural constraints

### Co-locate

```json
{"id": "schedule_backbone", "kind": "co_locate", "vertices": ["schedule", "rng"]}
```

### Separate

All listed vertices must occupy distinct modules:

```json
{"id": "trust_split", "kind": "separate", "vertices": ["parser", "secret_store"]}
```

### Pin

```json
{"id": "abi_owner", "kind": "pin", "vertex": "abi", "module": "capi"}
```

### Allowed modules

```json
{"id": "gpu_placement", "kind": "allowed_modules", "vertex": "kernel", "modules": ["gpu", "compute"]}
```

### Forbid replacement

```json
{"id": "identity_survives", "kind": "forbid_replacement", "vertex": "slot_generation"}
```

## Semantic constraints

These describe revision-bound evidence obligations:

```json
{
  "id": "resolver-parity",
  "required": true,
  "observation": "resolve-corpus",
  "evidence_kind": "executed_observation",
  "required_closure": "complete_for_declared_corpus",
  "negative_control_id": "escaping-reference-mutant"
}
```

Each constraint needs an observation declared in the decision, an exact
required closure, and an `evidence_kind`. `executed_observation` requires an
exclusive strict `negative_control_id`; `checked_certificate` forbids that
field and instead requires a proof artifact in its evidence. Supported
closures are `observed_only`, `sampled`,
`complete_for_declared_corpus`, `exhaustive_finite`, and
`sound_overapproximation`.

## Candidate document

```json
{
  "schema_version": 3,
  "id": "resolver-module-extraction",
  "subject_identity": "candidate-executable-or-tree-identity",
  "assignments": {
    "schedule": "core",
    "terrain": "world"
  },
  "replacements": {
    "terrain_copy": "terrain"
  },
  "evidence": {
    "resolver-parity": {
      "semantic_constraint_id": "resolver-parity",
      "subject_identity": "candidate-executable-or-tree-identity",
      "candidate_semantics_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
      "kind": "executed_observation",
      "claim_id": "resolver-parity-claim",
      "closure": "complete_for_declared_corpus",
      "status": "accepted",
      "plan_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "receipt_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
      "evaluator_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
      "skill_sha256": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      "procedure_manifest_sha256": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
      "commands": ["kerf-evidence execute plan.json --require-accepted"],
      "artifacts": ["evidence/package.json"],
      "negative_control": {
        "id": "escaping-reference-mutant",
        "status": "passed",
        "receipt_sha256": "9999999999999999999999999999999999999999999999999999999999999999"
      }
    }
  },
  "adapters": [
    {
      "id": "old_abi_facade",
      "implementation_sha256": "8888888888888888888888888888888888888888888888888888888888888888",
      "cost": 3,
      "removal_condition": "all managed callers use v3"
    }
  ],
  "nonclaims": ["the evidence does not prove behavior outside the declared corpus"]
}
```

### Assignments

Every surviving representative needs exactly one assignment. Replaced vertices
must not retain an assignment; unknown and inactive IDs are errors.

### Replacements

`old -> representative` removes a duplicate representation from the candidate
and rewires declared edges through the representative. Chains are allowed and
resolved transitively; cycles are rejected. The final map is idempotent.

This declaration is not evidence that the replacement is behaviorally valid.
Attach semantic constraints and real translation/conformance evidence.

### Evidence

Each required semantic ID needs one accepted identity-bound claim at the exact
required closure. It binds both the explanatory `subject_identity` and the
derived `candidate_semantics_sha256`, whose projection includes the complete
normalized graph hash, content source, protected observations, semantic
constraints, assignments, replacements, and adapters but excludes evidence
itself. This conservative binding means even an objective-only graph edit
invalidates evidence; in exchange, hand-authored or imported topologies cannot
replay receipts across graph changes. Run `score` once with empty evidence to
obtain this stable hash, then attach receipts.

Executed observations need a command or artifact and one exclusively owned
passing strict negative control. Checked certificates need a proof artifact and
forbid an empirical control. Plan, receipt, evaluator, skill, procedure
manifest, and control receipt hashes are lowercase SHA-256 values. Claim IDs,
claim receipts, control IDs, and control receipts cannot be shared across
semantic constraints; claim and control receipt sets are globally disjoint.
Evidence without a declared owner is invalid.

The evaluator checks this binding shape and includes its own current evaluator,
skill, and normative procedure-manifest hashes in every result. Evidence kinds
are declarations emitted by the upstream evidence authority, not a classifier:
this evaluator does not infer whether an artifact is empirical or formal. It
also does not execute commands, open artifacts, or authenticate supplied
hashes. Use Semantic Architecture's executed-evidence kernel to authenticate
the source state, classify the procedure, produce receipts, and audit the
claimed sentence and closure. A presence-only artifact is not a certificate.

### Adapters

Adapters need a non-empty ID, exact implementation SHA-256, and removal
condition. An optional non-negative `cost` contributes to migration cost. A
missing removal condition makes the candidate infeasible.

## Objective formulas

The evaluator reports:

- `sync_work`: `weight * frequency * max(volume,1) * crossings` for cut sync
  edges;
- `mutable_cut`: the same expression for mutable/write edges;
- `ordered_cut`: `weight * max(frequency,1) * crossings`;
- `boundary_information_bits_per_step`: cut boundary trace bits divided by
  declared semantic steps;
- `mdl_bits`: surviving description + module overhead + cut protocol;
- `special_case_bits`: surviving special-case description;
- `contract_churn`: declared contract cost for moved/replaced public vertices;
- `edit_contention`: declared history/collision edge penalty;
- `runtime_cut_cost`: `runtime_cost * frequency * crossings`;
- `memory_bytes`: surviving memory + cut boundary memory;
- `expected_rebuild_ms`: under `independent_bernoulli`, for each module, sum
  compile cost times `1 - product(1 - change_probability)`; and
- `migration_cost`: moved/replaced vertex and adapter costs.

`duplicate_authority` and `proof_deficit` are reported under `diagnostics` and
as hard violations where nonzero. They never participate in Pareto dominance.
All arithmetic and dominance use exact fractions serialized under
`objective_rationals`; the parallel `objectives` map is display-only.

These are transparent estimators. Use one graph scope and unit system when
comparing candidates. Replace the estimator, not its result, when the domain
needs a more faithful model.

## Commands

### Validate

Checks schema, references, hard constraints, and identity-bound evidence/control
shape:

```bash
python3 scripts/fold_graph.py validate --graph graph.json --candidate candidate.json
```

### Score

Returns feasibility, objective vector, resolved ownership, every cut, and exact
graph/candidate, candidate-semantics, evaluator, skill, and procedure-manifest
identities:

```bash
python3 scripts/fold_graph.py score --graph graph.json --candidate candidate.json
```

### Single-move frontier

Moves one surviving representative between existing modules, rejects
structurally infeasible or baseline-dominated moves, and returns a bounded
Pareto frontier:

```bash
python3 scripts/fold_graph.py moves --graph graph.json --candidate candidate.json --limit 25
```

Generated moves discard baseline evidence and are marked `evidence_status:
pending`; stale evidence is never inherited. The structural frontier is a
conservative superset: any structural dominator must be evidenced before
acceptance. This command does not synthesize replacement, rewiring, adapter, or
new-module moves.

### Evaluator identity

Obtain the exact script, skill-manual, and normative procedure-manifest hashes before binding external
evidence:

```bash
python3 scripts/fold_graph.py identity
```

The same hashes are included in every score, moves, and convergence result. A
same-version code or normative reference edit therefore changes result identity.

### Bounded local Pareto stationarity

Run the complete single-representative/existing-module generator and compute a
canonical state:

```bash
python3 scripts/fold_graph.py converge --graph graph.json --candidate candidate.json
python3 scripts/fold_graph.py converge \
  --graph graph.json \
  --candidate candidate.json \
  --prior-state-sha256 <first-state-sha256>
```

One accepted baseline, no strictly dominating structural move, and full
generator exhaustion return `bounded_single_move_pareto_stationary` on the
first pass. Supplying its unchanged state SHA-256 returns
`bounded_single_move_pareto_stationary_confirmed`; this only confirms
idempotence. A stale prior returns `state_changed`; an available strict
structural improvement returns `open_structural_dominator_pending_evidence`.

This is exact only for moving one active representative among modules already
present in the candidate. It does not exhaust replacements, adapters, new
modules, rewiring, multi-move migration paths, source-model recovery, or the
space of future falsifiers. The command does not interpret the free-text
retirement condition and cannot attest an external discovery sweep or campaign
fixed point. Semantic Architecture owns those claims.
