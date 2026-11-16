<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# hex_ground — the ground structures stand on, and the routines that shape it

The ground of one **place**, authored: a height and a surface kind per cell (`hex_field`'s
`Heights` and `Labels` over a window), and five ways to shape it — terraces made smooth, a round
hill, a structure seated, a structure dug in, and cliffs where a front must not slope.

It is not `hex_terrain`. That package *generates* land: an overland lattice, hydrology, rivers, a
surface sampled anywhere. This one shapes the ground of a town somebody drew.

Part of the `hex_*` family beside `hex_field` (the heights and labels it writes), `hex_draw` (the
straight sides a bank and a cliff run along) and `hex_place` (the seat).

## Using it

Add the dependency and `use hex_ground;`. [`USAGE.md`](USAGE.md) is the map of the surface, with
every claim a passing assertion in `tests/01-hex-ground.loft`.

**Units.** A ground carries its `step`: the distance between two neighbouring cell centres in your
unit, which is the unit heights are in too. `sqrt(3)` is the lattice's own world unit; `1.5` is a
ground in metres where a hex step is 1.5 m. Every distance you pass — a hill's radius, a bank's
width, a cliff area's box — is in that unit.

**Straight, not zigzag.** A bank and a cliff follow `hex_draw::cells_outlines` of the cells
concerned: the same straight lines a road's edge or a house's wall is drawn along. So a stream's
bank does not wobble with the cells, and a cliff stands where the terrace's edge is drawn.

Extracted from tile-world-game, whose town it was checked on: the whole place regenerates byte
for byte from these routines. Its rules for the ground (`HG-…` in its `formal/hex-houses.md`)
are cited by tag from the routines that keep them.
