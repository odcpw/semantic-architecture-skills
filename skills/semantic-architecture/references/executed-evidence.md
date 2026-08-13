# Executed evidence and convergence

This reference defines the executable trust boundary added after the Dispersion
pilot and before the Power BI project-resolution pilot. It is the operational
answer to two recurring failures: declaring a command without proving it ran,
and declaring “done” because another pass happened to find nothing.

## The small trusted kernel

`kerf.executed-evidence-plan.v1` is the prospective contract. It binds:

- the decision, declared closure, Git commit/tree and current-checkout state;
- executable, dependency/lock, toolchain program and toolchain-output hashes;
- exact command, working directory, cleared environment, stdin, and case ID;
- typed input/output topology using `kerf.fs-topology.v1`;
- validators, validator authority, immutable fixture evidence, claims, and
  strict negative controls; and
- explicit nonclaims.

`execute_plan` validates this contract, rechecks every binding, executes only
the declared programs with an empty inherited environment, captures receipts,
recomputes validators, and derives claim and manifest status. A package is the
plan plus its manifest. `audit_manifest(plan, manifest)` reconstructs internal
consistency from both; a manifest alone has no authority.

Filesystem topology is semantic data, not just a list of file bytes. The
canonical encoding distinguishes the root, regular files, empty directories,
symlinks with raw targets, and dangling symlinks. It never follows links while
walking. A file is read once for its length and content hash so node data and
the topology digest cannot describe different reads.

## Validator authority

The authority order is typed, but labels cannot create authority:

- `self_produced` may report observations but cannot support a positive
  equivalence claim;
- `independent_fixture` requires an exact value in a hash-checked file from a
  distinct subject at a strict earlier commit in the observed subject's Git
  history;
- `trusted_oracle` and `reproducible_build` require a hash-checked external
  receipt, but their labels still cannot establish source correspondence;
- `second_implementation` requires distinct invocations, subjects, and
  executable hashes; and
- no v1 authority may establish `source_correspondence`.

The last rule is intentional. Source identity + lock identity + compiler
identity + executable identity is a coordinate tuple, not a proof that one was
built from the others. A future source-correspondence mechanism needs a real
attestation verifier with an independent trust root, not another enum label.

Every positive claim has one exclusively owned strict negative control. Its
validator must be outside the claim's support set and must make a nearby
deliberately wrong prediction. The control passes only when that prediction is
refuted. A control that agrees with the observed result makes the entire
manifest fail closed. Immediate execution enforces `Accepted => audit Accepted`.

An executed claim is machine-scoped to its structured `bound_executable`
subject IDs, which must exactly equal the executable subjects reachable through
its support and control validators. Its description is explanatory text and is
not a source-level proposition.

## Closure

Empirical executed evidence may use `complete_for_declared_corpus`,
`observed_only`, `sampled`, or `unknown`. It may not promote itself to
`exhaustive_finite` or `sound_overapproximation`; those require an actually
enumerated formal model or sound analysis. In v1, every claim closure equals
the plan closure so a claim cannot borrow a larger scope than the run.

Case coverage is an identity set. The convergence contract lists every member
of a finite universe, and only accepted invocations reachable from its mandatory
claims and controls contribute exact `case_id`s. Counts cannot establish
exhaustion because duplicate or substituted cases can preserve a count while
changing the universe.

## Fixed point

`kerf.decision-contract.v1` names the retirement condition, blockers, mandatory
claim identities `(plan hash, claim ID, required closure)`, eligible tests,
explicit finite members, provider versions, bounds, nonclaims, and the exact
SHA-256 identities of the evaluator executable and embedded skill manual. A
contract authored for different evaluator or skill bytes is rejected before
state evaluation, even when the semantic version string is unchanged.

Evaluation re-audits every package against its exact plan. Only accepted
packages contribute positive claims. Case coverage is narrower: an invocation
contributes only when it is reachable from a contract-mandatory claim's support
validators or its strict control. Validators outside all plan claims/controls
are invalid, and non-mandatory claims cannot pad the mandatory decision's finite
universe. The canonical state hash includes the entire normalized contract
(including nonclaims), the evaluator identity, decision
projections of every audited package, full claim receipts, and observed case
identities. It ignores only `prior_state_sha256` itself. Raw package hashes
remain in the evaluation for forensic identity, but the stopping state uses the
validator/claim/control projection: diagnostic bytes that no validator consumes
cannot prevent semantic convergence. Conversely, a changed stdout, stderr,
exit, or topology value that a validator consumes changes that projection.
Every invocation must be consumed by a validator and every validator must be
owned by a claim or its control. Coverage still comes only from the mandatory
decision subgraph.

A first complete sweep returns `open` and the state hash. After the caller runs
one more complete scheduler/discovery sweep over the same declared frontier,
the caller supplies that hash as `prior_state_sha256`. The evaluator returns a
decision or bounded-exhaustive fixed point only if:

- the retirement condition is met and blockers are empty;
- all mandatory plan-bound claims are supported at the exact required closure;
- no eligible information-bearing test remains;
- generators and any explicit finite universe are exhausted; and
- the entire canonical state is unchanged.

The evaluator cannot observe whether the caller really performed a complete
sweep, so it never claims to be a scheduler. Budget expiry returns
`budget_exhausted_with_unknowns`.

## CLI contract

```bash
kerf-evidence identity
kerf-evidence snapshot-topology ROOT
kerf-evidence expand-parity-corpus SPEC.json
kerf-evidence execute PLAN.json [--require-accepted]
kerf-evidence evaluate CONTRACT.json PACKAGE.json... [--require-fixed-point]
```

`kerf.parity-corpus-spec.v1` is a lossless constructor for the common finite
baseline/candidate shape. For every positive case it emits a baseline and
candidate invocation, an immutable-fixture validator, and a
distinct-implementation parity validator. It also emits one separate candidate
invocation for the strict semantic negative control and one claim consuming all
positive validators. Duplicate cases, missing subjects, identical
baseline/candidate subjects, invalid fixture bindings, and any invalid expanded
plan fail before execution. This is authoring compression, not epistemic
promotion.

The PowerBI resolver corpus is the first real consumer: a compact specification
expanded to a plan structurally identical to the previously hand-authored 45
invocations and 45 validators. The complete expansion result envelope can be
given directly to `execute`, preserving input identity and avoiding ad hoc
`.result` extraction.

`evaluate` accepts a raw package or the complete result envelope emitted by
`execute`; `execute` accepts a raw plan or the complete result envelope emitted
by `expand-parity-corpus`. Default mode returns completed negative semantic results as
structured exit-zero data. Gate flags convert an unsatisfied semantic result
to typed `gate_unsatisfied` on stderr and exit 3.

Exit codes are 0 for completed operation/requested gate satisfied, 2 for
invalid input, 3 for an unsatisfied requested semantic gate, 4 for an internal
invariant failure, and 5 for I/O failure. Stdout is successful data only;
stderr is typed error JSON only.

`identity` returns the exact evaluator executable and embedded `SKILL.md` hashes
required by a decision contract. Rebuilds, code changes, and skill-procedure
changes intentionally invalidate previous fixed-point hashes; obtain a new
identity and run a fresh two-sweep evaluation.

## Explicit limits

- Commands are not sandboxed and have no timeout or output cap. Review plans
  before execution and use only operator-authorized programs.
- Environment clearing does not prevent filesystem, process, device, or
  network effects available to the invoked program.
- A persisted unsigned package is internally auditable, not cryptographic
  proof that physical execution occurred. Hostile or remote provenance needs
  external attestation.
- Toolchain `target`, `profile`, and build-attestation fields are declarations;
  only the bound programs/files and observed hashes are checked.
- A fixed point exhausts its pinned decision frontier and declared universes,
  not all possible bugs, inputs, architectures, or future mechanisms.

Executable bytes are checked once while binding the subject and again
immediately before spawn. This detects substitution between binding and launch,
but it is not a post-execution check and does not eliminate pathname races. The
cost scales with executable size times case count. The development profile
therefore optimizes `sha2`; do not remove that profile override without
benchmarking a large multi-case evidence plan. This is a throughput
optimization only—the number and placement of trust checks are unchanged.

## Design review record

Grok 4.6 independently argued for the same minimal kernel: executed evidence,
typed topology, hard authority ceilings, strict negative controls, and no
campaign platform. Its retained report hash is
`65c7fbbf31da2d0e5e4ae3a5d6bf0943cab2b783d3010cb97ede4d09fb303ae4`.

Oracle's independent implementation review found the initial draft unsafe at
the trust boundary. The implemented corrections made audit plan-aware, bound
contracts to exact plan hashes and closures, rejected source correspondence,
separated strict negative controls from support, derived coverage from explicit
member identities, hashed the whole normalized contract, and hardened Git and
topology checks. These are regression-tested failure modes, not documentation
claims.

Oracle's final PowerBI review then reopened the first fixed point with concrete
counterexamples for unbound evaluator identity, non-decision coverage padding,
non-bijective control ownership, relabelled self-goldens, and source-language
claims over unattested binaries. The revised kernel binds evaluator bytes,
derives mandatory-decision reachability, enforces `Accepted => audit Accepted`,
requires prospective fixture ancestry, and supports only structured
bound-executable subjects. It also corrected the two executable checks from
“pre/post” to binding-time/pre-spawn. The retained review hash is
`b1c798761339c04b69c4c052e509a081f988603854161c80d4b8667c030c8012`.
