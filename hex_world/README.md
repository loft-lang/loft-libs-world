<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_world — sparse hex-grid world data model for loft

```sh
loft install hex_world
```

A sparse hex-grid world of single-layer cells, stored in 32×32 chunks that exist only where
something was written.  Pure loft, no dependencies.

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).

## What's in it

- `Cell` — 4 bytes: `c_color: u8`, `c_height: u8`, `c_age: u16`.  `c_color == 0` is the
  EMPTY sentinel, not a colour: start a palette at 1.
- `Chunk` — a 32×32 grid of cells; `World` — the chunks that exist, plus a `tick`.
- Addressing — global AXIAL `(q, r)`; `chunk_idx_32(v)` / `hex_idx_32(v)` floor-divide and
  wrap correctly for negative coordinates, where `v / 32` and `v % 32` do not.
- Cells — `get_cell` (an absent chunk reads as `cell_empty()`, and reading never allocates),
  `set_cell` (creates the chunk), `has_chunk`, `ensure_chunk`, `cell_count`,
  `neighbour_count` (filled axial neighbours, 0..6).
- `tick_and_decay(w, base_lifetime, neighbour_lease, decay_window)` — one world step: every
  filled cell ages by 1, and a cell whose age reaches `base_lifetime + neighbour_lease ×
  neighbours + decay_window` (neighbours counted before the step) is emptied.
- I/O — `world_save(w, path)` / `world_load(w, path)`: a little-endian file with a `'WRLD'`
  magic and version, the tick, and each non-empty chunk's filled cells (6 bytes each).
  `world_load` answers 0 for a missing file, a wrong magic or an unknown version.

Nothing in the library frees a chunk: one emptied by `set_cell` or `tick_and_decay` stays until
the caller drops it, or until a `world_save` / `world_load` round trip, which writes non-empty
chunks only.

⚠ The `(q, r)` here are axial, while `hex_grid`'s `(q, r)` are an odd-r offset pair — both are
two integers, so nothing catches a mix.  And this is not `hex_voxel`, the layered voxel column
store with its own `'WTTH'` file format.

## Tests

```sh
cd hex_world && loft --interpret --tests tests
```

`tests/hex_world.loft` covers get/set; `tests/02-persist.loft` save/load and its edge cases
(empty file, bad magic, bad version, negative coordinates); `tests/worked-examples.loft` the
five contracts `@HXW-001..005` cited from the functions they belong to.
