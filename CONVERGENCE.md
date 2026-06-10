<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# CONVERGENCE.md — one coherent hex basis for crawler, dryopea/lavition and moros

How the hex_* family's NEW routines (`hex_grid`, extracted from crawler) merge with the
EXISTING ones (`hex_world`, `gridmesh`, dryopea's in-monorepo `wall.loft`/`overland.loft`,
moros's Python convention tooling) into one basis every game consumes. Grounded in the
2026-06-10 ecosystem audit; each step is small, ordered, and independently shippable.

## The landscape today (verified)

| Code | What it is | Convention | Consumers |
|---|---|---|---|
| `hex_grid` (this repo, 0.1.0) | pure GEOMETRY: lattice<->world, neighbors, metric, corners, canonical edges + the 12-orientation `cell_*` square basis | **odd-r offset** (pointy-top, `L = √3` — the moros convention) | crawler (live; its in-repo copies deleted) |
| `hex_world` (this repo, 0.1.0) | chunked cell STORAGE: get/set, save/load, decay | **axial** (its `neighbour_count` walks the axial ring) | lavition |
| `gridmesh` (loft-libs-graphics) | chunk-local grid->mesh toolkit: spatial index, `SegMesh`, `ChunkField` dirty rebuilds | **axial** (own `axial_dq/dr` tables) | audience_crystal, moros_render |
| dryopea `lib/wall.loft` | 3D hex walls: corner coordinates, layers, openings | own embedded hex math | dryopea (in-monorepo) |
| dryopea `lib/overland.loft` | overland map cells: material/height/water | (data layout, little geometry) | dryopea (in-monorepo) |
| crawler `wallgeo` + `gen` | 2D wall outlines (boundary -> corner graph -> room-snap -> smooth); seeded dungeon gen | odd-r via `hex_grid` | crawler (extraction Tier 2) |
| moros `tools/build_overworld_map.py` | the convention's Python origin (docs + map tooling) | odd-r offset | moros |

Function-level overlap is ZERO today — but THREE independent hex-math implementations
exist (`hex_grid`, gridmesh's tables, wall.loft's corners), and the family speaks two
coordinate conventions. That is what convergence resolves.

## The coordinate decision (the crux)

Both conventions stay — each where it is strongest — with `hex_grid` owning the bridge:

- **Axial is the INTERCHANGE + STORAGE convention.** `hex_world` keys, `gridmesh`
  indices, and every cross-library API use axial (q, r): it is parity-free (no row
  stagger), so neighbor math, chunking and algorithms need no `row & 1` branches.
  The existing storage/meshing layer is already axial — zero churn.
- **Odd-r offset is the AUTHORING + PRESENTATION convention.** Map files, the moros
  overworld JSON, and rectangular level data stay odd-r: it matches row-major storage
  and the documented moros convention. crawler keeps odd-r internally — it converts at
  the library boundary (pure-function bridge, negligible cost).
- **The bridge lives in `hex_grid`** (the geometry axis owns conversions):
  `axial_to_offset(q, r)`, `offset_to_axial(col, row)`, plus axial twins of the lattice
  fns (`axial_neighbor`, `axial_distance`, `axial_to_px`). The world-position formula is
  ONE formula — the axial forms convert and delegate, so px output is bit-identical
  from either convention (tested by round-trip).

## The target layering

```
                 hex_grid          (geometry: both conventions + the bridge; NO deps)
                 ^      ^
         hex_world      gridmesh   (axial storage)   (chunked meshing; graphics chunk)
                 ^      ^
        hex_walls        hex_terrain     (pending; see merge plans below)
                 ^
  crawler (2D, odd-r)   dryopea/lavition (3D, axial)   moros (3D + Python tooling)
```

The single rule: **lattice math is implemented once, in `hex_grid`.** Every other hex_*
library consumes it; none re-implements a neighbor table or a px formula.

## Merge plans per existing component

1. **`hex_grid` 0.2.0 — the bridge release** (small, first):
   axial<->offset conversions + axial twins + a CONVENTION section in the package header
   stating the rule above. Round-trip tests: offset->axial->offset = identity;
   `axial_to_px == hex_to_px` after conversion, exact.
2. **`hex_world` 0.1.x — documentation only**: state explicitly that its (q, r) are
   AXIAL and reference `hex_grid` for geometry. No behavior change. (Its
   `neighbour_count` may later delegate to `hex_grid`'s axial ring — at its next minor,
   not required for coherence.)
3. **`gridmesh` — converge at its next minor**: its `axial_dq/dr` tables duplicate what
   `hex_grid` now owns. Either depend on `hex_grid` (a graphics-chunk -> world-chunk dep:
   light, world is loft-data-only — OWNER DECISION per the chunk transitive-deps rule)
   or freeze the local tables with a comment naming `hex_grid` as canonical. Until then
   the tables must stay numerically identical (a parity test in gridmesh is cheap
   insurance).
4. **`hex_walls` — the real merge** (pending design, now with a concrete inventory):
   dryopea's `wall.loft` (3D: corner coords, layers, openings) and crawler's `wallgeo`
   (2D: boundary edges -> corner graph -> room-line snap -> Laplacian smooth) share a
   CORE: boundary-edge collection + the corner graph + canonical shared edges (already
   partly in `hex_grid`). Extract that core as `hex_walls`; keep the per-game passes —
   crawler's smoothing/room-snap, dryopea's 3D layering — as consumer-side modules
   (or `hex_walls` submodules if a second consumer appears for either). Prerequisite:
   crawler's wallgeo decouples from `Sim` (tiles-predicate parameterization — its
   extraction Tier 2). wall.loft's lattice math is REPLACED by `hex_grid` calls during
   the move — retiring the third implementation.
5. **`hex_terrain`** — from dryopea's `overland.loft` (cell layout: material/height/
   water). Lavition-led; crawler's streamed overworld adopts it when streaming lands
   (the `hex_world` + `gridmesh` + `hex_grid` pipeline; see crawler EXTRACTION.md).
6. **moros** — stays the convention's *specification* home; `hex_grid` is the
   *executable* form. SCENE_MAP.md gains a pointer to `hex_grid`. Parity lock: a small
   shared fixture (`hex_fixtures.json`: sampled (col,row) -> (x,y) pairs) asserted by
   BOTH `hex_grid`'s loft tests and moros's Python tests — the two implementations can
   never drift silently.

## Order of work

bridge (1) -> docs (2, 6) -> gridmesh parity test, converge at next minor (3) ->
wallgeo decoupling, then the hex_walls core merge (4) -> hex_terrain when lavition
extracts it (5). Each step lands through its repo's own gate (branch -> local gate ->
PR with trace -> CI -> squash-merge), registry releases per REGISTRY_SUBMIT.md.

## The capability roadmap — what the coherent stack buys

The point of convergence is that ADDITIONS FLOW FREELY: a capability built for one game
drops into the others because the representations are shared. The target capabilities,
each homed in the layering:

**Exactly-round towers.** Round structures must be ANALYTIC (centre + radius arcs), not
polygon approximations — "exactly round" survives every zoom and the 3D build. Home: the
shared wall REPRESENTATION in `hex_walls` generalizes from straight segments to
*segment + arc* primitives (a tower wall = arcs anchored to a hex centre; a doorway = an
arc gap). Renderers tessellate as they wish; geometry stays exact. crawler's `WallSeg`
emit and dryopea's 3D wall build both consume the same primitive list.

**24-direction walls.** The `cell_*` square basis carries 12 orientations (k x 30 deg)
today; structures want k x 15 deg (the moros placement primitive already anticipates
12/24-dir rotation). Home: `hex_grid` extends the orientation vocabulary to 24
(direction tables + basis rotation); `hex_walls` places wall pieces at any of them.
One vocabulary — every game's structures, props and sprites rotate on the same 24 steps.

**Cliff sides.** A cliff is a wall whose cause is a HEIGHT DELTA instead of a
solid/floor flip. Home: the `hex_walls` boundary core takes a *boundary predicate*
(today: solid-vs-open; cliffs: `|height(a) - height(b)| > threshold` against
`hex_terrain` height cells). The same boundary->corner-graph->outline machinery then
emits cliff faces for free — in 2D as outline bands, in 3D as vertical faces. One core,
two faces of the same idea.

**Water-flow calculations.** dryopea's `overland.loft` already stores water + 3 bits of
flow direction per cell. The FLOW ALGORITHM (downhill routing over the height field,
accumulation, pooling) is game-agnostic: home it beside the terrain data (`hex_terrain`,
or a `hex_water` axis if it grows simulation state of its own), computing over
`hex_world` storage with `hex_grid` neighbor math. crawler rivers, dryopea irrigation,
moros oceans — one solver.

**Collision detection.** Layered, not monolithic: `hex_grid` provides the pure geometry
queries (point-in-hex, circle-vs-segment, circle-vs-arc — the arc case is what makes
round towers WALKABLE); `hex_walls` answers "does this move cross a wall/cliff
primitive" against its segment+arc sets; `physics_2body` (loft-libs-game) supplies
dynamics where a game wants them. crawler's `pos_blocked` becomes a consumer of the
shared queries; dryopea's 3D collision consumes the same primitives extruded.

The enabling rules (these are what "coherent" MEANS here):
1. one shared wall/structure primitive list (segments + arcs + their gaps) consumed by
   every renderer and the collision layer alike;
2. boundary generation is predicate-parameterized (solid, height-delta, material, ...);
3. orientation is one vocabulary (the 24 steps) defined once in `hex_grid`;
4. simulation routines (water, decay, growth) compute over `hex_world` cells with
   `hex_grid` math — never private neighbor tables.

## Invariants that lock coherence (test-enforced, not promised)

- ONE px formula: axial and offset forms produce bit-identical world positions
  (hex_grid round-trip tests).
- ONE direction table: gridmesh's tables == hex_grid's (parity test until the dep).
- Cross-language parity: the moros Python tooling and hex_grid agree on the shared
  fixture file.
- No new hex math outside `hex_grid`: a reviewer checklist line for every hex_* PR.
