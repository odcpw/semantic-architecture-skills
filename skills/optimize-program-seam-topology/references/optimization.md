# Optimization and solver strategy

## 1. Problem class

Even ordinary balanced graph partitioning is computationally hard in general.
This problem is harder: directed typed hyperedges, mandatory groups, pinned
owners, forbidden cuts, representative selection, multiple objectives, and a
migration path. Expect exact solutions only for bounded instances or reduced
models.

The goal is not to certify a globally optimal directory tree. It is to:

1. reject semantically infeasible designs;
2. expose materially different feasible candidates;
3. identify nondominated local improvements;
4. quantify sensitivity and uncertainty; and
5. choose an independently verifiable migration step.

## 2. Constraint hierarchy

Use three levels:

### Level A: feasibility

Hard semantic and policy constraints:

- protected trace/refinement conditions;
- one mutable owner per semantic fact;
- fixed order, identity, reset, arithmetic, and failure behavior;
- pinned public/deployment boundaries;
- legal, provenance, and security constraints; and
- explicitly bounded capacity constraints.

No objective can trade against these.

### Level B: lexicographic priorities

Use ordered groups when policy is genuinely hierarchical. Example:

```text
priority 1: eliminate duplicate authority and unbounded synchronization
priority 2: reduce mutable/ordered cuts without ABI regression
priority 3: reduce special-case MDL and contention
priority 4: improve runtime/build cost within accepted bounds
```

Solve one level, constrain its result to an accepted epsilon, then optimize the
next. State epsilons in real units.

### Level C: Pareto tradeoffs

Within a priority level, preserve the nondominated frontier. Do not claim that
one candidate is universally best when it exchanges runtime for build cost or
migration cost for architectural simplicity.

## 3. Why a single weighted sum is usually wrong

A scalar

```text
F = sum_i lambda_i f_i
```

is useful only when:

- every `f_i` is normalized against a meaningful unit or baseline;
- every `lambda_i` has a named decision-maker or empirical derivation;
- important constraints remain outside `F`;
- alternative weights and scale transforms are tested; and
- nonconvex parts of the Pareto frontier are not assumed recoverable.

Otherwise the number disguises policy and measurement error as mathematics.

## 4. Calibrating objectives

Prefer direct physical or workflow units:

| Objective | Example unit |
|---|---|
| synchronization | bytes/tick, fields/transaction, copies/reset |
| mutable cut | cross-owner writes/representative run |
| boundary information | fixed-codec bits/semantic step, by workload |
| ABI churn | consumer surfaces and migration steps |
| contention | conflicting writer-minutes or blocked tasks/month |
| runtime | median/p95 latency, cycles, allocations |
| memory | peak bytes and retained bytes |
| build | expected incremental seconds/change, peak RAM |
| migration | semantic carriers changed, adapters, proof reruns |
| MDL | bits/tokens/AST nodes under a fixed code |

If money is the true common unit, estimate engineering time and compute cost
with ranges, not point fiction. Keep the raw vector beside any economic model.

### 4.1 Empirical weight estimation

When historical observations are sufficient, fit a simple model linking
features to named outcomes such as defects, elapsed implementation time,
runtime cost, or merge delay. Use held-out periods and confidence intervals.
Do not interpret correlation as causal authority.

### 4.2 Preference elicitation

If two candidates remain incomparable, ask for a real tradeoff: for example,
“Would 200 ms less incremental build time justify one extra ABI consumer?”
This is more honest than asking a user to invent abstract coefficients.

## 5. Candidate generation

### 5.1 Preprocessing

1. Resolve proved representative substitutions.
2. Contract mandatory co-location groups.
3. apply pinned and forbidden-owner constraints.
4. Preserve edge types and provenance on contracted hyperedges.
5. Identify articulation-like semantic bottlenecks and high-risk ordered edges.

Do not contract merely high-weight contacts when they have incompatible
lifetimes or ownership semantics.

### 5.2 Multilevel hypergraph method

For medium or large graphs:

1. **Coarsen** compatible vertices connected by strong multi-layer contacts.
2. **Seed** several coarse partitions, including the current architecture.
3. **Uncoarsen** while applying local moves and refinements.
4. **Archive** nondominated feasible candidates at every level.
5. **Repeat** under plausible layer/weight perturbations.

This adapts the multilevel strategy used in hypergraph partitioning. Directed
and typed semantics still require custom feasibility checks and cut receipts.

### 5.3 Hierarchical statistical model

A nested stochastic block model with an MDL criterion can propose the number
and hierarchy of communities rather than requiring `k`. It is valuable for
exploration and noise control, especially across multiple scales.

It does not understand mutable authority, order, or ABI by itself. Treat its
blocks as hypotheses to compare with semantic constraints.

### 5.4 Partition refinement

When vertices denote transition-system states or behavior signatures, compute
the coarsest stable partition compatible with initial observation labels. This
is semantic quotienting, not module clustering. A Paige-Tarjan-style refinement
can efficiently split blocks until equivalent states have equivalent successor
behavior.

Use this before architecture optimization when the graph contains many
behaviorally indistinguishable states or handlers.

## 6. Exact bounded models

Use MILP or CP-SAT when the reduced graph has tens to low hundreds of decision
units and the constraints can be encoded precisely.

Typical binary variables:

```text
x[v,m] = 1 iff representative v is assigned to module m
y[e,m] = 1 iff hyperedge e touches module m
r[v,u] = 1 iff v is represented by u
a[k]   = 1 iff temporary adapter k exists
```

Useful constraints:

```text
sum_m x[v,m] = 1
x[v,m] = 0 for forbidden placements
x[u,m] = x[v,m] for mandatory co-location
sum_m y[e,m] - 1 = number of module crossings for e
representative choices are acyclic and idempotent
one canonical representative per authority group
```

Solve feasibility first. Then enumerate epsilon-constrained or lexicographic
optima. Record solver version, time limit, gap, and whether optimality is proved
or only a feasible incumbent was found.

## 7. Heuristic search

For larger or black-box objectives use one or more:

- greedy or Kernighan-Lin-like local moves;
- simulated annealing to escape shallow local traps;
- tabu search to prevent immediate reversal;
- Pareto local search;
- evolutionary multiobjective search such as NSGA-II/III;
- large-neighborhood search around authority groups; or
- Bayesian/derivative-free search when evaluation requires expensive builds.

Always seed the current architecture. Preserve an archive of feasible
nondominated candidates. Report evaluation budget and random seeds. Repeated
runs and convergence distributions are required before comparing heuristics.

Do not use metaheuristics to search semantic infeasibility unless violations
are needed solely to guide recovery to the feasible set; never present an
infeasible near-optimum as an architecture candidate.

## 8. Incremental optimization

The graph changes as behavior is discovered. Warm-start from the current
partition and impose a migration/change budget:

```text
distance(C_new, C_current) <= B
```

Distance may count moved representatives, altered contracts, adapter lifetime,
or proof reruns. This produces useful local improvements and avoids repeated
global redesign.

Maintain previous cut receipts and invalidate only those whose source evidence
changed. A new level/product should normally extend the graph with descriptor
and reached-behavior evidence; it should not force a full repartition unless it
reveals a genuinely new core contact.

## 9. MDL computation

Choose one stable coding scheme and include all model costs. A practical token
or AST estimator can be:

```text
L = encode(shared implementation)
  + encode(module and interface vocabulary)
  + encode(dispatch/ownership maps)
  + encode(per-product descriptors)
  + encode(special cases and adapters)
  + encode(unexplained exceptions)
```

For graph-only discovery, use a generative graph model and description length
for both partition and edges. A nested SBM is often less biased toward a fixed
community count than modularity maximization.

Report the coder and its stability. Never compare MDL numbers produced by
different tokenizers or schemas as if they were absolute.

## 10. Map equation and dynamic flow

The map equation partitions a network by compressing random-walk flow. It can
be valuable on observed control/runtime flow: transitions frequently visited
together may form a compact dynamical module.

Its random-walk assumptions may not match deterministic phase order, rare
failure paths, or hyperedges. Use it as one dynamic-layer proposal, and retain
protected rare edges separately.

### 10.1 Boundary information rate

For a candidate partition `P`, encode the canonical cross-boundary protocol
trace under one fixed schema/codec and report:

```text
BIR(P) = encoded boundary bits / semantic steps
```

This measures protocol variety and volume without misusing the specialized term
“graph entropy.” Stratify by workload so rare capabilities are not averaged
away. Codec choice, schema, and sample identity are part of the result.

## 11. Information bottleneck

For interface design:

- `X`: detailed state/implementation facts;
- `Y`: consumer observations or future change tasks;
- `Z`: interface or module boundary.

Seek a compact `Z` that retains information about `Y`. Estimate with actual
trace/task distributions only when sample size is credible. Otherwise use the
principle qualitatively: expose exactly the sufficient facts required by the
consumer and proof.

Do not compress distinct source identities merely because current workloads do
not distinguish them.

## 12. Sensitivity and robustness

For each recommended seam:

1. omit each nonsemantic evidence layer in turn;
2. vary disputed measurements over plausible intervals;
3. alter history windows and workload mixes;
4. compare at least two candidate-generation methods;
5. check whether the seam survives coarsening granularity; and
6. report the smallest perturbation that changes the decision.

A robust seam remains feasible and competitive across these variants. A fragile
seam needs more evidence or a reversible experiment.

## 13. Selection rule

Choose a candidate only when:

- it is feasible at the required proof strength;
- no clearly superior candidate dominates it;
- its decisive tradeoff matches explicit policy;
- it remains reasonable under sensitivity analysis;
- its next migration step is small and independently verifiable; and
- the predicted benefit exceeds the measurement and migration cost.

The final human decision is part of the model. Mathematics narrows and explains
the choice; it does not erase product priorities.
