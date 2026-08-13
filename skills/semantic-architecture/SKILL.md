---
name: semantic-architecture
description: Construct a new system or recover an existing codebase as one evidence-bearing semantic model; expose contradictions, compile implementation and proof work, and lower bounded questions into Kerf. Use for whole-system architecture, greenfield modeling before code or Beads, brownfield recovery, intent-to-implementation conformance, semantic seams, or coordinating seam topology, behavioral factorization, de-monolithization, and isomorphic simplification.
---

# Semantic Architecture

## Overview

Converge greenfield intent and brownfield evidence on
`kerf.semantic-model.v1`. Make every conclusion name its subject, scope,
evidence, closure, epistemic class, and falsifier before using it to drive code.

The implementation lives in the Semantic Architecture project and depends on
Behavioral Factorization as its exact finite kernel. On Oliver's machine its
canonical checkout is `/home/oliver/Projects/odcpw/semantic-architecture`.

## Route the task

- For a new system, start with intent, protected observations, carriers,
  authority, transitions/effects, contracts, scoped acceptance claims, and
  work. Run `derive-work` before creating tracker items or code.
- For an existing system, pin the revision and collect source, build, test,
  runtime, history, and intervention evidence. State extraction coverage and
  unknowns. Add a hashed binding between model symbols and concrete artifacts,
  invocation/observation predicates, normalization, and reset behavior. Never treat a source graph or LLM interpretation as recovered
  semantics without a binding claim.
- For both, run `analyze`, resolve blockers/errors, then declare an explicit
  finite projection and run `project`. Feed the emitted input to the named
  `kerf` operation.
- After an accepted factor or morphism, use de-monolithization to change
  physical structure and isomorphic simplification to shrink the code. Retain
  the same protected observations and proof receipts.

Read [workflow-and-schema.md](references/workflow-and-schema.md) before creating
or changing a semantic model. Read the repository's
`references/semantic-architecture.md` for theory, boundaries, and integration.

## Operate the compiler

From the canonical repository:

```bash
cargo run --quiet --bin kerf-model -- analyze MODEL.json
cargo run --quiet --bin kerf-model -- derive-work MODEL.json
cargo run --quiet --bin kerf-model -- check-coordinates COORDINATES.json
cargo run --quiet --bin kerf-model -- project MODEL.json PROJECTION_ID
cargo run --quiet --bin kerf-observe -- OBSERVATION_PLAN.json
cargo run --quiet --bin kerf-pipeline -- verify-square COORDINATES.json RECEIPT.json
cargo run --quiet --bin kerf-evidence -- identity
cargo run --quiet --bin kerf-evidence -- snapshot-topology ROOT
cargo run --quiet --bin kerf-evidence -- expand-parity-corpus PARITY_SPEC.json
cargo run --quiet --bin kerf-evidence -- execute EXECUTION_PLAN.json --require-accepted
cargo run --quiet --bin kerf-evidence -- evaluate DECISION_CONTRACT.json PACKAGE.json... --require-fixed-point
```

All success data is versioned JSON on stdout. Errors are typed JSON on stderr.
Do not parse decorative terminal output and do not mutate Beads from inside the
model compiler.

For a projection result, write `result.target_input_json` to an input file
without changing its bytes, invoke `kerf` with `result.operation`, and record
the input/result hashes and status as a checker receipt in the relevant claim.
An accepted receipt proves only the finite target statement. The manifest's
`source_claim_ceiling` limits what can be said about the actual implementation.
Use `kerf-observe` to obtain reality contact: its repeated raw observations,
negative control, and trace certificate can support an observed claim, but not
an exhaustive behavior claim.

For mutation or publication decisions, distinguish an **observer** from an
**assertion boundary**. A checker can execute successfully while returning a
report that rejects the candidate; `Ok(report)`, exit zero from a library call,
or a discarded verdict is not acceptance. Model the caller-owned stage, any
borrowed input, the pre-existing destination, explicit bypass, readback, backup,
and commit point as separate entities/transitions. Challenge the model with an
invalid candidate, a pre-existing-destination sentinel, and a successful
committed-path readback. Acceptance must control publication, rejection must
preserve borrowed and pre-existing artifacts, and returned commands must not
leak a private staging identity.

Observation plans execute their declared programs without a sandbox, timeout,
or output cap. Inspect every invocation and run only commands the operator has
authorized for the target system.

Use `kerf-evidence` when a decision depends on commands actually being run at
the pinned repository, executable, dependency, toolchain, input-topology, and
environment identities. An execution package contains the exact plan and its
manifest together. `evaluate` accepts either that raw package or the complete
JSON envelope emitted by `execute`; do not scrape `.result` into an ad hoc
format. Add `--require-accepted` or `--require-fixed-point` at automation gates;
without those flags, completed negative results remain exit-zero data for
analysis. Read [executed-evidence.md](references/executed-evidence.md) before
authoring a plan or claiming convergence.

When the evidence shape is one pinned baseline, one pinned candidate, a finite
case corpus, immutable expected values, per-case parity, and one semantic
negative control, author `kerf.parity-corpus-spec.v1` and run
`expand-parity-corpus`. The pure compiler emits two invocations and two support
validators per positive case plus the separate control. Its result envelope is
accepted directly by `execute`; do not extract or rewrite `.result`. Use a full
execution plan for other shapes. Compilation reduces repetition but never
raises authority or closure.

The v1 authority ceiling is deliberately hard. A self-produced golden cannot
support an equivalence claim. A second implementation must be a distinct
invocation of a distinct executable. Positive external authority must resolve
to an immutable, hash-checked Git fixture value. For `independent_fixture`, the
fixture subject must be distinct from the observed subject and its commit must
be a strict ancestor in the same repository; relabelling a candidate-produced
golden earns no authority. `source_correspondence` is unconditionally rejected:
source, lock, compiler, and binary pins establish identity coordinates, not a
build derivation proof. A strict negative control must use a validator outside
the support set, be owned by exactly one claim, and pass only when its
deliberately wrong prediction is refuted.

Treat an executed claim's structured `subject` as the proposition coordinate:
v1 supports only `bound_executable` subjects, and the declared subject IDs must
exactly match the invocations reachable through its support validators and
control. The claim `description` is non-authoritative explanatory text. Never
repeat it as a source-level result without a separately supported source↔binary
claim.

When two locally valid paths are claimed to correspond, define a
`kerf.coordinate-model.v1` before asserting parity. Label each compared fact by
semantic kind, unit, chart, effective/observation clock, authority, and identity.
If the tuples differ, require a named directional conversion with a hash-bound
checker. Give the claim a verification square with two executable command
manifests, observations, comparator, closure ceiling, and decisive falsifier.
`check-coordinates` is pure: it can report structural readiness for square
execution but cannot close a positive correspondence. Re-hash the resolved
inputs and outputs with `kerf-pipeline verify-square`; only an accepted result
may promote the named correspondence, and only to the receipt's closure. The
current verifier checks already-produced artifacts and commands but does not
execute them or attest their provenance. A blocker is a useful compiled
obligation; do not erase it by relabeling unlike coordinates as equal.

## Converge instead of asking for ritual reruns

Treat “run once” as one user invocation of an adaptive campaign, not one static
sweep. Within the invocation, retain every counterexample, invalidate dependent
claims, refine only the implicated semantic neighborhood, and rerun affected
checks until the named decision reaches a declared fixed point or a bound
expires. Read [convergent-campaigns.md](references/convergent-campaigns.md)
before designing a whole-repository or repeated audit.

`kerf-evidence evaluate` implements only the stopping predicate. First obtain
the exact evaluator identity with `kerf-evidence identity` and bind it in the
decision contract. A changed evaluator binary or skill manual rejects the old
contract even if the human version string is unchanged. The first complete sweep returns a
canonical state hash and remains `open`. After the caller has run a second
complete discovery sweep over the same declared frontier, place that hash in
`prior_state_sha256` and evaluate again. A fixed point is available only if the
entire normalized decision contract, evaluator identity, evidence packages,
claim receipts, mandatory-decision case identities, providers, bounds, and
nonclaims are unchanged. The tool never infers that the discovery sweep was
complete.

Do not repeat an expensive pass when subject revision, evidence, falsifiers,
providers, bounds, and decision contract are unchanged. A second invocation on
the same canonical state should be a hash/no-op confirmation. Reopen only for a
changed subject, a new decision-relevant falsifier, wider declared closure, or
a prior accepted transformation. Budget exhaustion returns unknowns; it never
certifies a fixed point.

## Maintain the epistemic boundary

Use these classes exactly:

- `theorem_instance`: locally checkable theorem instance with its hypotheses;
- `exact_checked_model`: exact result on a supplied finite model;
- `bounded_check`: result inside declared bounds or corpus;
- `evidence_inference`: scoped empirical or static evidence;
- `heuristic`: candidate generator or ranking device;
- `metaphor`: explanatory analogy only.

Use closure separately: `exhaustive_finite`, `sound_overapproximation`,
`complete_for_declared_corpus`, `observed_only`, `sampled`, or `unknown`.
Absence is not impossibility except where the declared closure actually permits
that inference.

Do not create numeric confidence by blending these classes. Do not present
graph modularity, embeddings, LLM consensus, traces, provenance, tests, or
natural-system analogies as semantic proof.

## Accrete only earned mechanisms

Apply this section to recursive changes to this skill or its tool. Admit an
origin only when it supplies a **necessity witness**:

- an observed case that exposes a missing distinction, falsifies a claim, or
  changes a prospectively named consumer decision; or
- a checkable formal counterexample or theorem instance whose explicit
  hypotheses apply to a named consumer and refute, bound, or compel a current
  claim.

Let analogy, taste, literature resemblance, and model consensus propose
hypotheses only. Require operational mathematics to produce a checker,
certificate, lower bound, counterexample generator, invariant, or measurable
prediction used by a named decision and discriminating falsifier. Typed
terminology alone earns nothing; prompts never count as evidence.

Keep one minimal accretion receipt in prose or the existing project record:
`origin; consumer_decision; mechanism_claim; falsifier; reuse; retirement`.
Name the mechanism's subject, observation projection, scope, and closure;
closure never widens what the observer measures. Predeclare the consumer action
for every possible result. Give executable mechanisms a passing control and a
nearby failing witness. Do not create an accretion schema until two independent
tools must exchange this record.

Derive lifecycle state from the receipt rather than authoring another score:

- `incubating`: the origin is admissible, but first-consumer use or the
  falsifier remains open;
- `admitted`: the first real consumer exercised the decision map and the
  falsifier discriminates;
- `generalized`: the same mechanism passes a qualifying transport witness;
- `removed`: its falsifier fails, retirement fires, no consumer remains, or a
  simpler retained mechanism subsumes it. Keep only a tombstone receipt.

Require a second consumer only for `generalized`. Qualify it with a concrete
assumption-breaking transport witness: name a property consumer one has, the
abstraction does not require, and consumer two lacks. A specialization relying
on that property must be able to pass consumer one and fail consumer two while
the shared mechanism passes both without consumer-named semantic branches.
Bindings and native adapters may differ. Another fixture, dataset, revision,
rename, or deliberately split subsystem is replication, not transport. A
theorem may be mathematically general without making its tooling generalized.

Compress before adding. Keep a field only when a producer and non-documentary
consumer use it and corruption or omission changes a named falsifier; compute
canonical derivatives. Keep a test only when it rejects a named defect or
distinguishes claim states for a named consumer. When a stronger property or
metamorphic check covers retained witnesses at equal or broader closure,
retire weaker duplicates except for an explicitly cheaper sentinel. Prefer
one general check plus the shortest historical counterexample over fixture
families. Ablate math-derived or model-generated machinery when a simpler
retained mechanism makes the same admitted decisions.

Never optimize accretion counts, test counts, pass counts, mutation score, or
model agreement. Do not rescue a refuted mechanism by silently narrowing its
scope; create a revised claim and retain the original counterexample. When one
system proposes both an interpretation and its expected result, require an
independently grounded semantic witness before admission.

Declare `accretion_fixed_point` only for a pinned skill revision, origin
corpus, formal challenges, candidate generators, consumers, and bounds. One
complete mining and subsumption pass must add no non-subsumed candidate, change
no receipt state or consumer decision, and reproduce the same canonical state
hash. This exhausts only that declared frontier. Budget exhaustion is not a
fixed point; a new necessity witness, transport witness, falsifier, consumer,
generator, or bound reopens it.

## Optimize honestly

There is no computable, language-independent “universally smallest program”
for unrestricted real software. Define observations, contexts, candidate
language, bounds, machine/cost model, and deterministic tie-breaking. Then:

1. reject hard semantic and contract violations;
2. ask Kerf for exact finite quotients/factors/morphisms where applicable;
3. search only the declared finite candidate space;
4. preserve cost units and the Pareto frontier;
5. label the result “minimum within this model/bound,” never globally optimal.

## Finish

Before handing off:

- rerun `analyze` and ensure every remaining finding is reported;
- demonstrate deterministic `derive-work` and projection hashes;
- run the projected Kerf checker when the question has an exact finite form;
- retain counterexamples and unknowns rather than weakening the model;
- verify every decision-driving correspondence square against its resolved
  artifacts and retain the minimal mismatch witness;
- execute decision-driving evidence plans with their immutable fixture
  authority and strict negative controls, then retain the full package;
- state whether stopping was a decision fixed point, bounded exhaustive fixed
  point, or budget exhaustion with unknowns;
- state the source revision, closure, assumptions, and remaining risk;
- run repository tests, clippy, formatting, and skill validation after changes.
