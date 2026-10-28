<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# tile_world — an unbounded, layered square-grid world, and the routines that answer questions about it

```sh
loft install tile_world
```

The world model a 2-D (90-degree) game stands on.  It holds what is where, and answers:
- how far it is to walk somewhere (`shortest_path`);
- what can be seen from where (`line_of_sight`);
- how much light falls on each cell (`shades`).

What the pieces of a game do, such as pushing, climbing or picking up, is the game's: it
declares its kinds here and moves things with `place` and `take`.

## The memory model

- **Layers:** `z` is the floor level, unbounded both ways: negative below the ground,
  positive above.
- **Chunks:** each layer is an unbounded grid held in 32x32 chunks, which exist only where
  something was set.  They're found through a hash keyed on `(cx, cy, z)`; this is the
  hex_world and moros addressing, with floor division for negative coordinates.
- **Terrain:** two bytes a cell, a kind (0..15) and a tile number (0..4095, the picture), so a
  chunk is 2 KB.
- **Ground:** a layer can have a ground (`set_ground`): every cell of it nobody set reads as
  that terrain, so open land costs no memory.  A cell set to void is a pit even then; `clear`
  gives a cell back to the ground.
- **Edges and rise:** a cell holds its north and its west edge (the east and south ones are its
  neighbours'), so walls, fences, doors and windows can stand BETWEEN cells; and a rise, its own
  height in steps, for slopes.  They share one 32-bit word a cell, in an array a chunk gets only
  when one of its cells first has an edge or a rise.
- **Things:** numbers the game chooses, kept in a second sparse hash of per-cell stacks,
  bottom first.

## Kinds

The game declares each terrain kind and each thing; the routines read only what it declared.

```loft
w = tile_world::world();
tile_world::terrain_kind(w, FLOOR, true, false, false, ".");   // walk, opaque, light, glyph
tile_world::terrain_kind(w, WALL, false, true, false, "#");
tile_world::thing_kind(w, TABLE, 2, true, false, false, "T");  // height, blocks, opaque, light, glyph
tile_world::thing_kind(w, TORCH, 0, false, false, true, "i");
```

- **Undeclared terrain:** a terrain kind nobody declared (other than void) is a wall: not
  walkable, and opaque.
- **Undeclared things:** a thing nobody declared is 1 high, blocks, and is not opaque.
- **Edges:** `edge_kind(w, kind, walk, opaque, glyph)`: a wall is neither walked across nor seen
  through, a door is crossed but blocks sight, a window and a fence are seen through but not
  crossed.  An undeclared edge kind is a wall.

## API

| Function | |
|---|---|
| `world()`, `terrain_kind`, `thing_kind` | a world, and what its kinds are |
| `set_terrain`, `fill`, `terrain`, `tile`, `open_sky`, `chunk_count`, `chunk_of`, `inside` | the grid |
| `layers(w)`, `chunks(w, z)`, `stacks(w)` | what the world holds, in a fixed order: for saving it, or walking all of it |
| `set_ground(w, z, kind, tile)`, `ground`, `ground_tile`, `clear`, `is_set` | a layer's open land, and unset versus set-to-void |
| `edge_kind`, `set_edge(w, x, y, z, side, kind)`, `edge`, `crossable`, `see_through` | edges between cells (`NORTH`, `EAST`, `SOUTH`, `WEST`) |
| `set_rise(w, x, y, z, steps)`, `rise` | a cell's own height, 0..15 steps; `height` adds it under the things |
| `place`, `take`, `things`, `top`, `height` | the stacks |
| `passable`, `opaque`, `lit` | questions about one cell |
| `shortest_path(w, x0, y0, x1, y1, z, limit = 64) -> vector<(integer, integer)>` | breadth-first over passable cells and crossable edges; the goal may be a thing to walk up to |
| `line_of_sight(w, x0, y0, x1, y1, z) -> boolean` | no opaque cell or edge between (Bresenham; a diagonal step sees round either side of a corner); a wall itself is seen |
| `shades(w, x0, y0, x1, y1, z, lights = []) -> vector<integer>` | 0 at a light, one shade darker per step, up to `DARKEST` (7) and never black; the open sky is daylight, opaque cells are lit but pass no light on, `lights` adds what figures carry |
| `view(w, x0, y0, x1, y1, z, marks = [])` | a layer as text, from the declared glyphs |

## Tests

`tests/world.loft` and `tests/edges.loft` cover each routine, the edges, the ground and the
rise over a neutral set of kinds (floor, wall, window;
box, lamp, curtain), with hand-worked answers.
