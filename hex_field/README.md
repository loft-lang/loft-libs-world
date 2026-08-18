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

**0.1.1 — the settled core, seeded from crawler's `hexform` (plan #5).**

0.1.1 is 0.1.0 with the same file format (byte for byte) and five behaviours corrected, all
found by writing this package's worked examples (@PLN141 row 10):

| what changed | before |
|---|---|
| `validate` code 5 means *no* outer loop | *not exactly one*, so any chunk holding two forms was refused while its areas summed exactly |
| `validate` code 6 refuses a self-touching outline | a map that visits a vertex twice passed every other check |
| a material outside `0..EDGE_MAT_MAX` is refused and counted (`edgeset_refused`) | the checked narrowing cast fell back to `0`, which in this layer means NO WALL — so `edge_set_mat(e, …, 300)` deleted the wall it was setting |
| `stencil_rotate` / `stencil_mirror` carry both edge slots | the surface half was dropped, so six turns lost the geometry while the cells returned exactly |
| `doc_read` checks `w`/`h` against the file size (`HXF_BAD_EXTENT`) | a 32-byte file claiming 4000×4000 allocated sixteen million cells before reporting a missing section |
| the `EDGE` section is written keyed by each edge's two cells | a copy of the source layer's vector, so an `EdgeSet` at another extent relocated every wall into a file that then loaded cleanly |
| `stencil_rotate` / `stencil_mirror` leak nothing | each leaked one `Stencil` **per call** — 4000 stores in 2000 rotations and 2000 reflections. Not a bug in this package: a loft function whose return paths disagree about ownership (the parameter on one, a freshly built value on the other) never frees the fresh one ([loft#982](https://github.com/loft-lang/loft/issues/982)). Both functions now express "an empty stencil comes back unturned" as a zero-turn copy instead of an early `return st` |

`outline_count`, `edgeset_refused`, `EDGE_MAT_MAX` and `HXF_BAD_EXTENT` are new; nothing was
removed or re-typed, so 0.1.1 is a drop-in for 0.1.0.

Landing next, in this order:

| what | why it is not here yet |
|---|---|
| **stencils** — a stencil is a small *field*; stamping is merging two fields; 60° rotation is an exact integer map (six rotations = identity, reflection gives 12 orientations) | designed, unimplemented — the first new work built *in* this package |
| **the document format** — round-trip = identity, gated by every consumer | does not exist anywhere yet |
| `EdgeSet` / `Surfaces` / `Materials` / `Features`, the region cache, levels | still in crawler while its kernel migration (plan #11 P2) exercises them; they move once settled |

The rule that keeps this honest: **no two copies, ever.** When a module moves here, the
consumer's copy is deleted in the same step.

## Known lint

`lattice_m(q, r)` and `nb_r(q, r, d)` do not read `q`, and the compiler says so. The
parameter is kept **deliberately**, so `lattice_k(q,r)` / `lattice_m(q,r)` and
`nb_q(q,r,d)` / `nb_r(q,r,d)` stay symmetric pairs at every call site. Dropping it would
make callers read `lattice_k(q,r), lattice_m(r)` — warning-free and worse. Revisit only
with the consumers present.

### The revisit — a consumer's answer (moros, 2026-07-22)

The second consumer now exists, so: **the lint is real but small, and it is a symptom of
something bigger.**

**The lint, measured.** Every consuming package inherits it — 18 warnings in `moros_map`, and
again in each of the four packages downstream of it. A dependency's lint is paid by everyone
who imports it, which is a reason to keep a library's own output clean that does not apply to
a leaf program. Not fatal; it does make "zero warnings" unusable as a signal in a consumer,
which is how a real warning hides.

**The bigger thing.** `lattice_k` / `lattice_m` and `nb_q` / `nb_r` **re-implement
`hex_grid`**, and the comments say so — *"Matches `hex_grid::hex_to_px` … verified against the
library before porting"*, *"Same SET as `hex_grid::hex_neighbor`"*. Verified once, by hand, and
not since. `hex_field` has no dependency on `hex_grid`.

They still agree. Checked over 100 cells (`q`, `r` ∈ −5…4, so both parities and both signs):
**0 position disagreements, and all 600 neighbour steps land in `hex_grid`'s set.**

**But moros carried the same duplicate and it silently diverged.** `moros_render`'s copy of the
odd-row shift used `(r % 2)` where `hex_grid` uses `(r & 1)`. Those agree for non-negative `r`
and differ for every negative odd row — loft's `%` yields −1 there — so every such row sat a
full hex step (√3) west of where it belonged. Nothing caught it for a year: both sides were
internally consistent, the suite was green, and the geometry looked plausible. Two more
duplicates in the same file (a neighbour table that could not be parity-aware, and a corner
ring in the opposite index order) had the same shape.

That is the cost of "verified before porting" — it is true on the day it is written.

**A proposal, not a patch** — it is your API and crawler is the other consumer:

- `hex_grid::hex_neighbor(q, r, dir)` already returns a **tuple**, so delegating dissolves the
  `nb_q` / `nb_r` split *and* its lint together, rather than trading one for the other.
- `lattice_k` / `lattice_m` could likewise become one `lattice(q, r) -> (k, m)`.
- The cost is a dependency edge `hex_field → hex_grid` that does not exist today. If leaf
  purity is deliberate, say so in this section and the duplication becomes a *decision* with a
  parity test guarding it — which is the other honest answer, and better than silence.

Either resolution beats the current state, where the only thing standing between the two
implementations is a comment.
