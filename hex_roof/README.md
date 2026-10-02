<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_roof — roof profiles as a height field, and the fit that recovers them

The **height** axis of the `hex_*` family. A roof is not a separate object: it is
`hex_field`'s `Heights` over a cell set. Seven profiles write it — `roof_cone`, `roof_ridge`
(a gable, or a hip with a shorter ridge), `vault_arc`, `roof_hip`, `dome`, `vault_groin`,
`vault_cloister` — and `roof_match` reads a height field back as a plane, a cone, a dome or
a ridge (`ROOF_UNKNOWN` when none fits), which `roof_eval` then draws exactly.

**A roof must drain.** `roof_ponds` counts cells with no downhill neighbour; a profile that
ponds is a roof that leaks, and it is the load-bearing property every profile is checked on.

⚠ `roof_match` takes a **tolerance** and is a genuine fit — it recovers a profile from a
height field nobody authored. That is licensed; it is not an `ε` smuggled into an exact
path.

`eave_spread` is the second check: a roof built from the wrong distance source still has an
apex and still drains, and only the spread of its eave heights shows it.

Six contracts a signature does not carry — a roof that passes every cheap check and fails
at the eave, a centre given in cells instead of world units, groin against cloister,
ponding on the boundary, and drawing the recovered surface rather than the cells — are
worked in `tests/02-worked-examples.loft` (`@HXR-001..006`), cited from the functions they
belong to.

Depends on `hex_field` for cells and heights and on `hex_way` for ridge and crown lines.

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).
