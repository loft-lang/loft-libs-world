<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `hex_body`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-hex-body.loft`](tests/01-hex-body.loft); read that file for the exact, compiling form. The
> `README.md` is the *why*; this is the map.

## The things you will reach for

- **Author a rig, save and reload it** — `rig_new` · `rig_bone` · `rig_write` · `rig_read` · `rig_eq`
- **Check a rig is well-formed** — `rig_admissible` (topological + ordered limits)
- **Pose a part / the whole body** — `pose_of` (one joint) · `rig_world_seg` (the tree)
- **Author and pose a body in SPACE** — `rig_bone3` (an explicit axis + `oz`) · `rig_world_seg3` ·
  `bone_planar` / `rig_planar` (which record a bone writes)
- **Roll a wheel without slip** — `wheel_value` (from travel) · `wheel_angle` · `wheel_skid`
- **Refuse-with-an-offer at a joint limit** — `joint_fits` · `joint_offer` · `joint_residual`
- **Derive a collision proxy** — `bone_obb` (contains the bone's capsule) · `obb_contains` · `seg_dist`

## The worked examples, in the tests

[`tests/03-worked-examples.loft`](tests/03-worked-examples.loft) takes the eight things a caller
gets wrong and works each one as a running test, cited from the function it belongs to:

| tag | what it teaches |
|---|---|
| `@HXB-001` | a body is a **rig, never a pose** — the type has nowhere to put one, so two callers can pose one rig two ways at the same instant |
| `@HXB-002` | the values vector is **positional, unchecked and defaulted**: an empty one poses the whole rig neutral, and 9.0 turns poses exactly like 0 |
| `@HXB-003` | hold the **frame**, not the walk — `rig_world_frame3` once, `frame_point` per point, byte-identical to walking the tree each time |
| `@HXB-004` | the **axis is as given**: `(0,0,2)` is the same hinge as `(0,0,1)`, and `(0,0,0)` poses as the identity and is refused rather than repaired to `+z` |
| `@HXB-005` | the 2-D path is **planar-only** — on a spatial rig `rig_world_seg` (and every proxy built on it) answers for a bone 3.2 units away, silently |
| `@HXB-006` | a proxy **contains** its shape, and its overshoot is exactly `w * (sqrt(2) - 1)` at the corners |
| `@HXB-007` | a wheel rolls **by construction**: derived from travel it stays at machine dust where accumulating the same steps drifts a thousand times further |
| `@HXB-008` | **index order is the labelling**, the reader takes exactly the writer's spelling, and the PARENT is the doorstep's business rather than the parser's |

## The rules that bite

- **A pose is a pure function of the current joint values.** Two paths to the same value give a
  byte-identical pose; never accumulate a pose across frames, or you smuggle in history.
- **The wheel value is derived from travel, not accumulated** — that is what makes it roll without
  slip. The skid is machine-ε (a float round-trip), not algebraic 0; it is the body's one float step.
- **Flex is joints, not deformation.** A bending part is more bones. There is no skinning here.
- **A joint refusal is data, not an error.** A value outside a limit returns the nearest admissible
  angle and a residual; your editor should show them.
- **The axis is stored as given, not normalised** — the length of an axis carries no meaning, and
  normalising on the way in hands the author back a number they did not write. `rig_world_seg3`
  normalises at use. An axis of zero length is refused by `rig_admissible`.
- **⚠ The 2-D path is for PLANAR rigs, and it does not say so at the call.** `rig_world_seg`
  composes by adding joint angles, so it reads neither `oz` nor the stored axis: on a rig hinged
  about `+y` it answers for a bone up to **3.2 units away** on a bone 2 long — and `bone_obb` /
  `bone_shape_has` inherit it, so the collision proxy boxes empty space. Ask `rig_planar` first;
  a spatial rig belongs on `rig_world_frame3` / `rig_world_seg3` (`@HXB-005`).
- **The reader takes exactly what the writer emits.** `len 1.0` is not how this library spells
  `1.5`'s neighbour — a float has one spelling here, and `len x` is refused rather than read as a
  bone of length 0. The PARENT, though, is not the parser's business: a forward or absent parent
  round-trips faithfully and `rig_admissible` is what refuses it (`@HXB-008`).
- **A planar rig is bit-identical through the 2-D path, and only a single bone is through the 3-D
  one.** `rig_world_seg` composes by ADDING joint angles; `rig_world_seg3` composes rotations, and
  `cos(a+b)` is not `cos a cos b - sin a sin b` in f64. Measured over a five-bone planar chain: the
  root agrees exactly and the chain to **2.2e-16 m**. Use `rig_world_seg` for a planar rig — it is
  not deprecated and is not a slower path.
