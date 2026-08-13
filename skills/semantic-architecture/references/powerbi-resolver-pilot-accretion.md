# PowerBI resolver pilot accretion record

Date: 2026-08-13. This record separates what changed in PowerBI CLI from what
the real consumer taught the Semantic Architecture skill and evidence kernel.

## Pinned decision

The target seam was PowerBI CLI project resolution on Linux. The immutable
behavior corpus was committed first at
`bd14c32ec970631a61816a94f927effd05fd02e1`; the physical extraction landed at
`552655854cc3781349a03c83a863eeb406b4ca1e`; and the semantic negative control
landed at `0bae502b179ebc8eea282c2f9d4286dfe7e84b37`.

The declared finite universe contains 22 path-resolution cases: directory and
explicit PBIP discovery; missing and ambiguous inputs; empty, absolute, colon,
and lexical reference failures; report and semantic-model symlink containment;
missing targets; by-connection rejection; and relative-component
normalization. A twenty-third identity is the deliberate escaping-reference
negative control.

The machine-supported bounded claim is only that the exact bound baseline and
candidate test executables satisfy the immutable Linux case fixture and agree
on every declared positive case. The source revisions are identity coordinates,
not supported source↔binary correspondence. This is not trace equivalence,
Windows parity, build provenance, or behavior outside the corpus.

## Project result

`resolve_project` and its private helpers moved from the CLI monolith into the
202-line `project_resolution` module without changing the public CLI surface.
Both pre- and post-extraction executables passed all 22 positive cases. The
candidate-only semantic control deliberately substitutes an escaping report
reference and is rejected. The executed evidence plan contains 45 invocations,
45 validators, one supported bounded claim, one passing strict control, and no
counterexample.

Oracle's final adversarial review invalidated the first stopping result with
five P1 counterexamples. After those repairs, the plan hash is
`806e9fea51885cb651f47d78900fdcbf3ae5e2cdbb81bbd4d4b7bb7f431414e9`
and the evaluator is bound to executable SHA-256
`60af496e8c20cdac3b7c3abb2f7004062a153937de7abc43aa31da6de817e938`
plus embedded skill-manual SHA-256
`d197fd181fea69fa4c7aec0f7934ccc4f2419a01cddc52b0969b0b1a8b73c6a7`.
Two new complete executions produced different raw package hashes but the same
semantic decision projection,
`6a25a1b157d99bed64c26f216eeddd348c699b2e6ad1622c96c846d67a7430e2`,
and canonical state,
`ddae27db4045de38984ac33bad4d52951aafb19d02a01097ebe3914474484f2a`.
The stopping result is `bounded_exhaustive_fixed_point` for this declared
frontier, not universal optimality.

## Necessity witnesses mined into the skill

| Origin observed in the pilot | Reusable mechanism and consumer decision | Discriminating test |
|---|---|---|
| Rechecking a roughly 51 MiB executable at binding and immediately before each of 45 spawns made the first debug run take roughly 3.5 minutes | Optimize only `sha2` in the development profile; retain both substitution checks | The unchanged plan must retain the same validator decisions and decision projection; one subsequent run took 19.133 seconds, an engineering observation rather than a benchmark distribution |
| Equivalent semantic runs had different raw hashes because Rust panic diagnostics contained ephemeral paths/process data | Keep raw package hashes for forensics, but derive fixed-point state from the validator/claim/control decision projection | Different unconsumed stderr bytes must change the raw hash but preserve the decision projection and canonical state |
| An extra invocation could contribute a case identity without contributing evidence to a claim | Reject every execution plan with an invocation that no validator observes or compares | The `unconsumed_invocation_cannot_inflate_finite_coverage` counterexample must fail plan validation |
| Hand-authoring the PowerBI corpus required 45 repetitive invocations and 45 repetitive validators | Add the pure `kerf.parity-corpus-spec.v1` compiler for this exact evidence shape | The compact PowerBI specification must expand to a JSON plan structurally identical to the hand-authored plan |
| The first compiler result was an envelope that `execute` could not consume without scraping `.result` | Make successful expansion envelopes first-class execution inputs | The CLI integration test must expand, persist the unchanged envelope, and execute it with the acceptance gate |
| A same-version evaluator rebuild or skill-manual edit could reuse an old prior-state hash | Bind every decision contract and canonical state to the verified evaluator executable and embedded `SKILL.md` hashes | Changing either hash while retaining version `0.1.0` must reject the contract and prevent closure |
| A validator outside all claims, or an accepted non-mandatory claim, could pad finite coverage | Require every validator to belong to a claim/control and derive coverage only through contract-mandatory decision paths | An orphan validator is invalid; a supported non-mandatory `unearned-case` must not enter `observed_finite_members` |
| Two claims could share one claim-specific control during execution and then fail audit | Enforce bijective control ownership and the invariant `execute Accepted => audit Accepted` | A two-claims/one-control plan must fail validation before execution |
| A candidate-side Git golden could be relabelled `independent_fixture` | Require a distinct fixture subject at a strict ancestor commit in the observed subject's repository | Pointing fixture authority at the candidate subject/commit must fail; the prospective PowerBI baseline must pass |
| A supported free-text statement named source implementations despite absent build attestation | Scope the machine proposition to an exact structured `bound_executable` subject set; keep descriptions non-authoritative | Omitting a validator-reachable executable subject must fail; swapping source coordinates cannot promote the description into source correspondence |
| Documentation called two pre-launch hashes “before and after” | Name the receipt fields and guarantee as binding-time and immediately-pre-spawn checks | Post-spawn path mutation is explicitly outside the guarantee and must not be described as observed |

These mechanisms are retained because each changed a real consumer decision or
removed a demonstrated failure mode. No PowerBI-specific path rule, test name,
or command branch entered the generic compiler.

## Stop and reopen rule

The current fixed point is pinned to the resolver commits, exact evaluator
bytes, 22-case fixture, semantic negative control, Grok/Oracle candidate
generators, Linux execution coordinate, evidence schemas, and present bounds.
Reopen it for a new
decision-relevant counterexample; Windows execution evidence; a second consumer
that breaks an abstraction assumption; source/build attestation; a new
validator kind; or a changed subject, fixture, provider, or decision contract.
Repeating the same unchanged corpus is a hash/no-op confirmation, not more
knowledge.
