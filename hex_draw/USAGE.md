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
  a defect, not a knob (`SPEC` **P4**).
