# Contextual boundaries and minimal realizations

## Evidence labels

- **[T]** established theorem in the cited setting.
- **[E]** engineering inference that transfers a theorem's structure under
  additional declared assumptions.
- **[C]** conjecture requiring prospective evaluation.

## Root object

Architecture begins with an open behavior, not a repository graph. Let a
system expose typed boundary variables or ports and admit a set of trajectories
under an environment contract. Protected observations select the part of those
trajectories that a refactor must preserve.

For a proposed decomposition into components `A` and `B` through boundary `Z`:

```text
behavior(S)  ~=  behavior(compose_Z(A, B))
```

The relation may be exact trace equality, bisimulation, refinement, weak
equivalence, or bounded conformance. It must be declared before architecture is
optimized.

Structured cospans and wiring-diagram operads provide formal composition laws
for open systems [T]. A typed temporal hypergraph remains useful evidence, but
without a composition operation it cannot by itself prove that cutting and
recombining a system preserves behavior.

## Exact contextual quotient

For histories on one side of a proposed seam, define:

```text
h ~_(Omega,E) h'
  iff the residual admissible continuation domains after h and h' are equal,
      and every word in that domain produces the same protected Omega
      observation
```

Then `Z* = H / ~` is the coarsest exact deterministic boundary state relative
to those contexts and observations. Under finite deterministic assumptions,
this is the same structural idea as Myhill-Nerode minimization and stable
partition refinement [T]. Any exact deterministic realization must distinguish
the quotient classes; a fixed-width state therefore needs at least
`ceil(log2 |Z*|)` bits. In `kerf`, `Z*` is the joint machine×context-monitor
realization. Its bit count is not a lower bound on machine- or seam-owned state
when the context is external side information.

Important limits:

- The quotient can be infinite.
- General program equivalence is undecidable.
- A quotient relative to one context language need not be valid in a larger
  environment.
- A finite-prefix quotient does not establish liveness, fairness, or arbitrary
  infinite-trace properties.
- Timing, RNG, failure, allocation identity, ABI, and reset matter only when the
  model exposes them as protected state, action, emission, or observation.
- A semantic quotient does not automatically authorize merging physical state
  carriers when ownership, lifetime, ABI identity, placement, or policy forbid
  it.

## Controlled stochastic extension

For stochastic systems, two histories may be equivalent when every admissible
future intervention induces the same distribution over protected exterior
futures:

```text
h ~ h' iff for all admissible interventions a+,
  P(Y+ | h, a+) = P(Y+ | h', a+)
```

This is a controlled extension of causal-state/minimal-predictive-state ideas.
Causal states are unique minimal sufficient predictive representations for the
specified stochastic process under their assumptions [T]. Transferring that
claim to a program requires a distribution, intervention model, observation
horizon, stationarity assumptions where relevant, and an inference procedure.

Do not call ordinary bounded trace equivalence causal. Causal identification
requires interventions or explicit structural assumptions.

## Markov closure and lumpability

A coarse state is sufficient only when its future dynamics close. For a finite
Markov chain, strong lumpability requires every concrete state in one block to
have the same transition probability into every quotient block [T]. For a
deterministic system, stable partition refinement supplies the analogous test.

A practical residual is:

```text
D = information or prediction error about the next protected future
    left in hidden history after conditioning on the proposed port state
```

`D > 0` refutes a memoryless seam. A sampled estimate near zero is bounded
evidence, not proof.

## Why eliminated state returns as protocol

Mori-Zwanzig projection gives an exact reduced equation containing an
instantaneous term, a memory kernel, and an unresolved/orthogonal term [T]. The
software transfer is strong but remains an inference [E]: when an architectural
boundary removes relevant state, the missing degree of freedom tends to return
as callback chains, caches, invalidation, retries, temporal coupling, adapter
state, or unexplained nondeterminism.

Diagnostic:

```text
small declared port + growing coordination machinery
  => search for omitted boundary state before adding another adapter
```

## Linear exact special case

For a linear network Laplacian partitioned into boundary and interior nodes:

```text
L = [L_BB L_BI]
    [L_IB L_II]

L_reduced = L_BB - L_BI (L_II)^-1 L_IB
```

Kron reduction preserves the declared boundary current-voltage relation while
eliminating interior nodes [T]. This is an exact example of observation-
preserving black-boxing. Nonlinear, hysteretic, or stateful internals generally
produce a memory-bearing boundary operator rather than a memoryless Schur
complement.

## Local-to-global consistency

Sheaves encode local views and their restriction maps. A global section is a
collection of local assignments that agree on every overlap [T]. This can model
ABI consumers, cache-validity assumptions, replicated state views, or recovery
contracts [E].

For a cellular sheaf of vector spaces:

```text
consistency energy = ||delta x||^2 = x^T L_F x
```

Here the word energy is literal quadratic mathematics. Zero proves consistency
only with the encoded restriction maps; a consistently wrong model remains
wrong.

## Boundary lower bounds

Keep three distinct floors:

1. **State memory:** contextual quotient class count or causal-state entropy.
2. **Protocol communication:** worst-case communication complexity of the
   protected function across the cut.
3. **Average information:** conditional mutual information under a declared
   workload distribution and codec.

They answer different questions. A million-byte payload reconstructible from a
small seed can have high byte traffic but low semantic information. One rare
authority bit can have tiny volume and infinite correctness importance.

## Falsification patterns

- Merge two quotient classes and search for a shortest distinguishing word.
- Remove one port field and test whether hidden past predicts future output.
- Add an admissible rare context; a seam that disappears was context-overfit.
- Compare local contract satisfaction with existence of a global section.
- Compare full and reduced boundary responses; inject memory, hysteresis, and
  nonlinear cases.
- Approach the same endpoint by two migration histories; different residual
  protocol state refutes endpoint-only analysis.
