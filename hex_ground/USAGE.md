<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_ground`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-ground.loft`](tests/01-hex-ground.loft); read that file for the exact, compiling form. This page is the map.

## What it does

The ground of a place: a height and a surface kind per cell, shaped by terraces, a hill, a seat,
a dig and cliffs.

## The things you will reach for

- **make the ground** — `ground_new(q0, r0, w, h, step, kind)`; `ground_centre` and
  `ground_cell_near` go between cells and points in the ground's unit
- **the measurements** — `ground_measures(storey, hillside, bank, blend)`
- **smooth a stepped map** — `ground_terraces` from a `GroundSteps` (levels on a square grid);
  `terrace_height` is the height at one point
- **add a hill** — `ground_hill`, a cosine from its top to its foot
- **seat a structure** — `ground_seat`: its footprint level, the ground round it blended back, the
  lowest ground under it answered (`hex_place::seat_blend`)
- **dig a bed** — `ground_dig`: the cells' ground lowered, its bank along their straight sides;
  `sides_distance` is the distance it measures by
- **cliffs** — `ground_cliffs` for the stretches of a level's straight sides inside the cliff
  areas (`GroundBox`), `ground_sharpen` to stand the ground at its level's height there

- **read the ground** — `ground_height_at` for the smooth surface at a point, `ground_corner_height`
  for a corner (the mean of its cells), `ground_height_on` for one side of a cliff

## The rules that bite

- **One unit.** Heights, distances and `step` are all in the unit you chose; nothing here converts.
- **A dig and a cliff need cells that carry straight sides.** A blob too small to show a direction
  has no outline (`hex_draw::cells_outline` answers no walls for a loop of fewer than three), so
  it is not dug and has no cliff.
- **`ground_seat` writes heights only.** The footprint's cells and edges are untouched; mark its
  surface kind yourself if a floor differs from the grass round it.
