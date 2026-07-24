<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_place`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-place.loft`](tests/01-hex-place.loft); read that file for the exact, compiling form. This page is the map.

## What it does

A body carries a **pose** and is never stamped into the world lattice — the pose transform is
exact in the body's own frame. Two stencils **combine** by "mark all, then cut once", which is
order-free and fuses their shared edge. A stencil is **seated** on terrain by writing the height
slot alone, so its footprint is untouched. Arbitration fails safe toward solid.

## The things you will reach for

- **pose a body and query it** — `pose_new · pose_fwd_x/y · pose_inv_x/y · disk_hit`
- **combine two stencils order-free** — `combine_cut · shared_marked`
- **arbitrate overlapping solids** — `arb_solid — fail-safe toward solid (I4)`

## The worked examples, in the tests

Each `fn test_*` in [`tests/01-hex-place.loft`](tests/01-hex-place.loft) is a small, complete program you can copy:

- `fn main()` — the smallest call that does something real.
- each `fn test_*` — one contract, stated as an `assert` with the expected value in the message,
  so a failure tells you both what broke and what it should have been.

Run them yourself: `loft --interpret --tests tests` from the `hex_place/` directory.

## The rules that bite

- **Discover the API from source or `loft api hex_place` once published** — not from memory. Even the
  authors mis-recall signatures; the tests exist because guessing is unreliable.
- **A refusal is data, not an error.** Where a map can decline (the doorstep, recovery), it returns
  a *reason* and an *offer*, and your editor should show them rather than treating the call as failed.
- **No `ε` in an R1 comparison.** For content you authored, recovery is exact; a tolerance there is
  a defect, not a knob (`SPEC` **P4**).
