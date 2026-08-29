<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_grid — the hex lattice, and a square basis for what you build on it

The **coordinate** axis of the `hex_*` family: conversions between hex coordinates and the
world plane, neighbours, distance, corners and edges. Pure functions over `(q, r)` — no
structs, no state, nothing to construct or free.

It is the base every other `hex_*` package builds on: `hex_field` (occupancy and outlines),
`hex_shape` (the line, box and arc vocabulary), `hex_form` (turtle forms), `hex_draw`,
`hex_fit`, `hex_world`, `hex_terrain`. Four of them depend on it directly.

## Install

```sh
loft install hex_grid
```

```loft
use hex_grid;
```

## The smallest thing that does something

```loft
use hex_grid;

fn main() {
    (x, y) = hex_grid::hex_to_px(2, 3);
    println("centre of (2,3) = ({x}, {y})");
    (q, r) = hex_grid::px_to_hex(x, y);
    println("round-trips to ({q}, {r})");
    println("distance (0,0)->(2,3) = {hex_grid::hex_distance(0, 0, 2, 3)}");
}
```

```
centre of (2,3) = (4.330127018922193, 4.5)
round-trips to (2, 3)
distance (0,0)->(2,3) = 4
```

## The convention — read this before anything else

Everything here follows one convention, and mixing it with another hex library's is the
mistake that costs an afternoon:

- **Pointy-top hexes**, **odd-r offset** coordinates. Odd rows are shifted half a hex to the
  right, so a rectangle of cells stays upright. Axial coordinates would shear it.
- `(q, r)` **are** `(col, row)` offsets throughout — not axial, not cube.
- World scale is `L = √3` per hex step: `x = √3 · (col + ½·(row & 1))`, `y = 1.5 · row`.

```
x = SQRT3 * (q + 0.5 * (r & 1))      y = 1.5 * r
```

The scale constants `HEX_SIZE`, `HEX_LEN` and `SQRT3` are exported so a consumer can work in
the same units rather than re-deriving them.

## The two families

The prefix tells you which lattice a function belongs to. They are deliberately parallel, so
once you know one you know the other.

| | |
|---|---|
| `hex_*` | the hex lattice itself — six neighbours, hex distance, six corners |
| `cell_*` | a **square** local basis for right-angled structures placed *on* the hex world, oriented to one of the 12 directions (`k · 30°`) |

`cell_*` exists because buildings, corridors and anything with a right angle are painful to
express on a hex lattice and trivial on a square one. So place the structure in `cell_*`
coordinates, oriented to a hex direction, and convert at the boundary.

| what you want | hex | square |
|---|---|---|
| coordinate → world point | `hex_to_px` | `cell_to_px` |
| world point → coordinate | `px_to_hex` | `px_to_cell` |
| step one direction | `hex_neighbor` | `cell_neighbor` |
| which direction is that? | `hex_neighbor_dir` | `cell_neighbor_dir` |
| how far apart? | `hex_distance` | `cell_distance` |
| corner `i` of a tile | `hex_corner_px` | `cell_corner_px` |
| the shared name of an edge | `hex_canon_edge` | `cell_canon_edge` |

## The one that surprises people — `*_canon_edge`

An edge belongs to **two** tiles, and each names it differently: the edge between `(2,3)` and
`(3,3)` is direction 0 from one and direction 3 from the other. Store it under either name
and the two tiles disagree about whether there is a wall between them.

`hex_canon_edge(q, r, dir)` returns the **one** `(q, r, dir)` triple both sides agree on. Use
it as the key whenever an edge carries data — a wall, a door, a river, a shared boundary — and
the two tiles can never contradict each other.

## Rounding is exact where it matters

`px_to_hex` goes through `hex_round`, which rounds in cube space rather than rounding `q` and
`r` independently. Independent rounding picks the wrong hex near a corner; cube rounding
cannot. `hex_to_px` → `px_to_hex` round-trips exactly, which is what the tests pin.

## Testing

```sh
cd hex_grid && loft test
```

- `tests/01-hex-grid.loft` — the conversions, neighbours, metric and edges.
- `tests/02-parity.loft` — the `hex_*` and `cell_*` families answer consistently.
- `tests/03-worked-examples.loft` — `@HXG-001` … `@HXG-006`, the call sites the doc comments
  in `src/hex_grid.loft` cite. Each one is a real running test, so a citation cannot rot.

## Status

Stable and additive. Extracted from the **crawler** roguelike, and shared with **moros** —
the convention above is the single executable source of it, which is the point of the package:
two games that disagree about where a hex centre is do not interoperate.

Everything is a pure function over integers and floats, so there is no store, no lifetime
question and nothing to leak.

## License

LGPL-3.0-or-later.
