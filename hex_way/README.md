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

- `track_new` + `track_straight` / `track_arc` — build the centreline; `track_len`,
  `seg_len`, `seg_point`, `seg_tangent`, `seg_curvature` read it.
- `track_distance` / `nearest_seg` — how far a point is from the way, and which segment.
- `track_offset` / `offset_legal` — the parallel curve, and whether the way may be that wide.
- `way_param` / `seg_param` — the milepost: arc length along the way to a point.
- `way_surfaces` + `way_stamp` — rasterise the band into a `hex_field` cell set and cut its
  boundary into an `EdgeSet`, each edge tagged with its segment's `hex_edge` surface.
- `way_mark` + `cut_arb` — the same in two phases, for a way built from several parts: mark
  every part, then tag each boundary edge with its **nearest** surface, order-free.
- `way_steps` — a staircase or terrace: heights that rise by `rise` every `tread`.

The quantisation floor is what all of this buys you out of, and it is **not one number**:
a band resolves 1.5 across a way running down a row and 0.866 down a column, so between two
rings of cell centres every requested width gives the identical footprint. An offset has no
floor at any width.

The seven ways a caller gets a plausible wrong answer — each passing every cheap check — are
worked in `tests/02-worked-examples.loft` (`@HXY-001..007`), cited from the functions they
belong to.

Depends on `hex_field` for cells and edges and on `hex_edge` for the surfaces it tags.

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).
