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

[`tests/02-worked-examples.loft`](tests/02-worked-examples.loft) works the six things a caller
gets wrong. Every one of them produces a roof that still has an apex, still is not flat and
still sheds water — so the conformance checks pass and the damage shows at the **eave**:

| tag | what it teaches |
|---|---|
| `@HXR-001` | a hip **is** a gable with a shorter ridge — the same call, a shorter `Track` |
| `@HXR-002` | on a quantised footprint a point-source cone cannot get both the eave and the drainage right; `roof_hip` gets both, because it never measures a radius |
| `@HXR-003` | the centre and the `Track` are **world** points, not cell indices |
| `@HXR-004` | groin and cloister are the same two barrels and the opposite operator (max vs min) |
| `@HXR-005` | ponding counts **interior** cells only — a hollow on the eave is not a pond |
| `@HXR-006` | recover the analytic surface and draw *that*; a plane wins ties, and the fit is R2 |

Each `fn test_*` in [`tests/01-hex-roof.loft`](tests/01-hex-roof.loft) is a small, complete program. Run them:
`loft --interpret --tests tests` from `hex_roof/`.

## ⛔ A fallback is not a recovery — `roof_match` can answer `ROOF_UNKNOWN`

This package **draws** a ridge, a hip and three vaults, and **fits** a plane, a cone and a
dome. Until 0.1.2 `roof_match` returned the least-bad of the three with `rf_kind` naming a
shape the field is not, and the residual was the only signal.

Measured on fields this package itself draws:

| the field | 0.1.1 said | residual | the truth |
|---|---|---|---|
| `roof_ridge` gable | `ROOF_CONE` | 1.209 | apex 6.318 where it is 6.0, slope 0.409 where it is 0.5 |
| `roof_hip` | `ROOF_CONE` | 0.615 | — |

`roof_match` returns **`ROOF_UNKNOWN`** now when no candidate is within `tol`, keeping the
best candidate's parameters and residual for diagnosis. `roof_eval` draws nothing for an
unknown kind, so a fallback can no longer be evaluated into a surface that is not the roof.

⚠ **No existing recovery changes** — a candidate within `tol` returns exactly as before; only
the path where all three were already wrong is affected.

✅ **AND THE MISSING KIND IS BUILT — `ROOF_RIDGE` / `roof_ridge_fit`.** A ridge is the cone's
own profile about a **segment** instead of a point, so the fit is `cone_at` with the radius
replaced by `track_distance` — the same primitive `roof_ridge` draws with, rather than a second
opinion about what distance-to-a-segment means. `RoofFit` gained `rf_x2` / `rf_y2` for the
segment's other end, zero for every other kind.

| the field | 0.1.1 | 0.1.2 |
|---|---|---|
| `roof_ridge` gable | ⛔ `ROOF_CONE`, residual 1.209 | ✅ `ROOF_RIDGE`, residual ≤ 0.001, peak and slope exact |
| `roof_hip` | ⛔ `ROOF_CONE`, residual 0.615 | ⚠ `ROOF_UNKNOWN` — still not one of the four, and saying so |

⚠ **THE RIDGE IS ASKED LAST**, after plane, cone and dome. A cone *is* a ridge whose segment
has zero length, so a fitter tried earlier would answer the less specific shape for both — the
same reason a plane wins ties. **No existing verdict moves.**

⚠ **AND THE HIP STILL ANSWERS UNKNOWN, WHICH IS DELIBERATE**: it keeps the refusal path
reachable. An unreachable refusal is one nobody would notice breaking.

## The rules that bite

- **Discover the API from source or `loft api hex_roof`** — not from memory.
- **A refusal is data, not an error** where a map can decline.
- **No `ε` in an exact comparison** (`SPEC` **P4**).
- **Check `eave_spread`, not just the apex.** A roof built from the wrong distance source
  looks right everywhere except the eave (`@HXR-002`, `@HXR-003`).
- **Coordinates here are world units.** Cell `(4,4)` is at world `(6.93, 6.0)` on a
  pointy-top odd-r lattice, and passing cell indices produces a plausible wrong roof
  (`@HXR-003`).
