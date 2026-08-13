# Natural systems: useful operators and hard breaking points

## Rule

A natural analogy earns a place only after it produces a declared variable,
assumption, algorithm, or falsifiable prediction. It never supplies software
evidence by itself.

Evidence labels: `[T]` is a theorem or formal domain result, `[E]` is empirical
or measured domain evidence, and `[C]` is an engineering conjecture. Combined
labels require each named evidence kind.

The molecule → protein → supramolecular assembly → soil → root/fungal network →
tree/ecosystem ladder does not repeat one universal folding or energy law. Each
scale introduces new state variables, environments, dynamics, constraints, and
selection criteria.

Use a sequence of typed coarse-graining maps. At each scale test closure:

```text
coarse(step(microstate)) ~= effective_step(coarse(microstate))
```

If closure fails, retain more state, allow a memory-bearing model, or reject the
scale.

## Analogy ledger

| Natural idea | Established home-domain content | Defensible engineering inference | Breaking point |
|---|---|---|---|
| covalent backbone | fixed chemical connectivity | some order/identity laws are non-negotiable | source code has no universal physical chain |
| protein domain | partly autonomous fold/interaction unit | candidate component with small sufficient boundary | contact density alone does not prove a module |
| folding funnel | many kinetic paths toward viable basins | compare migration paths, traps, and endpoint quality | no software temperature or free-energy landscape |
| frustration | incompatible physical interactions | explicit conflicting constraints or Pareto regret | arbitrary negative “architecture energy” |
| allostery | remote effect mediated by physical structure | nonlocal dependence found by slicing/intervention | unexplained “spooky” coupling |
| chaperone | temporary anti-aggregation folding environment | one-way temporary adapter or isolation surface | adapters do not self-remove and can create authority |
| cotranslational folding | structure forms as sequence emerges | re-evaluate architecture as capabilities arrive | requirements are not amino-acid order |
| self-assembly | type-specific bonds, nucleation, kinetic paths | minimal interaction alphabet and architecture nucleus tests | no universal self-assembly optimizer for programs |
| condensate | dynamic physical colocation and exchange | workload-dependent runtime affinity domain | colocation does not grant state authority |
| soil aggregate | porous multiscale binding and transport | distinguish transient/lifecycle/persistent contracts | modules are not soil particles or pores |
| humic fraction | extraction-dependent heterogeneous organic matter | test seam stability across extraction/evidence methods | no canonical “humic software ball” |
| mycelium/root | adaptive transport with growth/pruning/redundancy | workload backbone and resilience hypotheses | traffic does not prove ownership or altruistic purpose |
| vascular tree | transport/material/resilience tradeoff | hierarchical distribution with justified read redundancy | one tree law is not a universal architecture optimum |
| ecosystem | open multi-currency flow under changing constraints | Pareto budgets and adaptive observations | no one conserved currency or ground state |

## Protein and self-assembly lessons

Protein folding supplies constraint, kinetics, domains, alternative folds,
frustration, allostery, chaperones, and disorder as real biological concepts.
It does not imply one native program architecture. Metamorphic proteins can
adopt distinct functional folds from one sequence. That biological observation
widens the set of possible outcomes; it does not define an architecture
objective (Dishman and Volkman, 2018 review; see the source ledger).

Supramolecular assembly adds useful path warnings:

- uniform strong attraction can create wrong nuclei or kinetic traps;
- heterogeneous interaction types can improve addressable assembly;
- time-dependent protocols can matter;
- identical endpoint contacts can hide different histories; and
- migration order may harden a locally tidy but globally awkward interface.

Forward-test the software transfer by varying type alphabets, migration order,
and the minimal discriminating invariant set. Do not import physical rate laws.

## Dynamic affinity versus authority

Biomolecular condensates can dynamically exchange material with their
surroundings. The software analogue is a conjecture `[C]`: dynamically colocate
hot actors/data when measured split cost exceeds colocated cost plus switching
cost, with hysteresis to prevent thrash (Zhang et al., 2024; see the source
ledger).

The affinity domain does not become a new canonical writer. Runtime placement
and semantic authority remain separate layers.

## Soil and humic guardrails

The older story of persistent universal giant humic macromolecules is contested.
Modern evidence often treats soil organic matter as a heterogeneous continuum
whose persistence depends on minerals, accessibility, microbes, and environment.
Operational extraction fractions can create apparently crisp categories.
The continuum model and its rebuttal remain an active scientific dispute; this
method relies only on the extraction-dependence warning.

Software lesson: run seam discovery independently on static dependence,
runtime flow, writes, ownership, build invalidation, and history. Measure
partition agreement. A module that exists only in one “solvent” remains a
hypothesis.

The glomalin story adds a proxy warning: an operational extraction can correlate
with aggregate stability while containing heterogeneous non-target material.
Imports, coupling scores, co-change, boundary entropy, and LLM clusters can be
useful without being specific measurements of semantic cohesion.

## Roots, fungi, and network evidence

Adaptive fungal/slime-mold networks can balance construction cost, transport,
adaptability, and damage tolerance. Their conductance-reinforcement algorithms
can propose transport backbones. They cannot decide semantic equivalence,
authority, or rare-path correctness.

Never prune a semantically mandatory recovery edge because nominal flow is low.
Evaluate workload and failure ensembles.

Use this evidence ladder:

```text
possible link
< physical/runtime continuity
< measured flow
< recipient state change
< causal performance benefit
< intentional coordination
```

Popular “wood-wide web” and mother-tree claims often jump several rungs. A call
graph or message trace makes the same mistake when treated as purpose or benefit.
Karst, Jones, and Hoeksema's 2023 evidence audit is the source for this caution.

## Flux cones and conserved quantities

Reaction-network geometry suggests a concrete software model. Let a signed
incidence matrix `S` map transition counts/rates `v` into state/resource change:

```text
S v = b
```

Left-null vectors identify conserved quantities; feasible nonnegative fluxes
identify execution modes. Transitions whose feasible fluxes remain in fixed
ratio are candidate flow units. This can outperform call clustering when real
conservation and complete transition counts exist [E/C].

Control of throughput can be distributed. A slow transition need not control
system throughput. Perturb capacities and measure sensitivity rather than
assuming the visible bottleneck owns all control. Schilling et al. support the
flux-cone construction; Kacser and Burns support distributed flux control (see
the source ledger).

## Renormalization and relevance

Use “renormalization” only when the analysis defines:

- a repeatable scale transformation;
- observations whose law is preserved or transforms predictably;
- effective parameters at each scale; and
- ideally a fixed point or universality statement.

Otherwise say hierarchical coarsening.

The transferable test is multiscale relevance: variables that repeatedly
reappear after elimination are relevant to the declared observations. Stable
interface vocabulary across workloads and horizons is evidence for a durable
architecture. It is not proof that software is a field theory.

## Criticality, free energy, BEC, and absolute zero

- A power law alone does not establish criticality. Require an order parameter,
  susceptibility/variance behavior, finite-size scaling, and dynamics.
- Correctness-critical state should generally operate inside a stable region,
  not at an “edge of chaos.”
- Free energy requires a physical thermodynamic ensemble or a precisely defined
  variational model. A weighted architecture score is not Helmholtz free energy.
- Minimum entropy production is restricted to particular near-equilibrium
  regimes; it is not a law of arbitrary living or software systems.
- Absolute zero and Bose-Einstein condensation contribute no ordinary software
  design theorem. A live system needs throughput; its literal ground state is
  halted.
- Landauer concerns physical dissipation of logically irreversible operations,
  not module count or abstraction.

The honest zero point is: no demonstrated feasible neighbor dominates the
candidate, and residual flow is either lower-bound-supported or explicitly
unresolved.

## Adversarial natural-system evals

1. Equal biomass but different age structure must fail coarse closure when
   future dynamics differ.
2. A connected fungal graph without recipient intervention must not be called
   beneficial coordination.
3. A seam appearing under only one extraction layer must be reported unstable.
4. A low-flow recovery path must survive nominal-flow pruning.
5. Two migration histories with the same endpoint graph but different adapter
   debt must not be called equivalent architectures.
6. A power-law plot without finite-size/dynamic evidence must not be labeled
   critical.
7. A biology metaphor must add no score unless translated into independently
   defined measurable features.
