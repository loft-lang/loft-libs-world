<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_body — the moving body: rigs, revolute joints, pure poses, and collision proxies

The **body** axis of the `hex_*` family — hexbody's own body model, and deliberately *not* a
generic bones/animation library. A body is a **rig**: a tree of bones connected by revolute joints,
with the limits in the joints between them, **never a pose**. The pose is *computed* from the
current joint values, and that computation is a **pure function**: this answers *where a part is
NOW*, never *what it will do*. A part that bends is **more bones on more joints**, not a soft bone —
so there is no vertex deformation, no skinning, no soft body.

**What makes it our model, not a generic one:** the joints carry **correctly computed restrictions**
(their limits are the doorstep — a value outside is refused *with an offer*, because a joint value
is ordinal), and the model has room for the physical data a hex-world body needs (strength, leeway)
that a generic animation rig does not carry.

- `Rig` / `rig_write` / `rig_read` — a rig's canonical text; `write(read(R)) = R` byte-for-byte,
  floats included, a malformed text refused not repaired.
- `pose_of` — forward kinematics for one joint; `rig_world_seg` composes the whole tree.
- **`rig_bone3` / `rig_world_seg3`** — the same rig in SPACE: an explicit revolute **axis** and an
  `oz`, the eight numbers `hex_part::Hinge` already carries. `rig_bone` is the planar special case
  (`oz = 0` about `(0,0,1)`) and its signature is unchanged. ⚠ The axis is stored **as given, not
  normalised**, and one of zero length is refused by `rig_admissible` — a joint with nothing to
  turn about. ⚠ A planar rig's bytes are unchanged; a spatial bone writes its own record `bone3`,
  so an older reader refuses it rather than silently reading its planar projection.
- `wheel_value` / `wheel_angle` / `wheel_skid` — a wheel is a joint whose value is *derived from
  travel*, so it rolls without slip **by construction**.
- `joint_fits` / `joint_offer` — the joint doorstep: an out-of-limit value is refused with the
  nearest admissible angle and a residual.
- `bone_obb` / `obb_contains` / `seg_dist` — the **collision proxy**, derived from the posed
  geometry: each bone's OBB **contains** its capsule, with a stated overshoot bound.

**Why `hex_body` and not `body`.** It is part of the `hex_*` world-building family and composes with
the hex geometry — a body lives in, is seated on, and collides with a hex world, and interaction
and seating (the next milestones) will couple it to the lattice. It carries **no lattice import
today** (pure continuous-space kinematics — `sin`/`cos`/`sqrt` only); the prefix names its family
and its trajectory, not a current dependency.

Produced by **hexbody**, the workshop. See its `plans/m1-moving-body/` for the design and the
exhaustive gate.
