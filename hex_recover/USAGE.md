<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_recover`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-recover.loft`](tests/01-hex-recover.loft); read that file for the exact, compiling form. This page is the map.

## What it does

The `rebuild` map, and the reason it is trustworthy. **Constructive** recovery reads the form
off the field, enumerating nothing — every admitted form is convex, so the convex hull of the
filled cells IS the turtle polygon. An arbitrary blob that no grammar form draws lands in **R2**
with a positive residual, never a false R1.

## The things you will reach for

- **recover an authored stencil** — `rebuild — returns regime, form, rho, matches`
- **recover as text for undo/redo** — `rebuild_text`
- **reach unequal-sided houses the enumeration cannot** — `the constructive path — see `X45``

## The worked examples, in the tests

Each `fn test_*` in [`tests/01-hex-recover.loft`](tests/01-hex-recover.loft) is a small, complete program you can copy:

- `fn main()` — the smallest call that does something real.
- each `fn test_*` — one contract, stated as an `assert` with the expected value in the message,
  so a failure tells you both what broke and what it should have been.

Run them yourself: `loft --interpret --tests tests` from the `hex_recover/` directory.

## The rules that bite

- **Discover the API from source or `loft api hex_recover` once published** — not from memory. Even the
  authors mis-recall signatures; the tests exist because guessing is unreliable.
- **A refusal is data, not an error.** Where a map can decline (the doorstep, recovery), it returns
  a *reason* and an *offer*, and your editor should show them rather than treating the call as failed.
- **No `ε` in an R1 comparison.** For content you authored, recovery is exact; a tolerance there is
  a defect, not a knob (`SPEC` **P4**).
- **`rho` is a COUNT of cells, not a distance.** It does not shrink towards a match. On the
  constructive path it is the number of cells the field's own convex hull ADDS, so a ring of six
  scores `rho 1` — the smallest positive value there is, and still a flat refusal, because no
  grammar form has a hole. Read a small `rho` as *"the hull is tight"*, never as *"nearly a
  stencil"* (`@HXV-005`).
- **Recovery does not depend on which way the stencil faces, or on how big it is.** It used to:
  the redraw that verifies a construction went into a fixed 25×25 window and clipped anything
  larger, so a side-13 triangle came back R2 with the right answer in hand, and at a vertex
  heading an 85-cell shape was refused while a 105-cell one passed. Fixed in 0.1.1; the window is
  derived from the form (`fit_chunk`), and `@HXV-003` pins it at all twelve headings.
