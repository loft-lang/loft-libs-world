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
- **A planar rig is bit-identical through the 2-D path, and only a single bone is through the 3-D
  one.** `rig_world_seg` composes by ADDING joint angles; `rig_world_seg3` composes rotations, and
  `cos(a+b)` is not `cos a cos b - sin a sin b` in f64. Measured over a five-bone planar chain: the
  root agrees exactly and the chain to **2.2e-16 m**. Use `rig_world_seg` for a planar rig — it is
  not deprecated and is not a slower path.
