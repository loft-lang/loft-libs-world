<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_place — place, seat and combine — the operations a model survives

A body carries a **pose** and is never stamped into the world lattice — the pose transform is
exact in the body's own frame. Two stencils **combine** by "mark all, then cut once", which is
order-free and fuses their shared edge. A stencil is **seated** on terrain by writing the height
slot alone, so its footprint is untouched. Arbitration fails safe toward solid.

Part of the `hex_*` family beside `hex_field` (cell sets, edges, heights), `hex_form` (shapes by
their sides), `hex_grid` (the lattice), `hex_edge` (collision), `hex_way` (linework) and `hex_roof`
(height profiles).

## Using it

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).

Add the dependency, `use hex_place;`, and call the maps below. `USAGE.md` is the map of the package,
with every claim pointed at a passing test in `tests/01-hex-place.loft`.
The six places a caller goes wrong — and in each case the nearby call that looks the same and is not —
are worked in `tests/02-worked-examples.loft` (`@HXP-001..006`), cited from the functions they belong to.
