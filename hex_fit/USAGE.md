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

[`tests/02-worked-examples.loft`](tests/02-worked-examples.loft) takes the eight things a caller gets
wrong and works each one as a running test, cited from the function it belongs to:

| tag | what it teaches |
|---|---|
| `@HXI-001` | **one model, one text** — a run has two spellings that draw the identical edges, and only the one the field answers with survives a round trip, so diff against `draft_canon_text` |
| `@HXI-002` | the round trip has to **notice a dropped run**: the footprint recovers either way, and only the run half can tell the two apart |
| `@HXI-003` | the reader takes **exactly** what the writer emits — a trailing space, `+3`, a `wall` line in the middle are refused, and a refusal is a stencil with an **empty form**, not an error |
| `@HXI-004` | **spelling is one gate, legality is another**: `draft_read` admits `d 99`, `draft_fits` refuses it by name and `draft_fit_p` offers the longest run that does fit |
| `@HXI-005` | the admissible `t` is **read off the field**, never computed — the two side families differ, and a door in the middle of the side is legal on one and refused on the other |
| `@HXI-006` | an off-shell radius shows **both candidates' cost**: `N = 35` is offered 36 (one unit away) and would silently have drawn 12 (twenty-three) |
| `@HXI-007` | a material id is a **name, not a magnitude**, so its refusal offers nothing — and an id the slot cannot hold erases the wall rather than truncating |
| `@HXI-008` | a doorstep with **nothing to refuse says so** (a level), kept honest by the same doorstep refusing a height off the voxel's invisible grid |

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
- **A round trip diffs against `draft_canon_text`, never against `draft_write`.** A stencil's form has
  one canonical corner and its embedded run has one canonical direction, and neither is fixed by the
  model you authored — write the wall from its other end and a byte diff reports *the model changed*
  on a model that did not (`@HXI-001`).
- **Hand-written text is refused, not repaired.** A trailing space, an extra field, a direction that is
  not a number — each yields a stencil with an empty form rather than a guess. Check
  `form_sides(s.df_form) == 0` after every `draft_read` (`@HXI-003`).
