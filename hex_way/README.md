<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_way — a way is an exact centreline plus offsets, never a rasterised band

The **linework** axis of the `hex_*` family. A road, rail or path is a `Track` of straight
and arc segments in world space; its width is an **offset from that centreline**, and the
cells it marks are a *rasterisation* of the band, never the truth.

That distinction is the whole point: offsetting a stored band quantises and drifts, while
offsetting a centreline is exact at any width.

- `track_straight` / `track_arc` — build the centreline; `track_len`, `seg_point`,
  `seg_tangent`, `seg_curvature` read it.
- `track_offset` / `offset_legal` — the parallel curve, and whether it self-intersects.
- `way_mark` / `way_stamp` — rasterise the band into a `hex_field` cell set.
- `cut_arb` — tag each boundary edge with its **nearest** analytic surface, order-free.

Depends on `hex_edge` for the surfaces it tags. Split out of `crawler`/`hexbody` on
2026-07-24 (`SPEC` **L11**).
