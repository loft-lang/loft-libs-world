<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_fit — the doorstep — refuse at authoring time what would not round-trip

`fits?` and `snap`. A continuous parameter off the grid the field distinguishes is silently
**snapped**, not rejected — so the doorstep refuses it up front, with a **named reason**, an
**offer** of the nearest fitting alternative, and the **residual** to show the user. Never a blank
no, never a silent correction.

Part of the `hex_*` family beside `hex_field` (cell sets), `hex_grid` (the lattice), `hex_edge`
(collision), `hex_way` (linework) and `hex_roof` (height profiles). Produced by **hexbody**, the
workshop that proves the family against an exact round trip.

## Using it

Add the dependency, `use hex_fit;`, and call the maps below. **`USAGE.md` is the worked guide**, with
every example pointed at a passing test in `tests/01-hex-fit.loft` — the tests are the executable documentation.
The eight things a caller gets wrong — starting with the one that made this package necessary, a value
the field cannot hold being *snapped* rather than refused — are worked one by one in
`tests/02-worked-examples.loft` (`@HXI-001..008`), cited from the functions they belong to.

Split from `ROUNDTRIP.md`'s objects and maps (`SPEC` **I-EXTEND**: a library defines its primitives
from its own semantics, for consumers it will never meet). See hexbody's `plans/lib-split/`.
