<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_roof`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-roof.loft`](tests/01-hex-roof.loft); read that file for the exact, compiling form. This page is the map; the
> `README.md` is the *why*.

## roof profiles as a height field, and the fit that recovers them

## The things you will reach for

- **write a profile** — `roof_cone · roof_ridge · roof_hip · roof_dome`
- **check it drains** — `roof_ponds — must be 0`
- **recover a profile from a height field (an R2 fit)** — `roof_match`

## The worked examples, in the tests

Each `fn test_*` in [`tests/01-hex-roof.loft`](tests/01-hex-roof.loft) is a small, complete program. Run them:
`loft --interpret --tests tests` from `hex_roof/`.

## The rules that bite

- **Discover the API from source or `loft api hex_roof`** — not from memory.
- **A refusal is data, not an error** where a map can decline.
- **No `ε` in an exact comparison** (`SPEC` **P4**).
