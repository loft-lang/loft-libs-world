<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# tile_world — an unbounded, layered square-grid world, and a small traversal-puzzle game in it

```sh
loft install tile_world
```

The game this library is built for is designed in [DESIGN.md](DESIGN.md).

## The memory model

- **Layers:** `z` is the floor level, unbounded both ways: negative for dungeons and mines,
  positive for upper floors and mountain tops.
- **Chunks:** each layer is an unbounded grid held in 32x32 chunks that exist only where
  something was set, found through a hash keyed on `(cx, cy, z)`. The same addressing as
  hex_world and moros, with floor division for negative coordinates.
- **Terrain:** two bytes a cell, a kind (void, floor, wall, water, rail) and a tile number for
  the picture, so a chunk is 2 KB.
- **Things** (carts, chairs, tables, rocks, anvils, potions) move, so they live in a second
  sparse hash of per-cell stacks, bottom first. They cost memory only where they are.

## The rules

- **Heights:** height counts in steps: a table is 2, a chair 1, a layer 3. The hero steps up at
  most 1 and down any amount.
- **Climbing a layer:** a table with a chair on top (3) is the way up a layer, and back down
  onto it from the floor above.
- **Pushing** (`walk(w, dx, dy)`): walking into a thing at the hero's level pushes it one cell.
  A chair or a table that cannot move is climbed onto instead.
- **Pulling** (`pull(w, dx, dy)`): the hero walks one cell and drags what is behind it. A chair
  standing on the floor can be pulled up onto a table the hero stands on.
- **Carts:** carts run on rails, and a line of carts moves as a train. The hero pushes up to 3
  carts and pulls up to 2.
- **Rocks and anvils:** pushed only, never pulled, and never by a cart or another rock. On the
  rails they block trains.
- **Light:** a wizard's staff is a light.  Fixed to a wall it is a torch: walking into
  that wall takes it, and walking into a bare wall while holding it fixes it there; on the
  floor it is picked up by walking onto it.  The open sky (nothing on the layer above) is
  daylight.  Each cell is one shade darker per step from the nearest light, down to
  `DARKEST`; a wall is lit but passes no light on.
- **Green potion:** walking onto one gives +2 strength for 60000 ms of the world's clock
  (`tick(w, ms)`).

| Function | |
|---|---|
| `world()`, `set_terrain`, `fill`, `terrain`, `tile`, `chunk_count` | the grid |
| `place`, `things`, `top`, `height`, `train` | the stacks |
| `put_hero`, `walk`, `pull`, `play(w, "s s d pa …")` | moving |
| `tick`, `boosted`, `push_strength`, `pull_strength` | strength and time |
| `shades(w, x0, y0, x1, y1, z)`, `light_source`, `open_sky`, `DARKEST` | light: 0 at a light, one shade darker a step, never black |
| `view(w, x0, y0, x1, y1, z)` | a layer as text |
| `first_climb()`, `reached_ledge(w)`; `the_cellar()`, `in_daylight(w)` | the demo levels |

## Play

```sh
loft examples/first_climb.loft
```

`w a s d` walk, `pw pa ps pd` pull. Reach the ledge above the tables.

## Tests

`tests/rules.loft` covers every rule as a small scene with hand-worked results.
`tests/first_climb.loft` replays the demo level's solution, and shows that a wrong first move
makes it unsolvable. The suite passes on the interpreter and `--native`, and the C backend
matches it on x86-64, 32-bit x86 and 32-bit RISC-V.
