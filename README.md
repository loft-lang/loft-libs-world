<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# loft-libs-world — hex-world data + addressing for loft

Multi-package chunk repo for **hex-grid world libraries** — data
models, addressing primitives, and authoring surfaces for games
built on a hexagonal world map.  Each subdirectory is an independent
loft package published to the registry under its own name.

Per the chunked-repo design in
[loft's lib_plans/12-library-extraction/](https://github.com/jjstwerff/loft/blob/main/doc/claude/lib_plans/12-library-extraction/README.md)
§ Chunk grouping, and the 6-chunk topology in
[LAVITION.md](https://github.com/jjstwerff/loft/blob/main/doc/claude/LAVITION.md)
§ Library model.

## Why this chunk exists

Hex-world libraries form a tightly-coupled family: `hex_world`
defines the addressing primitive (chunked sparse coords); `hex_walls`
places linear features at sub-hex resolution; `hex_terrain` adds
height + material layers per hex; `hex_items` places instances with
12-direction placement.  Shipping them as one chunk means a single
co-versioned release on the registry — consumers don't have to chase
mutually-compatible point versions across 4 separate repos.

The chunk is **headless-usable** — no GPU or audio dependency.
Rendering helpers (e.g. hex-to-screen projection, mesh extrusion)
live in the [`loft-libs-graphics`](https://github.com/loft-lang/loft-libs-graphics)
chunk and consume hex_world types via dep.

## Packages

| Subdir | Package | Status |
|---|---|---|
| `hex_world/` | hex_world | ✅ 0.1.0 — sparse 32×32-chunk world, 4-byte Cell, save/load |
| `hex_walls/` | hex_walls | (planned, depends on curve-detection design) |
| `hex_terrain/` | hex_terrain | (planned) |
| `hex_items/` | hex_items | (planned) |

## License

LGPL-3.0-or-later for every package in this chunk.  See `LICENSE`.

## hex_grid

The GEOMETRY axis: the canonical hex-grid math every other `hex_*` library builds on —
pointy-top, odd-r offset, `L = sqrt(3)` (the moros convention, executable): lattice<->world
conversions, neighbors, distance, corners, canonical edges, plus the 12-orientation square
(`cell_*`) local basis for right-angled structures placed ON the hex world. Pure functions,
no state, no deps. Extracted from the crawler roguelike.

Family convergence (one hex basis for crawler / dryopea / moros — the
coordinate-convention rule, the bridge, the hex_walls/hex_terrain merge plans):
**[CONVERGENCE.md](CONVERGENCE.md)**.
