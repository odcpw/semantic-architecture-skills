# OOXML CLI two-move metabolism pilot

Date: 2026-08-14. Bead: `sae-1xs.4`. Held-out target:
`odcpw/ooxml-cli` on Linux. Purpose envelope:
`ooxml-agent-cli-purpose-v1`.

## Result

The pilot passed. Two distinct generated source improvements were accepted and
landed sequentially without operator seam selection or semantic-model
reauthoring between moves. Both are on OOXML CLI `master` and pushed through
commit `2fc45dc`.

| Coordinate | Initial `a3ea067` | After move 1 | After move 2 | Cumulative |
|---|---:|---:|---:|---:|
| Rust normalized nonblank lines | 169,805 | 169,561 | 169,542 | -263 |
| duplicate 8-line cross-file excess | 5,552 | 5,348 | 5,320 | -232 |
| Rust library tests | 182 | 184 | 184 | +2 contract tests |
| range capability JSON SHA-256 | `e9b9d70b…` | identical | identical | no change |
| protected regressions | 0 | 0 | 0 | 0 |

The landed Git diff is +146/-465, net -319 raw lines. Both candidates predicted
improvement on both declared coordinates; both predictions matched independent
measurement. Each isolated run also required the unknown-command negative
control to exit 2 in baseline and candidate.

## Move 1: worksheet-root parser metabolism

- Generated candidate:
  `isomorphic-simplification-086e213611703f27`.
- Witness: a repeated 16-line parser region across eight XLSX modules.
- Semantic review found that the lexical clone concealed two real contracts:
  strict callers rejected a non-worksheet document root, while permissive
  callers continued scanning for a nested worksheet.
- Materialization shared the parser and bounds type in `xlsx_sheet_xml` while
  retaining strict and permissive entry points. Two tests pin that distinction.
- Isolated accepted content revision:
  `6b9949c14b6e482788ccffe819a3ded96afb97058a12cb182b67923c441e5898`.
- Rebuilt model SHA-256:
  `a182163967d0fa3ebf1958edc85f7830c54e77915f811e3cab84554f86293969`.
- Landed commit: `1e4f67c`.

The isomorphic-refactor baseline scores this change 12.5:
`(LOC 5 × confidence 5) / risk 2`. It would also accept the refactor after the
error-contract split was identified. The semantic campaign adds a complete
364-file inventory, content-bound candidate identity, prediction ledger,
negative control, and automatic post-change model/search-basis transition.

## Autonomous rebuild and move 2

`advance_request.py` consumed move 1's accepted receipt and landed Git commit.
It computed the new Git archive identity and search basis using the rebuilt
model hash. No model node, source path, seam, or candidate ID was carried over
by hand. Generation replayed at the new source state: the candidate population
fell from 1,443 to 1,396, demonstrating that the first materialized change
altered the next search surface.

- Generated candidate:
  `isomorphic-simplification-8575575e72049b32`.
- Witness: three exact guarded implementations of direct worksheet-child
  lookup—the isomorphic skill's Rule of 3.
- Materialization moved the unchanged short-circuit, child enumeration, first
  match, and error propagation into `xlsx_sheet_xml`.
- Isolated accepted content revision:
  `a737c85d9b4c4801a2a65610ebff566ce73549a02e180912cf48a439a7df01b8`.
- Rebuilt model SHA-256:
  `dab8f0ae54ae5e80fcb94e8b7d2893274b4e578c11f7878cb052402f6054810f`.
- Landed commit: `2fc45dc`.

The isomorphic baseline scores move 2 at 15:
`(LOC 3 × confidence 5) / risk 1`. Here both systems select the same simple
lever. The metabolism system additionally establishes that it remained useful
after the first change instead of relying on a one-pass candidate map.

## Evidence and falsifiers

Each accepted receipt binds the request, complete generated batch, exact
materialization plan, source preimages, `/usr/bin/cargo`, `/usr/bin/python3`,
and the external probe script. Baseline and candidate were fresh copies. The
acceptance gate ran:

1. all Rust library tests;
2. exact stdout parity for `--json capabilities --for range`;
3. a deliberately invalid command that must exit 2;
4. both declared metric probes; and
5. a source-derived semantic-model rebuild naming the computed content
   revision.

The broader target gate after both commits passed `cargo fmt --check`,
`cargo test --all-targets --locked`, strict all-target Clippy, and focused UBS
with no critical findings.

Two early executions were correctly rejected because the isolation manifest
omitted transitive compile-time inputs: first `tests/support`, then `testdata`
goldens. Both baseline and candidate failed. The builder now binds and copies
all files under `src`, `tests`, and `testdata`. These are harness
counterexamples, not candidate regressions, and remain recorded here rather
than being misreported as successful moves.

## Comparison and limits

Ordinary expert inspection can find these particular helpers with `rg`, and
the isomorphic skill supplies the right local proof discipline. It does not by
itself construct a complete content-bound search state, close prediction versus
measurement, rebuild the model, or decide that a second pass is required after
the source changes. Semantic Architecture supplies that global continuity;
the isomorphic skill supplies the conservative source transformation contract.
They are complementary.

This is a two-move metabolism result, not a universal optimum and not proof of
desktop Office compatibility. The rebuilt model binds the whole Rust source
inventory and revision but does not claim that source scanning recovers every
domain semantic. Complete generator coverage is relative to the declared
16-line clone grammar, two metrics, environment, and purpose envelope.

## Accreted skill lessons

- Interpreter identity was insufficient: scripts and external model/probe
  inputs are now first-class hash-bound command inputs.
- Isolation must close over transitive compile-time fixtures, not just source
  extensions. The pilot builder now discovers and binds the target's real test
  and golden inputs.
- A normalized window can cross a semantic boundary or hide variant error
  contracts. Generator output remains heuristic; future candidate reduction
  should coalesce overlapping windows into syntax-enclosed maximal clone regions
  and describe observed variants before ranking.
- The two-run result falsifies the idea that one static source scan is enough.
  The correct unit is one campaign invocation containing accepted move,
  rebuild, regenerated search, and another independently gated move.
