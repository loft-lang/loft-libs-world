<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_draw`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-draw.loft`](tests/01-hex-draw.loft); read that file for the exact, compiling form. This page is the map.

## What it does

The `draw` map: a `Plan` becomes floor cells, wall edges, openings and a roof height field. And
the inverse — a wall's **analytic surface** recovered as the exact average of its stored edges, so
a wall renders as **one flat quad** and a game emits it as a collision proxy derived from the
geometry itself.

## The things you will reach for

- **draw a house** — `draw_floor · draw_walls · place_opening · draw_roof`
- **get a wall's collision surface** — `surface_of · surface_is_exact · surface_span`
- **the mitered render quad** — `surface_quad`

## The worked examples, in the tests

[`tests/02-worked-examples.loft`](tests/02-worked-examples.loft) takes the seven things a caller gets
wrong and works each one as a running test, cited from the function it belongs to:

| tag | what it teaches |
|---|---|
| `@HXD-001` | a wall is **one flat quad**, not the strip of edges it is stored as — and the strip's zigzag is an exact constant per family, `sqrt(3)` apart, not an error term |
| `@HXD-002` | exactness is a **zero cross product** in integers; both obvious ways to test it reject geometry that is perfectly exact (16 of 24, and 12 of 24) |
| `@HXD-003` | the **miter closes the outline** exactly — and its corner is not the `Plan`'s corner, ever: one is a lattice rational, the other an irrational multiple of `sqrt(3)/2` |
| `@HXD-004` | a door is a **material, never a hole**: writing material 0 deletes the edge and the closed wall grows two loose ends |
| `@HXD-005` | a wall is the **boundary of a filled region**, so it costs no floor — and counting its edges needs all six directions, not the three a cell owns |
| `@HXD-006` | a feature is indexed by its **exact `t`**, never by its position in the run — two edges adjacent along the wall can sit two apart in storage |
| `@HXD-007` | the gable **ridge runs past the ends**; stopping it at the wall rolls the end over by `sqrt(3)/4` units, and the halo ring is where that is visible at all |

Each `fn test_*` in [`tests/01-hex-draw.loft`](tests/01-hex-draw.loft) is a small, complete program you can copy:

- `fn main()` — the smallest call that does something real.
- each `fn test_*` — one contract, stated as an `assert` with the expected value in the message,
  so a failure tells you both what broke and what it should have been.

Run them yourself: `loft --interpret --tests tests` from the `hex_draw/` directory.

## The rules that bite

- **Discover the API from source or `loft api hex_draw` once published** — not from memory. Even the
  authors mis-recall signatures; the tests exist because guessing is unreliable.
- **A refusal is data, not an error.** Where a map can decline (the doorstep, recovery), it returns
  a *reason* and an *offer*, and your editor should show them rather than treating the call as failed.
- **No `ε` in an R1 comparison.** For content you authored, recovery is exact; a tolerance there is
  a defect, not a knob (`SPEC` **P4**). The exactness lives in the INTEGER half — the summed direction
  and the rational mean — so test it there (`surface_heading`), not on a float projected through
  world coordinates (`@HXD-001`).
- **Never diff a recovered surface against `Plan` geometry.** What `surface_of` recovers is the wall
  as DRAWN, and the drawn footprint is the plan quantised to cells: the recovered corner is a lattice
  rational and the plan's is not, so they agree only at the origin and the difference is not an offset
  you can correct for (`@HXD-003`).
- **An opening is a material, not a gap.** `OPEN_NONE` is the wall material 0 — it deletes the edge
  and breaks the wall into two runs. Use `OPEN_GAP` for a real hole with the wall still there
  (`@HXD-004`).
