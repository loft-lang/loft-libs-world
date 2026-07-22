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

`validate(v, cells)` checks the rest before a consumer sees the data: loops closed · every
segment a real hex edge · no zero-length segment · no repeated vertex (self-touching) ·
vertices integral · exactly one outer loop with holes wound opposite · the area round-trip
above. It returns `0` for a good map.

## Scale is the CONSUMER's, not this package's

Every threshold here is **dimensionless** — hex steps or pure ratios. This package never
states a metre. A consumer picks its own metres-per-hex and converts once, at the edge.
(A library that ships a metre has shipped a decision that belongs to the game.)

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

**0.1.0 — the settled core, seeded from crawler's `hexform` (plan #5).** Landing next, in
this order:

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
