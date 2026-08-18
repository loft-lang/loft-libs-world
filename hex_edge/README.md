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

## Moving a body: thread the cell, do not re-derive it

Found while building lavition's editor on this package (moros#10, 2026-07-28), and now
answered in the library — **use `sweep_path_from` in a movement loop**:

```
(t, cq, cr, d) = sweep_path_from(e, cq, cr, x0, y0, x1, y1);
```

carrying `cq, cr` from one step to the next. `sweep_path` is the same call with the
starting cell derived by `hex_at`, which is right for a first step and wrong for the step
after a stop.

**Why.** Stopping at a wall leaves the position exactly *on the bisector between two
cells* — the normal state, since it is where every blocked sweep ends. `hex_at` there has
to break a tie, and it does not toss a coin: measured over all six directions it answers
the **far** cell, every time. So the next call starts past the wall with the wall behind
it, sees nothing, and reports the whole segment clear. The symptom is "collision does not
work"; the truth is that it worked for exactly one step. The `havep` exclusion handles
this *within* one call, but that memory does not survive the return — and the sweep
already told you the cell, so hand it back.

**Why not a skin.** Coming to rest a small distance short does work — moros used 1 cm at
its scale — but the smallest distance that works is not a constant. Measured: `1e-15` at
the origin, `1e-11` about `1.7e3` world units out, `1e-9` about `1.7e6` out. It is a
float-resolution floor rather than a geometric clearance, so a value calibrated where it
was tested is silently wrong elsewhere in the same world, and the library has no better
idea of the right number than the caller does. That is why there is no `sweep_path_skin`:
the constant it would need does not exist. Threading the cell needs none.

**The far-field limit**, which no cure removes: past roughly `5e6` cells from the origin a
double position can no longer hold "on the bisector" to better than this code's `1e-9`
tolerance. There the position itself is the limit, not the sweep, and the answer is to
recentre the world rather than the arithmetic. Pinned in `@HXE-002`/`@HXE-003`.
