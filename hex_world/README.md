<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_world — sparse hex-grid world data model for loft

```sh
loft install hex_world
```

Sparse 32×32-chunk world data model — the addressing primitive for
hex-grid games built on the [lavition](https://github.com/lavition)
engine and the loft library ecosystem.

## What's in it

- `Cell` — 4-byte packed (`c_color: u8`, `c_height: u8`, `c_age: u16`).
- `Chunk` — fixed 32×32 grid of cells, allocated lazily.
- `World` — sparse map of chunks indexed by chunk-relative `(q, r)`.
- Addressing — `chunk_idx_32(v)` / `hex_idx_32(v)` (floor-divide
  for negative coords), `get_cell` / `set_cell` /
  `ensure_chunk` / `has_chunk` / `cell_count` / `neighbour_count`.
- I/O — `world_save(w, path)` / `world_load(w, path)` (4-byte cell
  + tick + sparse-chunk binary format with magic header).

## Why this package exists

`hex_world` is consumed by:

- TTT v5 (`game_protocol/examples/v5_*`) — the canonical multiplayer
  smoke test reuses `Cell` for X/O marks.
- The audience-generative-art demo
  ([plans/future/36](https://github.com/jjstwerff/loft/tree/main/doc/claude/plans/future/36-audience-generative-art))
  — projector + phone clients share the same chunked binary surface.
- Future hex_walls / hex_terrain / hex_items packages — they place
  features *within* the hex_world addressing space.

⚠ **The lavition editor is NOT a consumer**, though this list used to say it
was. It authors hex maps against a different lineage — moros's voxel column
store, published as `hex_voxel` — which shares neither this package's cell model
(one `Cell` per hex) nor its file format (`'WRLD'`, 4-byte cells, versus
`'WTTH'`, 8-byte cells with per-layer CRC and sections). The two were briefly
both called `hex_world`; that name stays here and the voxel store took
`hex_voxel` (loft-lang/loft-libs-world#13, moros plan #19 `L4`).

Renamed from `world` 2026-06-01 (see
[LAVITION.md W.1](https://github.com/jjstwerff/loft/blob/main/doc/claude/LAVITION.md#next-library-work--execution-order))
— the `hex_` prefix disambiguates from voxel / tile / BSP "world"
expectations.

## Tests

```sh
cd hex_world && loft --interpret --tests tests
```

Smoke test (`tests/hex_world.loft`) covers get/set/round-trip;
`tests/02-persist.loft` covers save/load + edge cases (empty file,
bad magic, bad version, negative coords).
