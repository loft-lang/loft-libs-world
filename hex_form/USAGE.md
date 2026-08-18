<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_form`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-form.loft`](tests/01-hex-form.loft); read that file for the exact, compiling form. This page is the map.

## What it does

The **model** axis of the `hex_*` family. A stencil's outline is a closed turtle cycle over the
12 headings of `H₁₂`, exact in `ℤ²`; the `Plan` is the rectangle a house rasterises from; and
every form has a **canonical text** it writes to and reads back from **byte-for-byte** — which is
what makes a project file diffable and an undo history exact.

## The things you will reach for

- **author a form and check it closes** — `form_new · form_admissible · form_fill`
- **save and reload byte-identically** — `form_write · form_read · form_eq`
- **one shape, one text, whatever corner it was drawn from** — `form_canon_text`

## The worked examples, in the tests

[`tests/02-worked-examples.loft`](tests/02-worked-examples.loft) takes the seven things a caller
gets wrong and works each one as a running test, cited from the function it belongs to:

| tag | what it teaches |
|---|---|
| `@HXF-001` | the canonical text is a **fixed layout**, pinned byte for byte — generate *these* bytes |
| `@HXF-002` | the reader accepts exactly what the writer emits, and a refusal arrives as a **zero-sided form**, not an error |
| `@HXF-003` | the **name** is written beside the model, not into it — and it has to be one word |
| `@HXF-004` | one shape, one canonical text, from any corner — and shifting a corner by hand means shifting `h0` too |
| `@HXF-005` | a **turn of 0** is a re-spelling, not a corner: both spellings fill the same cells, only one is admissible |
| `@HXF-006` | closing is **two independent conditions**, and each catches what the other cannot |
| `@HXF-007` | a length is a **step count**, never a distance — edge and vertex steps differ by exactly `sqrt(3)` |

Each `fn test_*` in [`tests/01-hex-form.loft`](tests/01-hex-form.loft) is a small, complete program you can copy:

- `fn main()` — the smallest call that does something real.
- each `fn test_*` — one contract, stated as an `assert` with the expected value in the message,
  so a failure tells you both what broke and what it should have been.

Run them yourself: `loft --interpret --tests tests` from the `hex_form/` directory.

## The rules that bite

- **Discover the API from source or `loft api hex_form` once published** — not from memory. Even the
  authors mis-recall signatures; the tests exist because guessing is unreliable.
- **A refusal is data, not an error.** Where a map can decline (the doorstep, recovery), it returns
  a *reason* and an *offer*, and your editor should show them rather than treating the call as failed.
- **No `ε` in an R1 comparison.** For content you authored, recovery is exact; a tolerance there is
  a defect, not a knob (`SPEC` **P4**).
- **Hand-written text is refused, not repaired.** A trailing space, an extra field, a length that is
  not a number, an `h0` outside `0..11` — each yields a zero-sided form rather than a guess. Check
  `form_sides(f) == 0` after every `form_read`; the signature cannot tell you the parse failed
  (`@HXF-002`).
