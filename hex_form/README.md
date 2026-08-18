<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_form — the exact turtle form on the hex lattice, and its canonical text

The **model** axis of the `hex_*` family. A stencil's outline is a closed turtle cycle over the
12 headings of `H₁₂`, exact in `ℤ²`; the `Plan` is the rectangle a house rasterises from; and
every form has a **canonical text** it writes to and reads back from **byte-for-byte** — which is
what makes a project file diffable and an undo history exact.

Part of the `hex_*` family beside `hex_field` (cell sets), `hex_grid` (the lattice), `hex_edge`
(collision), `hex_way` (linework) and `hex_roof` (height profiles). Produced by **hexbody**, the
workshop that proves the family against an exact round trip.

## Using it

Add the dependency, `use hex_form;`, and call the maps below. **`USAGE.md` is the worked guide**, with
every example pointed at a passing test in `tests/01-hex-form.loft` — the tests are the executable documentation.
The five rules of the canonical text — C1–C5, and every one of them a way a hand-written form parses
and means something else — are worked one by one in `tests/02-worked-examples.loft` (`@HXF-001..007`),
cited from the functions they belong to.

Split from `ROUNDTRIP.md`'s objects and maps (`SPEC` **I-EXTEND**: a library defines its primitives
from its own semantics, for consumers it will never meet). See hexbody's `plans/lib-split/`.
