# Observation harness

`kerf-observe` turns a declared connection to a real command surface into a
locally checkable finite trace certificate. It is the shared reality pump:
greenfield uses it to test an implementation against a prior model; brownfield
uses it to try to falsify a recovered model. The certificate checker does not
care which came first.

An observation plan (`kerf.observation-plan.v1`) contains:

- a finite `kerf.cert.lts-subject.v1`;
- a binding with an absolute working directory and exact environment;
- required open, reset, initial-probe, and close invocations;
- exact expected exit/stdout/stderr receipts for open, reset, and close;
- one invocation per used model action;
- exact raw exit/stdout/stderr classes mapped to model observations and,
  for action results, emissions;
- at least one nearby negative-control invocation expected to match no class;
- an action word and at least two repetitions.

Each invocation requires an absolute executable path and the expected SHA-256
of its bytes; execution fails if the program changes after validation. The
second hash immediately before spawn narrows but cannot eliminate a path-swap
race on a concurrently writable filesystem. Inherited environment is cleared.
The result hashes the complete plan input, canonical binding,
executables, stdout, and stderr. Raw observations—not merely their decoded
symbols—must be identical across repetitions. Lifecycle receipts must also
match, so a failed reset or close cannot silently pass. A varying result is
`nondeterministic`; an output outside the declared decoder is
`binding_rejected`; a negative control that matches is
`negative_control_failed`. Only then does the harness construct and locally
verify an `lts_trace` certificate, yielding `conformant` or `contradicted`.

Example:

```bash
cargo run --quiet --bin kerf-observe -- examples/toggle.observe.json | jq
```

The example uses harmless `/usr/bin/printf` probes and is bound to the exact
executable hashes on the machine where it was authored; rebind those hashes
after an operating-system package change or on another host. A real plan can
change external state because its commands are real. The harness does not
sandbox, time-limit, cap output, supply authentication, or normalize the
operating system, loader, filesystem, network, or hardware. Review the plan
before running it.

The evidence ceiling is intentionally limited:

- `conformant` means only that the declared plan produced a certificate
  accepted by the declared finite subject under the recorded binding;
- `contradicted` indicts the model-or-binding pair, not necessarily the model;
- the negative control makes one binding mistake observable but cannot prove
  the binding correct;
- repeated runs expose some nondeterminism but cannot prove determinism;
- command output supplies observations and emissions; internal model state is
  reconstructed from the declared transition system rather than observed;
- detaching the trace certificate from the full observation envelope discards
  its reality-contact receipts and leaves only a model-internal replay proof;
- absence from the action word is no claim about other behaviors.

Persist the result envelope or its hash as evidence in the semantic model. Use
an `observed_only` or `complete_for_declared_corpus` closure unless a stronger,
independently justified closure really applies.
