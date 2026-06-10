# hex_terrain

The OVERLAND terrain layer of the `hex_*` family: a coarse overland hex lattice
(pointy-top, odd-r — the moros convention via `hex_grid`) is the terrain
authority; the fine, walked world derives from it by **pure functions**.

Ported from a user-tuned Python blueprint (crawler repo: `OVERLAND.md` +
`tools/overland_blueprint.py`); designed to be consumed alike by crawler,
moros and dryopea (see `CONVERGENCE.md` at the repo root).

## Model

- **Cells** carry height / material / moisture / flow direction + accumulation
  / relief — the same record shape works at every scale (dryopea's
  `OverlandMap` is the sibling of this layout).
- **Seamless merge**: fields at a fine point are a smooth kernel blend of the
  overland cells around it — adjacent tiles merge by construction.
- **Terrain types are content**: per type TWO independent numbers — `tt_rise`
  (general lift taken from tile relief) and `tt_steep` (internal jaggedness,
  which also wins boundary contests: between any two types runs a crisp
  fractal LINE and the steeper type claims the contested band).
- **Relief**: max neighbor height delta, compounded by the COUNT of steep
  neighbors; water-flow cells contribute zero — rivers cut deep for free.
- **Rivers**: priority-flood hydrology for truth; geometry = canonical
  left/right edge bit per shared edge + hash-jittered interior control point
  (confluences) + fractal midpoint displacement that meanders harder on flat
  land near the sea; courses trim to attach exactly ON the sea/lake
  shorelines; every course carves an additive, slope-shaped valley and the
  carved mouth floods — the sea comes inland, proportional to river size.

## The invariant

**Window independence**: every sample is a pure function of
`(terrain, types, params, rivers, x, y)`. No hidden state, no sequential RNG.
The tests rebuild the world from the same seed and assert sampled equality.

## Use

```loft
use hex_terrain;

p = terrain_params(seed, 500.0);          // 500 m overland tiles
t = terrain_new(48, 37);
// ... author cell heights/moisture/materials (content side) ...
terrain_hydrology(t, p, LAKE_MAT);        // flood-fill, flow, accumulation
// ... classify land materials (content side) ...
terrain_relief_pass(t, types, p);
rivers = terrain_rivers(t, types, p);
smp = terrain_sample(t, types, p, rivers, x, y);   // height/material/water
```
