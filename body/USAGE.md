<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# Using `body`

> **The tests are the documentation that cannot rot.** Every claim here is a passing assertion in
> [`tests/01-body.loft`](tests/01-body.loft); read that file for the exact, compiling form. The
> `README.md` is the *why*; this is the map.

## The things you will reach for

- **Author a rig, save and reload it** — `rig_new` · `rig_bone` · `rig_write` · `rig_read` · `rig_eq`
- **Check a rig is well-formed** — `rig_admissible` (topological + ordered limits)
- **Pose a part / the whole body** — `pose_of` (one joint) · `rig_world_seg` (the tree)
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
