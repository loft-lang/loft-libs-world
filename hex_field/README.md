<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_field — exact-integer hex cell sets and their outlines

The **field** axis of the `hex_*` family: occupancy over a bounded chunk of hex
coordinates, per-cell labels and heights, and the tracer that turns a cell set into an
**exact integer vector map** — closed loops whose vertices are lattice points, not floats.

Beside `hex_grid` (the lattice + the square local basis), `hex_world` (chunked sparse
addressing) and `hex_terrain` (height + material layers).

A guide: [docs/01-getting-started.loft](docs/01-getting-started.loft).

## The contract — what a consumer may rely on

Everything here is **integer**. Every hex centre *and every corner* lies on

```
   x = k · √3/2      y = m / 2        (k, m INTEGER)
```

with `centre(q,r) = (2q + (r&1), 3r)` and the six corners at `(0,±2)`, `(±1,±1)`. Cell
centres satisfy `k ≡ m (mod 2)`. There is no float in the geometry, no epsilon compare,
and therefore no drift — which is what makes exact diffs, exact undo and (once stencils
land) exact rotation possible.

The load-bearing guarantee is the **area round-trip**:

```
   Σ integer shoelace over all loops  ==  12 × (number of hexes)      holes negative
```

A single hex shoelaces to exactly `12`. **Cell count and traced outline can therefore
never disagree** — the vector map provably describes the set it came from, which is what
makes it safe to hand to a renderer or a mesh builder.

`validate(v, cells)` checks the rest before a consumer sees the data, and returns `0` for a
good map:

| code | what it refuses |
|---|---|
| 1 | no loops at all |
| 2 | a loop of fewer than three vertices |
| 3 | a segment that is not one of the six hex edges — which is also how a zero-length one is caught, since its delta is `(0,0)` |
| 4 | the area round-trip above |
| 5 | no outer loop at all |
| 6 | a vertex that repeats — the outline touches itself, and a triangulator cannot resolve a pinch |

Integrality is not in the table because it is not a check: a vertex is an `integer` pair,
so there is nowhere for a non-lattice point to come from.

**Code 5 says *no* outer loop, not *more than one*.** A chunk is allowed to hold several
disjoint forms — two buildings in one 32×32 window, or one form the chunk edge cut in half —
and each contributes its own outer loop. Requiring exactly one refused those maps while
`trace` had produced them correctly and the areas summed exactly. `outline_count(v)` reports
how many there are, so a caller that knows its form is a single piece keeps the stronger
property by asserting on it.

`trace` can never produce a 6 — a hex vertex touches three *mutually adjacent* cells, so a
traced boundary cannot pinch. The check is there for the maps `validate` receives from
somewhere else: a file, or a consumer's own builder. Two hexes emitted as two circuits
joined at their shared corners give twelve legal hex-edge segments whose shoelace is
exactly `12 × 2`, so every other check here passes them.

## Scale is the CONSUMER's, not this package's

Every threshold here is **dimensionless** — hex steps, lattice world units, or pure ratios.
This package never states a metre. A consumer picks its own metres-per-hex and converts
once, at the edge. (A library that ships a metre has shipped a decision that belongs to the
game.)

**Dimensionless is not the same as interchangeable, and two of the three units here differ
by √3.** A *hex step* is the distance between neighbouring centres. A *lattice world unit*
is the one `x = k·√3/2, y = m/2` defines, in which a hex has circumradius **1** — so one hex
step is **√3 ≈ 1.732** world units. `form_hexdisk(w, n)` takes hex steps; `form_circle(w,
radius)` and `form_octagon(w, apothem)` take world units. The same `3` therefore means two
different discs:

| call | cells |
|---|---|
| `form_hexdisk(w, 3)` | 37 |
| `form_circle(w, 3.0)` | 13 |
| `form_circle(w, 3.0 * 1.7320508075688772)` | 37 |

## Bounded chunks on purpose

`HexSet` covers `q ∈ [q0, q0+w)`, `r ∈ [r0, r0+h)` — a chunk, never a global graph. Every
routine is `O(chunk)` and none walks an unbounded neighbourhood, so the same code serves a
32×32 world chunk and a one-off tower window. Forms spanning chunks are traced per chunk
with a halo and stitched. Nothing assumes the form is centred or the world reachable.

## Testing

```sh
loft test          # in this directory
```

The tests here assert the **contract** — the round-trip, the validator, lattice
integrality, occupancy bounds, and a negative control that must go red when a single cell
is added without re-tracing. A gate that cannot fail is not a gate.

Consumers keep their own checks: crawler diffs the traced output against a Python oracle's
golden JSON, which is a *consumer's* verification of this package rather than this
package's verification of itself.

## Status

The package holds the cell set (`HexSet`), the traced outline and its validator, the layered
document format (`doc_write*` / `doc_read`, round-trip = identity), stencils (`stencil_*`:
exact 60° rotation and reflection, stamping and unstamping), and the edge layer (`EdgeSet`,
one material per wall).  A material outside `0..EDGE_MAT_MAX` is refused and counted
(`edgeset_refused`) rather than narrowed — a narrowed value would be 0, which means NO WALL.

The rule that keeps the family honest: **no two copies, ever.** When a module moves here,
the consumer's copy is deleted in the same step.

## The lattice is `hex_grid`'s, restated

`hex_field` is a leaf: it does not depend on `hex_grid`.  `lattice_k` / `lattice_m` and
`nb_q` / `nb_r` therefore restate `hex_grid`'s convention (pointy-top, odd-r offset), and
`tests/08-hex-grid-parity.loft` keeps the two equal over both parities and both signs —
every world position and every neighbour step.  An odd-row shift written `r % 2` instead of
`r & 1` agrees for every non-negative row and is a full hex step off on every negative odd
one, so the test is what a second copy needs.

`lattice_m(_q, r)` and `nb_r(_q, r, d)` take a `q` they do not read, so each pair keeps one
argument list at every call site (`lattice_k(q, r)` / `lattice_m(q, r)`).
