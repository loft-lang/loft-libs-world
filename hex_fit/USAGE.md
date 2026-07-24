<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_fit`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-fit.loft`](tests/01-hex-fit.loft); read that file for the exact, compiling form. This page is the map.

## What it does

`fits?` and `snap`. A continuous parameter off the grid the field distinguishes is silently
**snapped**, not rejected — so the doorstep refuses it up front, with a **named reason**, an
**offer** of the nearest fitting alternative, and the **residual** to show the user. Never a blank
no, never a silent correction.

## The things you will reach for

- **is this arc radius realisable?** — `arc_fits · arc_fit_n · arc_fit_residual`
- **why was it refused, and what is offered instead?** — `fit_reason · the offer + residual`
- **what has nothing to refuse, and why that is a result** — `level_fits (X66)`

## The worked examples, in the tests

Each `fn test_*` in [`tests/01-hex-fit.loft`](tests/01-hex-fit.loft) is a small, complete program you can copy:

- `fn main()` — the smallest call that does something real.
- each `fn test_*` — one contract, stated as an `assert` with the expected value in the message,
  so a failure tells you both what broke and what it should have been.

Run them yourself: `loft --interpret --tests tests` from the `hex_fit/` directory.

## The rules that bite

- **Discover the API from source or `loft api hex_fit` once published** — not from memory. Even the
  authors mis-recall signatures; the tests exist because guessing is unreliable.
- **A refusal is data, not an error.** Where a map can decline (the doorstep, recovery), it returns
  a *reason* and an *offer*, and your editor should show them rather than treating the call as failed.
- **No `ε` in an R1 comparison.** For content you authored, recovery is exact; a tolerance there is
  a defect, not a knob (`SPEC` **P4**).
