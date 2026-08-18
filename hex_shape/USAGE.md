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

[`tests/02-worked-examples.loft`](tests/02-worked-examples.loft) works the nine things a caller
gets wrong. Every primitive here takes a **continuous** parameter the lattice cannot represent
continuously — a radius, a run length, an angle, a width — so every answer is split, and the
split is what a signature cannot carry:

| tag | what it teaches |
|---|---|
| `@HXS-001` | an arc's **centre** is exact and its **radius** is a grid of four shells over 0.5–4.5 wu; ask for 16 and you get 12, silently |
| `@HXS-002` | only **twelve** of the 24 directions are exact — the rest are 1.1021° off — and no two of the three `N` classes are commensurable, which is why `WALL_W` is a constant |
| `@HXS-003` | a legal run length depends on which **corner** you started from: `wall_min_p` is 1 or 2 for the *same* direction |
| `@HXS-004` | `wall_along_max` reads **0.97** on a correct in-between wall — the picket value. The chain count is the verdict; the max is a description |
| `@HXS-005` | a run reads back as exact integers, but **not which end you called the start** — `read.d == d` is the wrong assertion |
| `@HXS-006` | the twelve box angles are **two families of six** (33 cells against 31), related by no exact map |
| `@HXS-007` | **connected is not closed**: one cell out of 22 and `set_connected` still says true while the outside reaches all 27 courtyard cells |
| `@HXS-008` | three separate errors sit between the mouse and storage — anchor snap, endpoint snap, and the stored sawtooth |
| `@HXS-009` | a door is an **annotation**, so the doored tower is byte-identically one arc |

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
