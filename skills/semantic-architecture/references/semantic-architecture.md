# Semantic Architecture Compiler

## Thesis

The useful generalization is not a universal optimizer. It is a compiler from
an evidence-bearing description of a system into smaller proof problems and
ordered engineering obligations.

Both greenfield construction and brownfield recovery converge on the same
canonical object, `kerf.semantic-model.v1`. They remain different epistemic
operations:

- greenfield declarations constrain what may be built;
- brownfield observations constrain what may be claimed about what exists;
- neither is silently substituted for the other;
- conformance is an explicit relation between intent and implementation;
- exactness belongs to a named finite projection, not automatically to source.

This is the accretive bridge between seam discovery, Behavioral Factorization,
de-monolithization, and isomorphic simplification. Upstream tools may propose
entities and relations. `kerf-model` makes their uncertainty explicit and
compiles bounded proof targets. Kerf checks those targets. Downstream skills
apply only transformations justified by accepted claims and retain the
manifest as their behavioral contract.

For the independent review record and roadmap rationale, read
[design-synthesis.md](design-synthesis.md). For real command capture, read
[observation-harness.md](observation-harness.md).

## Canonical model

The model has ten layers:

1. **Subjects** — intent, implementation, or candidate, with revision/build
   identity where applicable.
2. **Scopes** — contexts, workloads, and horizon under which a claim is meant.
3. **Semantic entities** — requirements, carriers, authority protocols,
   derived views, actions/transitions, observations, effects, ports, contracts,
   actors, implementations, invariants, and resources.
4. **Typed open relations** — directed hyperedges such as `realized_by`,
   `writes`, `emits`, `implements`, `corresponds_to`, `decomposes`, and
   `contends_on`. Multiple tails and heads retain joint dependencies that an
   ordinary call graph would erase.
5. **Bindings** — a hashed connection from a source artifact or model alphabet
   to concrete locators, invocation/observation predicates, normalization,
   session/reset behavior, and negative controls. This is the reality bridge;
   hashes without a binding do not establish what a symbol means.
6. **Evidence** — subject, binding, content hash, acquisition method, source, revision,
   derivation, coverage, and limitations.
7. **Closure claims** — the indispensable statement of what the evidence or
   model purports to cover.
8. **Claims** — subject, scope, proposition kind, verdict, assumptions,
   conflicts, support/refutation, epistemic class, and optional checker receipt.
9. **Work** — changes, satisfied requirements, dependencies, evidence gates,
   cleanup, and an optional external tracker binding.
10. **Projections** — a total map-or-omit accounting from a semantic scope to an
   existing Kerf input, plus its claim/evidence/closure manifest.

Closure kinds are:

| Kind | What absence may mean |
|---|---|
| `exhaustive_finite` | Absent from the declared finite universe |
| `sound_overapproximation` | Safely absent from the analyzed abstraction, under its hypotheses |
| `complete_for_declared_corpus` | Absent from one enumerated corpus only |
| `observed_only` | Not observed in the recorded coverage |
| `sampled` | Not present in the sample |
| `unknown` | Nothing |

The implementation never converts the bottom four rows into impossibility.

Claim classes are deliberately ordinal, not numeric confidence:

| Class | Permitted interpretation |
|---|---|
| `theorem_instance` | A locally checkable theorem instance with hypotheses |
| `exact_checked_model` | Exact for the supplied finite model |
| `bounded_check` | Established only inside declared bounds |
| `evidence_inference` | Supported by scoped evidence, open to counterevidence |
| `heuristic` | Candidate generator or prioritizer |
| `metaphor` | Explanatory analogy only |

Heuristics and metaphors may propose candidates. They cannot positively
establish requirement satisfaction, equivalence, factorization, isomorphism,
invariance, or source correspondence.

Every claim also receives a separate reality-contact coordinate: `none`,
`declared_artifact`, `observed`, or `intervened`. Formal strength and reality
contact are never collapsed into a score. A theorem about a toy model can be
formally stronger yet less grounded than an intervention on the deployed
system; the pair makes that visible without pretending the axes are fungible.

## Dual workflow

### Greenfield: specification to build graph

1. Create an `intent` subject and make each `must` requirement externally
   observable.
2. Attach each requirement to carriers, transitions, effects, ports,
   contracts, or invariants with `realized_by`.
3. Declare one authority protocol per semantic fact, including all writers.
4. Make failure, persistence, reset, identity, ordering, and effect recovery
   explicit rather than leaving them in prose.
5. Add scoped acceptance claims. Leave their verdict `unknown` until evidence
   or a checker exists.
6. Declare implementation work and evidence gates.
7. Run `kerf-model derive-work`. Missing model and proof obligations appear
   before Beads or code are created; downstream application to a tracker remains
   a separate, reviewable action.
8. Once a finite behavioral projection is explicit, run `kerf-model project`,
   execute the named `kerf` operation, and bind its receipt to the claim.

This does not find “the universally smallest program.” For unrestricted
programs that objective is neither generally computable nor independent of
the language, machine, observations, and cost code. It can find an exact
minimum within an explicit finite candidate language or factor-search bound,
then preserve a Pareto vector for costs that have different units.

### Brownfield: repository to recovered model

1. Pin repository, build, and runtime subjects to exact revisions.
2. Declare a hashed binding for the source and, for runtime models, the action,
   observation, and emission alphabet plus open/reset/close protocol and nearby
   negative controls.
3. Extract source/build facts: definitions, calls, imports, schemas, ownership,
   storage, writers, invalidation, effects, tests, and build edges.
4. Add runtime and intervention evidence separately. Passive traces show what
   occurred, not what is possible; active interventions are stronger but still
   scoped.
5. Bind implementation coordinates to intended semantic entities with
   `implements` and state disagreements as claims, not overwritten fields.
6. Declare extraction closure honestly: normally `observed_only`, `sampled`,
   `complete_for_declared_corpus`, or `sound_overapproximation` when a static
   analysis really supplies that guarantee.
7. Run `analyze`; repair contradictions and missing authority, observation,
   invalidation, effect-recovery, and implementation bindings.
8. Project bounded behavioral questions into Kerf. Counterexamples refine the
   model or evidence scope. Call this CEGAR only when a sound abstraction and
   concretization relation have actually been supplied.
9. Feed accepted seams and their contracts to de-monolithization and
   isomorphic simplification. Re-run the same protected observations after each
   transformation.

Automated source adapters are evidence producers, not semantic oracles. A
language adapter should emit a proposed subject plus source hashes, coverage,
and explicit unknowns. An LLM may normalize names, connect prose to likely
entities, propose conflicts, and draft candidate relations; every such output
starts as inference or heuristic and must survive deterministic validation.

## Analyzer and work compiler

`kerf-model analyze` strictly validates references, relation endpoint types,
authority writer cardinality, source revisions, derived-view validity and
invalidation, effect recovery, closure evidence, implementation bindings,
claim/checker agreement, contradiction overlap, work cycles, projection
coverage, and projection JSON pointers.

Evidence hashes and checker receipts are declarations at this layer. `analyze`
does not fetch their external artifacts or re-execute the named checker. A
consumer that needs that stronger statement must perform the operation, bind
the resulting bytes, and retain the execution environment separately.

`derive-work` deterministically turns non-informational findings into typed
obligations. Requirement observation precedes realization and acceptance;
those precede implementation work. Declared work also depends on obligations
touching its changed entities, requirements, or evidence gates. The result is
versioned JSON and never mutates an external tracker.

`project` re-validates an embedded input using the exact existing Kerf schema.
Its manifest binds:

```text
semantic model hash
subject + scope
lowering version
target operation + schema + canonical input hash
mapped and explicitly omitted coordinates
claims + evidence + closures + assumptions
source-claim ceiling
```

The strongest honest composite statement is:

> Model M and projection P deterministically produced finite input K; Kerf
> checked property Q of K; evidence V with closure Γ supports the stated,
> separately bounded correspondence between K and source revision r.

## Mathematical mechanisms and guardrails

The compiler uses ideas from several fields only where their hypotheses can be
made executable:

- **Coalgebra and automata:** finite observational behavior, equivalence,
  morphisms, quotienting, and shortest distinguishing continuations.
- **Typed open hypergraphs/cospans:** compositional incidence and ports. This
  proves structural gluing only when backed by a semantic functor/checker; a
  pretty graph is not behavioral evidence.
- **Sheaf-style local-to-global consistency:** useful for checking agreement of
  contracts on overlaps. It does not synthesize missing local truth.
- **Petri nets/event structures:** appropriate when true concurrency and
  partial order are explicit; interleaving reduction requires a checked
  independence relation.
- **Abstract interpretation and CEGAR:** sound over-approximation can support
  absence claims; iterative refinement earns the CEGAR name only with a sound
  abstraction relation.
- **CEGIS/SyGuS:** exact minimum synthesis is meaningful inside a fixed grammar,
  bound, specification, and deterministic tie-breaker.
- **Information bottleneck/MDL:** useful for ranking only after declaring input,
  protected output, probability model, and coder. They do not supply a
  language-independent universal shortest program.
- **Control and causal inference:** interventions, invariants, and feedback can
  discriminate models. Correlation, provenance, and call edges are not causal
  identification.
- **Biology, chemistry, physics, language, and neuroscience:** bow-ties,
  homeostasis, retroactivity, coarse-graining, compositional representation,
  and multiscale dynamics may generate hypotheses only. They earn operational
  status only under the skill's accretion rule: checkable hypotheses, a named
  consumer decision, and a checker, certificate, lower bound, counterexample
  generator, invariant, or measurable prediction with a discriminating
  falsifier. Typed variables and terminology alone do not promote them.

An anti-ornamentality check applies: deleting every natural-system annotation
must leave feasibility, emitted Kerf inputs, exact results, Pareto membership,
and migration validity unchanged unless an annotation generated a concrete
claim with a measurable falsifier.

## Relationship to the existing skills

| Skill/tool | Role |
|---|---|
| `semantic-architecture` / `kerf-model` | Canonical intent/recovery model, epistemic ledger, obligations, projections |
| `optimize-program-seam-topology` | Repository discovery and proposed seam topology |
| `behavioral-factorization` / `kerf` | Exact finite behavioral checks and bounded minima |
| `de-monolithize-your-codebase-isomorphically` | Physical decomposition at accepted seams |
| `simplify-and-refactor-code-isomorphically` | Local/global code simplification under protected observations |

The accretion rule is simple: each stage consumes the previous stage's
machine-readable evidence and returns new evidence or a counterexample. No
stage rephrases an unproved assertion as a stronger fact.

## What is intentionally absent

- no architecture “energy” or hidden scalar objective;
- no universal optimality claim;
- no graph partitioner presented as semantic decomposition;
- no passive trace learner presented as a complete behavior model;
- no confidence percentage that mixes theorem, coverage, and taste;
- no automatic Beads mutation;
- no database or separate service before the JSON contract needs one;
- no LLM output admitted as proof.
