<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_shape`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-shape.loft`](tests/01-hex-shape.loft); read that file for the exact, compiling form. This page is the map.

## What it does

The rest of `𝕄*`: a **wall** as a line primitive with a constant width (never a count of lattice
rows), the **box** in 12 directions with its two non-interchangeable orbits, and the **arc** whose
centre recovers exactly and whose radius quantises to a realisable shell `3k²+m²`.

## The things you will reach for

- **fill an arc and read its shell back** — `arc_fill · arc_recover_centre · arc_shells_upto`
- **know which orbit a box angle is in** — `box_orbit — 60° is a lattice symmetry, 30° is not`

## The worked examples, in the tests

Each `fn test_*` in [`tests/01-hex-shape.loft`](tests/01-hex-shape.loft) is a small, complete program you can copy:

- `fn main()` — the smallest call that does something real.
- each `fn test_*` — one contract, stated as an `assert` with the expected value in the message,
  so a failure tells you both what broke and what it should have been.

Run them yourself: `loft --interpret --tests tests` from the `hex_shape/` directory.

## The rules that bite

- **Discover the API from source or `loft api hex_shape` once published** — not from memory. Even the
  authors mis-recall signatures; the tests exist because guessing is unreliable.
- **A refusal is data, not an error.** Where a map can decline (the doorstep, recovery), it returns
  a *reason* and an *offer*, and your editor should show them rather than treating the call as failed.
- **No `ε` in an R1 comparison.** For content you authored, recovery is exact; a tolerance there is
  a defect, not a knob (`SPEC` **P4**).
