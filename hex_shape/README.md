<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_shape — the shape vocabulary beside the turtle form — the line, the box, the arc

The rest of `𝕄*`: a **wall** as a line primitive with a constant width (never a count of lattice
rows), the **box** in 12 directions with its two non-interchangeable orbits, and the **arc** whose
centre recovers exactly and whose radius quantises to a realisable shell `3k²+m²`.

Part of the `hex_*` family beside `hex_field` (cell sets and edges), `hex_form` (shapes by their
sides), `hex_grid` (the lattice), `hex_place` (combining stencils), `hex_edge` (collision),
`hex_way` (linework) and `hex_roof` (height profiles).

## Using it

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).

Add the dependency, `use hex_shape;`, and call the maps below. `USAGE.md` is the map of the package,
with every claim pointed at a passing test in `tests/01-hex-shape.loft`.

Every primitive here takes a parameter the lattice cannot hold continuously, so **every answer is
split** — the arc's centre is exact and its radius is a grid of shells; a run's line is exact and
its orientation is not; twelve directions are exact and twelve are 1.1021° off. The nine ways a
caller gets that wrong are worked in `tests/02-worked-examples.loft` (`@HXS-001..009`), cited from
the functions they belong to.

Two of them are about this package's own **instruments** rather than its shapes: `wall_along_max`
reads 0.97 on a correct in-between wall, which is the value it was built to flag as broken (the
chain count is the verdict), and `set_connected` says true for a ring you can walk straight through
(`flood_outside` + `leak_count` is the verdict).
