<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_fit — the doorstep — refuse at authoring time what would not round-trip

`fits?` and `snap`. A continuous parameter off the grid the field distinguishes is silently
**snapped**, not rejected — so the doorstep refuses it up front, with a **named reason**, an
**offer** of the nearest fitting alternative, and the **residual** to show the user. Never a blank
no, never a silent correction.

Also the `Draft`: a `hex_form` form carrying one embedded wall run, with its own doorstep
(`draft_fits`) and a round trip that notices a dropped run.

Part of the `hex_*` family beside `hex_field` (cell sets), `hex_form` (forms), `hex_recover`
(reading a form back), `hex_grid` (the lattice), `hex_edge` (collision), `hex_way` (linework) and
`hex_roof` (height profiles).

## Using it

Add the dependency and `use hex_fit;`.

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).

[`USAGE.md`](USAGE.md) is the map of the surface.  The eight things a caller gets wrong — starting with the one that made this package necessary, a value
the field cannot hold being *snapped* rather than refused — are worked one by one in
`tests/02-worked-examples.loft` (`@HXI-001..008`), cited from the functions they belong to.
