# `kerf` v0.1 schemas and evidence contract

## Output contract

All successful analysis commands write one JSON object to stdout:

```json
{
  "schema_version": "kerf.result.v1",
  "tool_version": "0.1.0",
  "operation": "quotient",
  "input_sha256": "...",
  "status": "ok",
  "limits": null,
  "result": {}
}
```

Errors are one typed `kerf.error.v1` object on stderr. Errors discovered after
input hashing retain the operation and input hash; bounded analysis errors also
retain the applied limits. Reaching a configured analysis bound has
`status: "unknown"`. An operation may add deterministic partial coverage under
`error.details`; its absence preserves the ordinary error shape.
Exit codes:

- `0`: valid analysis completed, including an infeasible candidate report;
- `2`: invalid input, unsupported schema, or I/O failure;
- `3`: resource bound reached, result unknown;
- `4`: internal invariant/serialization failure.

IDs and actions are canonically sorted where order affects output. All semantic
observations and emissions are opaque UTF-8 strings. Rates and costs use exact
unsigned integer or rational strings; no binary floating point or tolerance is
part of v0.1.

LTS, factor, recomposition, and morphism semantic maps reject duplicate object
member names. A parser that silently keeps the first or last duplicate could
otherwise make textual declaration order change a verdict.

## Executable self-host corpus

`kerf-selfhost` consumes `kerf.self-host-corpus.v1`, separate from the semantic
kernel inputs:

```json
{
  "schema_version": "kerf.self-host-corpus.v1",
  "cases": [
    {
      "id": "successful-lts",
      "role": "successful_lts",
      "operation": "validate-lts",
      "input": "process.lts.json",
      "expected": {
        "exit_code": 0,
        "stdout_hex": "...",
        "stderr_hex": ""
      }
    }
  ]
}
```

Version 1 requires exactly one case for each of eight roles. Roles fix the
operation, exit class, result stream, envelope status, semantic verdict, and,
where applicable, a non-null witness or factor representation. Input paths are
relative regular files confined to the corpus directory. The frozen protected
observation is the tuple `(exit_code, stdout_bytes, stderr_bytes)`; lowercase
hex is only its lossless JSON encoding. JSON parsing validates each declared
role but does not replace raw-byte comparison.

`kerf.self-host-result.v1` reports the expected and observed Git HEAD, dirty
state, SHA-256 of the executable and corpus manifest, every input hash, the
actual process tuple including Unix raw wait status where available, and exact
mismatch fields. Exit `0` means all tuples matched, `1` is a checked tuple
mismatch, `2` is invalid input or I/O failure, and `4` is an internal failure.
The default rejects a dirty repository. `--allow-dirty` permits development
runs but cannot establish exact-revision evidence.

Exact claim: at the observed repository HEAD, the selected local executable
with the reported hash produced the declared raw process observations on the
fixed corpus in that run.

Nonclaims: reproducible derivation of that executable from the revision;
source-program equivalence; behavior outside the corpus; model, factor,
readout, contract, architecture, or migration inference; sandboxing,
termination, or output bounds; and operating-system or hardware normalization.

## `kerf.lts.v1`

Used by `validate-lts` and `quotient`.

```json
{
  "schema_version": "kerf.lts.v1",
  "machine": {
    "initial": "s0",
    "actions": ["tick"],
    "states": [
      {
        "id": "s0",
        "observation": "ready",
        "transitions": {
          "tick": {"to": "s0", "emission": "done"}
        }
      }
    ]
  },
  "context": {
    "initial": "c0",
    "states": [
      {"id": "c0", "transitions": {"tick": "c0"}}
    ]
  },
  "limits": {
    "max_product_states": 1000000,
    "max_pairs": 1000000,
    "max_witness_length": 100000
  }
}
```

Rules:

- State and action IDs are unique and non-empty.
- The machine is total and deterministic over every declared state/action pair.
- Failure, terminal, disabled-action, and deadlock behavior must be explicit;
  the tool never invents a sink.
- Context is optional, finite, and deterministic. A missing context transition
  prohibits the action. Without a context, every action is permitted forever.
- The context is an action-language monitor. It does not react to outputs.
- Quotienting uses only reachable `(machine_state, context_state)` products and
  reports unreachable machine states separately.
- Residual admissible-action domains are behavior: product states with different
  future action languages remain distinct.
- The stable signature contains current observation and each enabled action's
  emission plus successor class.

Exact claim: observational quotient of this finite deterministic product.

Nonclaims: source-program equivalence; infinite-trace acceptance; liveness;
fairness; timing; or any effect not encoded in the model.

## `kerf.compare.v1`

Used by `distinguish`.

```json
{
  "schema_version": "kerf.compare.v1",
  "left": {"initial": "...", "actions": [], "states": []},
  "right": {"initial": "...", "actions": [], "states": []},
  "context": null,
  "limits": {}
}
```

The machines must declare the same action set. Breadth-first search explores
`(left_state, right_state, context_state)`. A current observation mismatch has
an empty action word. Otherwise each enabled action exposes its transition
emission and successor observation at the same word length. Canonical
breadth-first action-word order yields a shortest word with deterministic
lexicographic tie-breaking.

If a configured pair or witness-depth bound prevents a decision, exit `3`
reports unknown. It never reports equivalent after truncation.

## `kerf.morphism.v1`

Used by `check-morphism` to check one supplied total state function between two
finite deterministic machines. The machines use the same canonical action
alphabet and opaque observation and emission strings.

```json
{
  "schema_version": "kerf.morphism.v1",
  "morphism": {
    "source": {"initial": "s0", "actions": ["tick"], "states": []},
    "target": {"initial": "t0", "actions": ["tick"], "states": []},
    "state_map": [
      {"source_state": "s0", "target_state": "t0"}
    ]
  },
  "scope": {
    "kind": "reachable_under_context",
    "context": null,
    "limits": {"max_product_states": 1000000}
  },
  "claims": ["carrier_surjectivity"],
  "section": {
    "state_map": [
      {"target_state": "t0", "source_state": "s0"}
    ],
    "claims": ["as_morphism", "two_sided_inverse"]
  }
}
```

The alternate scope is exactly `{"kind":"all_states"}`. Both state maps are
structurally total over their declared domains even in reachable mode. Missing,
duplicate, empty, or unknown entries are invalid; maps are never inferred.
Claim names are unique. A missing context means that every action is enabled.
The reachable product bound is positive and applies independently to the base
source-by-context traversal and a requested reverse section-by-context
traversal. Discovering another product state beyond the bound is unknown; exact
closure at the bound succeeds. All-state checking performs no product search
and has no applied limit. Structural validation and complete requested
reachable traversal precede a semantic verdict, so a truncated graph never
emits even an apparently exposed failure as a completed bounded result.

For source machine `S`, target `T`, and supplied `f:S→T`, the base all-state
claim is the pointed coalgebra law:

```text
f(s_initial) = t_initial
observation_T(f(s)) = observation_S(s)
emission_T(f(s), a) = emission_S(s, a)
f(successor_S(s, a)) = successor_T(f(s), a)
```

Reachable scope checks the lifted function `f × id_context` only at reachable
`(source_state, context_state)` pairs and only for context-enabled actions. A
passing reachable result therefore preserves every finite observation and
emission trace in that declared action language. It does not check or make a
claim about listed `unchecked_domain_states` or disabled actions.

Reachable failures use the shortest exposing action word in shortlex order.
At one word, successor observation precedes emission, which precedes transition
commutation. An all-state failure on an unreachable state cannot have a
reachable access trace; its witness instead uses canonical source-state and
action order, sets `word` and `length_actions` to null, and labels its basis
`canonical_all_state_coordinate`. Initial preservation always has the empty
word. A false law is a completed analysis on stdout with exit `0`.

Optional claims do not silently strengthen the selected behavioral scope:

- `carrier_surjectivity` checks the global declared carriers and reports the
  lexicographically least target state outside `image(f)`;
- `exact_isomorphism` is valid only in all-state scope and requires the base
  morphism plus global injectivity and surjectivity. A bijective coalgebra
  morphism has a morphic inverse, but this claim neither infers nor emits it;
- supplying `section.state_map` always checks the global right-inverse law
  `f(g(t))=t`, reporting the least failing target;
- section claim `as_morphism` checks `g:T→S` directionally under the same
  selected scope and, in reachable mode, its own bounded target-by-context
  traversal; and
- section claim `two_sided_inverse` separately adds the global left-inverse law
  `g(f(s))=s`. Its `holds` value requires both inverse laws.

A right inverse proves carrier surjectivity but neither injectivity nor that the
section is a morphism. A global carrier claim may use states left unchecked by
a reachable behavioral theorem; the output keeps those facts separate. The
finite coalgebra basis is summarized in Rutten's *Universal Coalgebra*, listed
in [sources.md](sources.md).

Exact claim: the reported laws and global set properties of the supplied finite
machines and supplied maps, under the named scope, context, and bound.

Nonclaims: relational simulation; partial or context-dependent maps; action
renaming; map or section synthesis; source-program semantics; minimality;
liveness; fairness; timing; causality; or unmodeled effects.

## `kerf.certificate-check.v1`

Used by `verify-certificate` to check a supplied finite proof object without
re-running the algorithm that produced it. The command accepts exactly three
claim kinds:

```json
{
  "schema_version": "kerf.certificate-check.v1",
  "claim": {
    "kind": "compare_equivalence_relation",
    "subject": {
      "schema_version": "kerf.cert.compare-subject.v1",
      "left": {
        "initial": "l0",
        "actions": ["tick"],
        "states": [{
          "id": "l0",
          "observation": "ready",
          "transitions": {"tick": {"to": "l0", "emission": "done"}}
        }]
      },
      "right": {
        "initial": "r0",
        "actions": ["tick"],
        "states": [{
          "id": "r0",
          "observation": "ready",
          "transitions": {"tick": {"to": "r0", "emission": "done"}}
        }]
      },
      "context": null
    },
    "subject_sha256": "...",
    "relation": [
      {"left": "l0", "right": "r0", "context": null}
    ],
    "provenance": null
  },
  "expected_provenance": null
}
```

`compare_equivalence_relation` checks an explicit relation over
`(left_state, right_state, context_state)`. It rejects duplicate, unknown, or
unreachable extra tuples; requires the initial tuple; and explores from that
tuple in sorted action order. At every reached tuple the observations must be
equal. For every action enabled by the shared context, the transition
emissions must be equal and the successor tuple must be present. Differences
behind context-disabled actions make no claim and do not invalidate the
relation.

Here “equivalence relation” means a cross-machine relation that certifies the
two initial behaviors are equivalent. It is not claimed to be a reflexive,
symmetric, and transitive relation on one common state carrier.

`compare_inequivalence_witness` uses the same compare subject and replaces the
relation with:

```json
{
  "word": ["cast", "cast"],
  "mismatch": {
    "kind": "transition_emission",
    "left": "goat",
    "right": "fireball"
  }
}
```

The other mismatch kind is `state_observation`. Replay starts at the two
initial states and the initial context. Every action must be known and enabled;
the claimed mismatch kind and values must occur exactly at the end of the
word. A mismatch at an earlier prefix rejects a padded witness. Replay proves
that word distinguishes the supplied machines; it does not prove the word is
globally shortest.

`lts_trace` uses a `kerf.cert.lts-subject.v1` subject containing one `machine`
and optional `context`. Its proof object records the initial point and every
step:

```json
{
  "initial": {"state": "locked", "context": null, "observation": "locked"},
  "steps": [
    {
      "action": "toggle",
      "emission": "changed",
      "to": {
        "state": "unlocked",
        "context": null,
        "observation": "unlocked"
      }
    }
  ]
}
```

The checker replays the supplied model and requires every recorded state,
context state, observation, emission, and successor to match. This certifies
only that one finite trace.

### Semantic subject identity

`subject_sha256` is exactly 64 lowercase hexadecimal characters encoding
SHA-256 over an injective, declaration-order-independent binary encoding, not
over JSON text. A noncanonical digest is a completed certificate rejection
with `subject_hash_invalid_format`. Version 1 is encoded as follows:

1. ASCII domain prefix `kerf.cert.subject.v1\0`, followed by `pair\0` for a
   compare subject or `single\0` for an LTS subject.
2. Every count and canonical state index is an unsigned 64-bit little-endian
   integer. Every string is its UTF-8 byte length in that format followed by
   the exact bytes.
3. The canonically sorted action vector is encoded once.
4. A machine encodes its state count, canonical initial-state index, then
   states sorted by ID. Each state encodes ID, observation, and, in canonical
   action order, the canonical target-state index and emission.
5. A context begins with byte `0` when absent. A present context begins with
   byte `1`, then encodes its state count, canonical initial index, and states
   sorted by ID. Each state encodes its ID followed by one transition per
   canonical action: byte `0` when disabled or byte `1` plus the canonical
   target index when enabled.
6. A compare subject encodes the left machine and then the right machine. An
   LTS subject encodes its one machine. The context follows the machine data.

The hash excludes JSON formatting and declaration order, and also excludes the
proof object, all provenance fields (including producer limits and revision),
expected provenance, and outer input bytes. Changing a represented state,
observation, emission, action, successor, initial state, or context therefore
changes the semantic subject hash.

Subject schema and model validity are checked first. A well-formed certificate
with a stale subject hash or false local proof returns exit `0` with
`accepted: false` and a typed rejection. An invalid semantic subject returns
the ordinary typed input error on stderr with exit `2`.

Optional provenance has `producer`, `operation`, `input_sha256`,
`tool_version`, optional LTS `limits`, and optional `revision`. Supplying
`expected_provenance` yields a separate `match` or `mismatch` status and exact
mismatching field names. Without an expectation, provenance is `absent` or
`present_unchecked`. Provenance never changes semantic `accepted`.

Exact claims: local closure of the supplied reachable relation; replay of the
supplied distinguishing word; or replay of the supplied finite trace, all
against the hash-bound finite subject.

Nonclaims: signature or authenticity; source-code or source-to-binary
equivalence; producing-revision validity; shortest witness; relation
minimality; context-assisted minimality; liveness; fairness; timing; or any
unmodeled effect.

## `kerf.factor.v1`

Used by `factor` to check a supplied context-relative boundary map.

```json
{
  "schema_version": "kerf.factor.v1",
  "machine": {"initial": "...", "actions": [], "states": []},
  "context": {"initial": "...", "states": []},
  "factor_map": [
    {
      "machine_state": "m0",
      "context_state": "c0",
      "boundary_state": "z0"
    }
  ],
  "limits": {"max_product_states": 1000000}
}
```

The map domain must be exactly the reachable `(machine_state, context_state)`
product. Duplicate, missing, unreachable, unknown, and empty entries are
invalid. Without a context, `context_state` is `null`. The only operation-
specific bound is positive `max_product_states`; reaching it before exploration
closes returns unknown.

For each fibre of the supplied map `q`, the checker requires:

- one protected observation;
- for an action enabled at two fibre members, one emitted value; and
- for that jointly enabled action, one successor `q` state.

Different fibre members may have different enabled-action domains because the
context monitor is retained externally. A lawful result emits a canonical
`kerf.factor-representation.v2`: the certified context monitor, initial
boundary state, observation per state, and a constrained transition for each
action enabled anywhere in the fibre. The context is canonicalized by state ID;
`null` denotes the all-actions unit monitor. An action enabled nowhere in the
reachable fibre is explicitly `unconstrained`; this means the supplied context
provides no reachable evidence for that coordinate, not that an implementation
may ignore the context. Detached v1 representations are rejected by
`recompose`: they lack the monitor needed to prevent a widened context from
combining stepwise constraints that never belonged to one certified run.

An unlawful map is a completed analysis (`status: "ok"`, `lawful: false`), not
invalid input. Its witness names the broken law and a deterministic pair of
shortest access words ordered left then right, plus the offending action for emission or successor
conflicts. Witnesses minimize maximum exposure length, then each access word's
length and lexical order, followed by action, law kind, and boundary ID.

`context_retained: true` is an operation-mode marker. It also applies when the
input omits `context`, because that case uses the synthesized all-actions unit
context.

Exact claim: the reachable finite product behavior does or does not factor
through `(q, id_context)` using the emitted boundary representation and the
supplied output-blind context monitor, relative to the declared protected
observations and emissions.

Nonclaims: minimum boundary cardinality; validity under larger contexts;
source-program equivalence; liveness; fairness; timing; causality; or any
unmodeled effect.

## `kerf.factor-search.v1`

Used by `search-factors` to exhaustively enumerate bounded factor maps over one
reachable finite machine-by-context product.

```json
{
  "schema_version": "kerf.factor-search.v1",
  "machine": {"initial": "...", "actions": [], "states": []},
  "context": {"initial": "...", "states": []},
  "limits": {
    "max_boundary_states": 8,
    "max_product_states": 1000000,
    "max_candidates": 1000000
  }
}
```

All three limits are required and positive. Product exploration uses the same
sorted-action breadth-first order as `factor`; its reachable sources are the
fixed element order for the search. For each boundary cardinality `k` from one
through `min(max_boundary_states, reachable_product_states)`, the search emits
restricted-growth strings in lexical order. Labels are nonnegative, the first
is zero, and each later label is at most one plus the preceding maximum. Keeping
exactly strings with maximum `k - 1` gives every unlabeled `k`-block set
partition exactly once.

Each retained partition consumes one candidate and is decided by the same
factor-law kernel as `factor`. The first lawful map at `k` establishes a
minimum cardinality only because all smaller cardinalities were completely
enumerated and rejected. The successful result contains the canonical source
order, exact counts for completed cardinalities, the successful candidate's
one-based local and global ordinals, its map, and its checked
`kerf.factor-representation.v2`. If every partition through the effective
boundary limit is rejected, the result claims only that bounded negative.

The cumulative candidate budget is checked only when another retained
partition exists. A budget exactly equal to the last candidate of the entire
requested search therefore permits a completed bounded negative, and a budget
exactly equal to a successful candidate permits the minimum. If another
candidate exists after the budget is consumed, exit `3` returns `unknown` with
`kerf.factor-search-coverage.v1` under `error.details`: completed-cardinality
counts, the partial current count, canonical source order, and `minimum: null`.
Product-bound exhaustion is likewise unknown and is never counted as a rejected
partition.

Exact claim: a reported minimum is the least lawful boundary cardinality among
all set partitions of the supplied reachable finite product under the existing
factor laws. A completed bounded negative excludes lawful partitions only up to
the effective boundary limit.

Nonclaims: source-code inference or equivalence; a globally optimal real
architecture; transport, placement, scheduling, Pareto, or migration quality;
probabilistic behavior; or any natural-system objective. Candidate bounds count
retained partition evaluations; generation and checking still scale with the
reachable product size.

## `kerf.recompose.v1`

Used by `recompose` to check the inverse supplied finite direction.

```json
{
  "schema_version": "kerf.recompose.v1",
  "factors": [
    {
      "id": "boundary",
      "representation": {
        "schema_version": "kerf.factor-representation.v2",
        "certified_context": null,
        "initial": "z0",
        "actions": ["tick"],
        "states": []
      }
    }
  ],
  "observation_readout": [
    {"inputs": {"boundary": "ready"}, "output": "ready"}
  ],
  "emission_readouts": {
    "tick": [
      {"inputs": {"boundary": "done"}, "output": "done"}
    ]
  },
  "reference": {"initial": "r0", "actions": ["tick"], "states": []},
  "context": null,
  "limits": {
    "max_product_states": 1000000,
    "max_pairs": 1000000,
    "max_witness_length": 100000
  }
}
```

Factor IDs are unique and canonicalized. Every factor and the total reference
must declare the same external action set. Readout inputs name exactly every
factor ID and contain factor observations or action-specific emissions, never
hidden factor-state IDs. Duplicate signatures are invalid. Extra unreachable
cases receive no claim; every tuple reached through proved transitions must
have exactly one applicable table entry.

The first pass synchronizes the proposed context and every factor's certified
context, then explores only the reachable product. A proposed-context action is
proved only when every certified monitor enables it and every factor coordinate
is constrained. Otherwise the edge is a hole with the shortest canonical word,
including the offending action. The checker never follows, defaults, or applies
an emission readout to a hole. The product-state bound covers factor states,
their certified monitor states, and the proposed context state.

The second pass compares the proved region with the reference using the same
shortest observation/emission witness convention as `distinguish`:

- `equivalent`: the bounded reachable product is complete and equivalent;
- `unproved`: no mismatch was found, but a reachable hole exists; or
- `inequivalent`: a fully proved word mismatches the reference.

Inequivalence is completion-monotone, so an earlier hole never hides a known
mismatch. In that case both `witness` and `unproved` are returned.
An exposed hole is retained even when its word is longer than the configured
witness-search depth. `witness_minimality: "completion_robust"` means product
exploration completed and no earlier canonical hole can hide an earlier
mismatch. `"proved_region_only"` means the reported mismatch is still definite
and shortest in the explored proved region, but an earlier hole or an unexamined
product frontier could contain an earlier mismatch. Pair, product, or
witness-depth exhaustion is unknown (exit `3`), not `unproved`, unless an
observation or emission mismatch is already fully exposed without crossing the
exhausted frontier. Such a definite inequivalence is returned first.
`explored_composed_states` counts the states admitted under the product bound;
it is not a total reachable-state count on an early inequivalence result.

Exact claim: finite-prefix equivalence or inequivalence of the supplied factor
representations and explicit readouts against the supplied reference, over the
proved reachable language admitted by all supplied monitors.

Nonclaims: provenance or lawfulness of fabricated factor representations;
factor or readout inference; source-program equivalence; typed ports; hidden
actions; feedback; delay; hierarchy; asynchrony; liveness; timing; causality;
or unmodeled effects.

## `kerf.migration-graph.v1`

Used by `validate-migration` to check a supplied finite graph of modeled
transformations after local equivalence certificates already exist.

```json
{
  "schema_version": "kerf.migration-graph.v1",
  "carrier_contracts": [
    {"id": "world", "required_context": null}
  ],
  "nodes": [
    {
      "id": "legacy",
      "carriers": [
        {"id": "world", "machine": {"initial": "s0", "actions": [], "states": []}, "writer": "legacy"}
      ],
      "cleanup": null
    },
    {
      "id": "factored",
      "carriers": [
        {"id": "world", "machine": {"initial": "t0", "actions": [], "states": []}, "writer": "rust"}
      ],
      "cleanup": {"obligation": "remove-adapter", "rank": 2}
    }
  ],
  "edges": [
    {
      "id": "migrate-world",
      "source": "legacy",
      "target": "factored",
      "operation": "replace-world-representation",
      "affected_carrier": "world",
      "evidence": {"schema_version": "kerf.certificate-check.v1", "claim": {}, "expected_provenance": null},
      "authority_transfer": {"from_writer": "legacy", "to_writer": "rust"},
      "cleanup_transition": {"kind": "introduce", "obligation": "remove-adapter", "to_rank": 2},
      "reverse_evidence": null
    }
  ]
}
```

The abbreviated machines and certificate above show field placement only; they
must satisfy their complete `Machine` and `kerf.certificate-check.v1` schemas.
All object types reject unknown fields.

Carrier-contract, node, edge, and per-node carrier IDs are non-empty and
unique. At least one contract and node are required; the edge set may be empty.
Every node contains exactly the common carrier-contract universe. Each carrier
contains one valid total deterministic machine and one non-empty writer ID.
For a given carrier, every node uses the same action alphabet. Its optional
`required_context` is validated against that alphabet and is the one context
under which every forward and reverse certificate for that carrier is checked.
Node cleanup is either absent or one non-empty obligation with positive `u64`
rank.

Every edge references known endpoints, has a non-empty operation label, and
names exactly one carrier whose canonical machine encoding differs between the
endpoints. No other carrier machine may change on that edge. Writer authority
may change only for the affected carrier; neither a machine nor a writer may
change outside it. `authority_transfer` is absent when its endpoint writers are
equal and otherwise must equal their exact ordered pair. A writer-only handoff
or exact no-op has an empty machine-change set and is therefore rejected in v1;
it is not laundered through an identity certificate.

`evidence` must have claim kind `compare_equivalence_relation`. Its subject's
left machine must canonically equal the source carrier, its right machine must
canonically equal the target carrier, and its context must canonically equal
the carrier contract's required context. The validator reruns the local
certificate checker and requires `accepted: true`; it never trusts a stored
verdict. A malformed nested certificate schema, subject machine, or context is
a typed input error, just as it is for standalone `verify-certificate`. A
structurally valid but false, stale, or nonaccepting certificate is a completed
graph violation.

Optional `reverse_evidence` has this shape:

```json
{
  "operation": "restore-world-representation",
  "certificate": {"schema_version": "kerf.certificate-check.v1", "claim": {}, "expected_provenance": null},
  "authority_transfer": {"from_writer": "rust", "to_writer": "legacy"}
}
```

The certificate is bound and checked from target to source; the transfer is the
exact reverse endpoint-writer delta. `operation` is required non-empty metadata
but is not checked against executable behavior. This validates only the
supplied reverse proof bundle. It is not an executable rollback and does not
add a graph edge.

`cleanup_transition` must exactly reflect the endpoint node states:

- `{"kind":"neutral"}` for debt-free to debt-free;
- `{"kind":"introduce","obligation":o,"to_rank":r}` for debt-free to
  `(o,r)`;
- `{"kind":"progress","obligation":o,"from_rank":r,"to_rank":r2}`
  for `(o,r)` to `(o,r2)`, requiring `0 < r2 < r`; or
- `{"kind":"discharge","obligation":o,"from_rank":r}` for `(o,r)` to
  debt-free.

Obligation swaps, nondecreasing positive ranks, inaccurate transition claims,
and positive nodes with no outgoing edge are invalid graph verdicts. Because
every continuing positive edge strictly descends in the positive naturals and
every positive node has an outgoing edge, every maximal continuation entering
rank `r` discharges within at most `r` edges; no path can remain positive for
`r` edges. No explicit cycle or path search is required.
Clean-to-positive-to-clean cycles remain valid.

Well-formed but false graph claims return exit `0` with `valid: false` and the
canonical first violation: edges are checked by sorted edge ID, followed by the
least-ID positive sink. Structural ambiguity returns exit `2`. The result has
no resource limit:

```json
{
  "algorithm": "finite_proof_carrying_migration_graph_v1",
  "valid": false,
  "nodes_checked": 2,
  "edges_checked": 1,
  "first_violation": {
    "kind": "edge",
    "id": "migrate-world",
    "carrier": "world",
    "code": "evidence_rejected",
    "details": {}
  },
  "nonclaims": []
}
```

Exact claim: every checked edge changes exactly its named finite carrier,
carries an accepted directionally endpoint- and context-bound equivalence
certificate, and exactly declares its endpoint writer and cleanup transition;
all supplied positive cleanup episodes terminate along every maximal
continuation within their starting rank.

Nonclaims: interacting-carrier composition; whole-system or source-program
equivalence; executable transformations; Git/revision binding; runtime writer
enforcement; rollback availability; carrier creation/removal; partial proof;
path search or path existence; migration-order optimization; Pareto quality;
scalar debt energy; or certificate-producer provenance matching. Producer
provenance remains orthogonal to local semantic certificate acceptance.

## `kerf.candidates.v1`

Used by `validate-candidates` and `pareto`.

```json
{
  "schema_version": "kerf.candidates.v1",
  "elements": ["world", "view"],
  "modules": ["core", "ui"],
  "constraints": {
    "co_locate": [["a", "b"]],
    "separate": [["world", "view"]],
    "pinned": {"world": "core"},
    "authority_groups": [
      {"id": "world-state", "members": ["world"]}
    ]
  },
  "metric_definitions": [
    {"id": "bits", "unit": "bits_per_step", "direction": "minimize"},
    {"id": "headroom", "unit": "bytes_per_second", "direction": "maximize"}
  ],
  "candidates": [
    {
      "id": "candidate-a",
      "assignment": {"world": "core", "view": "ui"},
      "authority": {"world-state": "world"},
      "metrics": {"bits": "128", "headroom": "4096"}
    }
  ]
}
```

Native feasibility checks:

- every element is assigned exactly once to a declared module;
- pins, co-location, and separation;
- one selected member for every authority group; and
- complete exact unsigned-integer metrics with declared units.

The schema deliberately has no `semantic_equivalence: true` escape hatch.
Semantic evidence must come from a separate proof/check and be connected by the
consumer's evidence ledger.

Pareto dominance uses every declared metric. Missing metrics are invalid input,
not zero. Equal vectors remain nondominated. The supplied-set certificate means
only that every supplied candidate was validated and every represented-feasible
candidate was pairwise compared. It does not claim candidate-space
exhaustiveness.

## `kerf.transport.v1`

Used by `score-routes`.

```json
{
  "schema_version": "kerf.transport.v1",
  "workload_id": "step-v1",
  "rate_unit": "bytes_per_second",
  "weighted_hop_traffic_unit": "byte_hops_per_second",
  "resources": ["cpu", "dram"],
  "links": [
    {
      "id": "cpu-dram",
      "from": "cpu",
      "to": "dram",
      "capacity": {"num": "1000000", "den": "1"},
      "latency_ns": "80"
    }
  ],
  "commodities": [
    {
      "id": "read",
      "source": "cpu",
      "sink": "dram",
      "demand": {"num": "64000", "den": "1"},
      "route": ["cpu-dram"]
    }
  ]
}
```

Rules:

- At least one link is required. Links are directed; full duplex requires two
  independently accounted edges.
- Routes are explicit, contiguous, unsplittable, simple directed paths: they
  may not repeat links or revisit resources.
- Equal source and sink require an empty route and add no routed load.
- Multicast must be expanded into explicit sink commodities before input.
- Capacity is positive; demand is nonnegative; rational arithmetic uses
  arbitrary-precision unsigned integers.

Computed values:

```text
load(link) = sum demand of routed commodities
utilization = load / capacity
weighted hop traffic = sum demand * route hops
demand-weighted link latency = sum demand * sum link latency
```

The latency omits queueing, arbitration, overlap, pipelining, and backpressure.
Average route load does not prove queue stability or burst tolerance.
`rate_unit` and `weighted_hop_traffic_unit` are opaque user-declared labels;
v0.1 does not check dimensional consistency. Lower-bound certificates are not
accepted because v0.1 has no mechanically checked lower-bound proof language.

## v0.1 exclusions

The executable does not ship automatic source extraction, causal inference,
probabilistic state reconstruction, hypergraph clustering, architecture search,
routing optimization, queue simulation, cache/NUMA prediction, physical energy,
Rent fitting, passivity checking, or migration-path optimization. These require
separate evidence and certificate checkers rather than unverified flags.
