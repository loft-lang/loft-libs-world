<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_recover`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/`](tests/); read those files for the exact, compiling form. This page is the map.

## What it does

The `rebuild` map, and the reason it is trustworthy. **Constructive** recovery reads the form
off the field, enumerating nothing — every admitted form is convex, so the convex hull of the
filled cells IS the turtle polygon. An arbitrary blob that no grammar form draws lands in **R2**
with a positive residual, never a false R1.

## The things you will reach for

- **recover an authored stencil** — `rebuild`, which answers a `Rebuilt` (regime, form, rho, matches)
- **recover as text for undo/redo** — `rebuild_text` (`""` is the refusal)
- **reach forms the enumeration cannot** (above `LEVEL`, unequal sides) — `rebuild_construct`
  / `rebuild_construct_text`
- **recover many fields against one candidate set** — `candidate_forms` once, then
  `index_build` and `rebuild_indexed`

A guide that walks these in order: [`docs/01-getting-started.loft`](docs/01-getting-started.loft).

## The worked examples, in the tests

[`tests/01-hex-recover.loft`](tests/01-hex-recover.loft) is the conformance suite, and
[`tests/02-worked-examples.loft`](tests/02-worked-examples.loft) works `@HXV-001` … `@HXV-009`,
each cited from the function it teaches.  Each is a small, complete program you can copy:

- `fn main()` — the smallest call that does something real.
- each `fn test_*` — one contract, stated as an `assert` with the expected value in the message,
  so a failure tells you both what broke and what it should have been.

Run them yourself: `loft --interpret --tests tests` from the `hex_recover/` directory.

## The rules that bite

- **Discover the API from source or `loft api hex_recover`** — not from memory.
- **A refusal is data, not an error.** Recovery that declines answers R2 with a residual, and the
  `_text` routes answer `""`; show that rather than treating the call as failed.
- **No `ε` in an R1 comparison.** For content you authored, recovery is exact; a tolerance there is
  a defect, not a knob (`SPEC` **P4**).
- **`rho` is a COUNT of cells, not a distance.** It does not shrink towards a match. On the
  constructive path it is the number of cells the field's own convex hull ADDS, so a ring of six
  scores `rho 1` — the smallest positive value there is, and still a flat refusal, because no
  grammar form has a hole. Read a small `rho` as *"the hull is tight"*, never as *"nearly a
  stencil"* (`@HXV-005`).
- **Recovery does not depend on which way the stencil faces, or on how big it is.** The redraw
  that verifies an answer goes into a window derived from the form (`fit_chunk`), and `@HXV-003`
  pins that at all twelve headings.
