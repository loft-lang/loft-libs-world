<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_shape — the shape vocabulary beside the turtle form — the line, the box, the arc

The rest of `𝕄*`: a **wall** as a line primitive with a constant width (never a count of lattice
rows), the **box** in 12 directions with its two non-interchangeable orbits, and the **arc** whose
centre recovers exactly and whose radius quantises to a realisable shell `3k²+m²`.

Part of the `hex_*` family beside `hex_field` (cell sets), `hex_grid` (the lattice), `hex_edge`
(collision), `hex_way` (linework) and `hex_roof` (height profiles). Produced by **hexbody**, the
workshop that proves the family against an exact round trip.

## Using it

Add the dependency, `use hex_shape;`, and call the maps below. **`USAGE.md` is the worked guide**, with
every example pointed at a passing test in `tests/01-hex-shape.loft` — the tests are the executable documentation.

Split from `ROUNDTRIP.md`'s objects and maps (`SPEC` **I-EXTEND**: a library defines its primitives
from its own semantics, for consumers it will never meet). See hexbody's `plans/lib-split/`.
