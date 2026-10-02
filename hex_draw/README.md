<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_draw — draw a model into the field, and read its wall surface back

The `draw` map: a `Plan` becomes floor cells, wall edges, openings and a roof height field. And
the inverse — a wall's **analytic surface** recovered as the exact average of its stored edges, so
a wall renders as **one flat quad** and a game emits it as a collision proxy derived from the
geometry itself.

Part of the `hex_*` family beside `hex_field` (cells, edges, labels, heights), `hex_form` (`Plan`
and the side runs), `hex_grid` (the lattice), `hex_edge` (collision), `hex_way` (linework) and
`hex_roof` (height profiles).

## Using it

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).

Add the dependency, `use hex_draw;`, and call the maps below. `USAGE.md` is the map of the package,
with every claim pointed at a passing test in `tests/01-hex-draw.loft`.
The seven things a caller gets wrong — beginning with drawing a wall as the strip of edges it is
stored as — are worked one by one in `tests/02-worked-examples.loft` (`@HXD-001..007`), cited from
the functions they belong to.
