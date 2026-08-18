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

[`tests/02-worked-examples.loft`](tests/02-worked-examples.loft) works the six things a caller
gets wrong. Each one answers something entirely plausible and passes every cheap check — an
offset that is exactly `d` from the line, a band that covers the cells you expected, a road
whose footprint is right. The number that tells them apart is never the first one you reach
for:

| tag | what it teaches |
|---|---|
| `@HXY-001` | an offset has to **join** where the centreline joins; equidistance holds just as exactly on the wrong side |
| `@HXY-002` | the rasterised band has a resolution floor and the offset has none — and the floor is **1.5 down a row, 0.866 down a column** |
| `@HXY-003` | `offset_legal` is a **width** test (both rails), and the sharpest turn anywhere sets the limit for the whole way |
| `@HXY-004` | mark every part, **then** cut once — stamping part by part leaves walls inside the road, owned by whoever went first |
| `@HXY-005` | a milepost just before a turn reads **0**, not a full lap; clamping it the other way is a full-flight riser |
| `@HXY-006` | the shortest safe **tread** is the spacing of the flight's own cells: `sqrt(3)` down a row, 1.5 down a column |

Each `fn test_*` in [`tests/01-hex-way.loft`](tests/01-hex-way.loft) is a small, complete program. Run them:
`loft --interpret --tests tests` from `hex_way/`.

## The rules that bite

- **Discover the API from source or `loft api hex_way`** — not from memory.
- **A refusal is data, not an error** where a map can decline.
- **No `ε` in an exact comparison** (`SPEC` **P4**).
