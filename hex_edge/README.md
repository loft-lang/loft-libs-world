<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_edge — collision as an edge set, and the analytic surfaces behind it

The **collision** axis of the `hex_*` family. A wall is stored as marked edges of
`hex_field`'s `EdgeSet`, and every marked edge carries the id of the **analytic surface**
it belongs to — a straight line or an arc — so the proxy is *derived from the geometry*
rather than authored beside it.

Beside `hex_field` (cell sets and their outlines), `hex_way` (centrelines) and `hex_roof`
(height profiles).

- `Surfaces` — straights and arcs, with exact `surf_distance` / `surf_normal`.
  ⚠ **id 0 is reserved for "no surface"**, so a fresh set already holds one slot.
- `Materials` — solidity, height, opacity, sound, permeability, bounce.
- `collide` / `passable` / `sweep_path` — queries over the marked edges.
- `edges_cut` / `edges_halfplane` — marking an edge set from a region or a half-plane.

Split out of `crawler`/`hexbody` on 2026-07-24, where it had been maintained as two
byte-identical copies. `SPEC` **L11**: the library owns the shared table.

## A note from a consumer: where `sweep_path` leaves you

Found while building lavition's editor on this package (moros#10, 2026-07-28), and
recorded here rather than changed, because it is a caller's obligation and not a defect:

**A caller must not come to rest exactly at the fraction `sweep_path` returns.** The stop is
exact and correct, and the position it leaves is *on the bisector between two cells* — so
the next call's `hex_at(x0, y0)` has to round, and when it rounds to the far cell the sweep
starts on the other side of the wall with the wall behind it. The character then walks
through. The symptom is "collision does not work"; the truth is that it worked for exactly
one step.

The `havep` exclusion in `sweep_path` anticipates this *within* one call — the comment says
so — but that memory does not survive the return. A fresh call beginning on a bisector has
no previous cell to exclude and is genuinely ambiguous.

The consumer-side fix is a skin: stop a small distance short along the segment, so the
resting position is unambiguously on the walkable side. Moros uses 1 cm at its scale.

Whether the library should offer this — a `sweep_path_skin(…)`, or simply returning a
fraction already backed off — is your call; the arithmetic is trivial and the trap is not
obvious, which is the argument for it living here.
