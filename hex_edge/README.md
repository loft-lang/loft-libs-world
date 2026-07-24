<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_edge — collision as an edge set, and the analytic surfaces behind it

The **collision** axis of the `hex_*` family. A wall is stored as marked edges of
`hex_field`'s `EdgeSet`, and every marked edge carries the id of the **analytic surface**
it belongs to — a straight line or an arc — so the proxy is *derived from the geometry*
rather than authored beside it.

Beside `hex_field` (cell sets and their outlines), `hex_way` (centrelines) and `hex_roof`
(height profiles).

- `Surfaces` — straights and arcs, with exact `surf_distance` / `surf_normal`.
  ⚠ **id 0 is reserved for "no surface"**, so a fresh set already holds one slot.
- `Materials` — solidity, height, opacity, sound, permeability, bounce.
- `collide` / `passable` / `sweep_path` — queries over the marked edges.
- `edges_cut` / `edges_halfplane` — marking an edge set from a region or a half-plane.

Split out of `crawler`/`hexbody` on 2026-07-24, where it had been maintained as two
byte-identical copies. `SPEC` **L11**: the library owns the shared table.
