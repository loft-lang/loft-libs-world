<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# body — the moving body: rigs, revolute joints, pure poses, and collision proxies

A body is a **rig** — a tree of bones connected by revolute joints, with the limits in the joints
between them, **never a pose**. The pose is *computed* from the current joint values, and that
computation is a **pure function**: hexbody answers *where a part is NOW*, never *what it will do*.
A part that bends is **more bones on more joints**, not a soft bone — so there is no vertex
deformation, no skinning, no soft body.

- `Rig` / `rig_write` / `rig_read` — a rig's canonical text; `write(read(R)) = R` byte-for-byte,
  floats included, a malformed text refused not repaired.
- `pose_of` — forward kinematics for one joint; `rig_world_seg` composes the whole tree.
- `wheel_value` / `wheel_angle` / `wheel_skid` — a wheel is a joint whose value is *derived from
  travel*, so it rolls without slip **by construction**.
- `joint_fits` / `joint_offer` — the doorstep: a joint value is ordinal, so a refusal offers the
  nearest admissible angle.
- `bone_obb` / `obb_contains` / `seg_dist` — the **collision proxy**, derived from the posed
  geometry: each bone's OBB **contains** its capsule, with a stated overshoot bound.

**Not part of the `hex_*` family, and deliberately so.** This library has no lattice dependency —
it is pure continuous-space kinematics — so by the family's naming rule (*`hex_` iff it depends on
hex code*) it carries no prefix. It composes *with* the `hex_*` geometry (a body is placed in a hex
world) but does not depend on it.

Produced by **hexbody**, the workshop. See its `plans/m1-moving-body/` for the design and the
exhaustive gate.
