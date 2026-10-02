<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_recover — rebuild a model from the field — exactly, or with a reported residual

The `rebuild` map, and the reason it is trustworthy. **Constructive** recovery reads the form
off the field, enumerating nothing — every admitted form is convex, so the convex hull of the
filled cells IS the turtle polygon. An arbitrary blob that no grammar form draws lands in **R2**
with a positive residual, never a false R1.

Part of the `hex_*` family beside `hex_field` (cell sets), `hex_form` (the turtle forms it
recovers), `hex_grid` (the lattice), `hex_edge` (collision), `hex_way` (linework) and `hex_roof`
(height profiles).

**Two routines, two reaches.** `rebuild` matches the field against the ENUMERATED candidate set
(three sides up to `LEVEL`, plus four to six sides at length 1), so its R2 means *"nothing in the
admitted set draws this"* — not *"no stencil draws this"*. `rebuild_construct` enumerates nothing
and is bounded only by the field: an unequal-sided house is R2 to the first and R1 to the second,
and both answers are correct.

## Using it

Add the dependency and `use hex_recover;`.

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).

[`USAGE.md`](USAGE.md) is the map of the surface, and `tests/02-worked-examples.loft` works each
contract the signatures cannot state (`@HXV-001` … `@HXV-009`) as a running test.
