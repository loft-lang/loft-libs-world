<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_roof — roof profiles as a height field, and the fit that recovers them

The **height** axis of the `hex_*` family. A roof is not a separate object: it is
`hex_field`'s `Heights` over a cell set. Seven profiles write it — cone, ridge, vault, hip,
dome, groin, cloister — and `roof_match` recovers one from an arbitrary height field.

**A roof must drain.** `roof_ponds` counts cells with no downhill neighbour; a profile that
ponds is a roof that leaks, and it is the load-bearing property every profile is checked on.

⚠ `roof_match` takes a **tolerance** and is a genuine fit — it recovers a profile from a
height field nobody authored. That is licensed; it is not an `ε` smuggled into an exact
path.

The six ways a caller picks the wrong distance source — each producing a roof that passes
every cheap check and fails at the eave — are worked in `tests/02-worked-examples.loft`
(`@HXR-001..006`), cited from the functions they belong to.

Depends on `hex_way` for ridge lines. Split out of `crawler`/`hexbody` on 2026-07-24
(`SPEC` **L11**).
