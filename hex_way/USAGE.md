<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_way`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-way.loft`](tests/01-hex-way.loft); read that file for the exact, compiling form. This page is the map; the
> `README.md` is the *why*.

## a way is an exact centreline plus offsets, never a rasterised band

## The things you will reach for

- **build a centreline and read it** — `track_straight · track_arc · track_len · seg_point`
- **the parallel curve at a width** — `track_offset · offset_legal`
- **rasterise the band, or tag edges by nearest surface** — `way_mark · cut_arb`

## The worked examples, in the tests

Each `fn test_*` in [`tests/01-hex-way.loft`](tests/01-hex-way.loft) is a small, complete program. Run them:
`loft --interpret --tests tests` from `hex_way/`.

## The rules that bite

- **Discover the API from source or `loft api hex_way`** — not from memory.
- **A refusal is data, not an error** where a map can decline.
- **No `ε` in an exact comparison** (`SPEC` **P4**).
