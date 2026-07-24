<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_edge`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-edge.loft`](tests/01-hex-edge.loft); read that file for the exact, compiling form. This page is the map; the
> `README.md` is the *why*.

## collision as an edge set, and the analytic surfaces behind it

## The things you will reach for

- **build a surface set and measure distance** — `surfaces_new · surf_straight · surf_arc · surf_distance`
- **the material table** — `materials_new · material_add · mat_solid · mat_height`
- **query collision over marked edges** — `collide · passable · sweep_path`

## The worked examples, in the tests

Each `fn test_*` in [`tests/01-hex-edge.loft`](tests/01-hex-edge.loft) is a small, complete program. Run them:
`loft --interpret --tests tests` from `hex_edge/`.

## The rules that bite

- **Discover the API from source or `loft api hex_edge`** — not from memory.
- **A refusal is data, not an error** where a map can decline.
- **No `ε` in an exact comparison** (`SPEC` **P4**).
