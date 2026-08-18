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

[`tests/02-worked-examples.loft`](tests/02-worked-examples.loft) works the six contracts a caller
gets wrong, each beside the nearby call that looks identical and is not:

| tag | what it teaches |
|---|---|
| `@HXP-001` | two adjacent stencils: **nobody** owns the shared edge — cut the union once, and the seam is interior |
| `@HXP-002` | order-free **by construction**, overlap included: the tie is the lower material id, not the stamp order |
| `@HXP-003` | a level is a **filter before the cut**, not an arbitration after it — and it is not a height |
| `@HXP-004` | seating writes the **height slot only**; the footprint never moves and the residual is returned |
| `@HXP-005` | the pose is the sole float step — and **one measurement of its error is not a bound** |
| `@HXP-006` | arbitration fails safe toward **solid**, the owner is the lowest id, and `-1` is a sentinel |

`@HXP-001` is the same question `hex_grid`'s `@HXG-003` answers one level down — there, one wall
between two hexes must resolve to one stored slot; here, one seam between two stencils should not
be stored at all.

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
- **Never draw two stencils' walls and overlay them.** Union first, cut once. The overlay is
  order-dependent and grows a wall down the middle of what the author drew as one building
  (`@HXP-001`).
- **Do not take `ε_seam` from one measurement.** `pose_residual` is a jagged, deterministic
  function of the point; near the frame origin it commonly reads exactly zero (`@HXP-005`).
