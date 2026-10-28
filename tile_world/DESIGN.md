<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# tile_world — the game design

This is what the game built on tile_world should be.
- What is shipped is in [README.md](README.md).
- This file is the target: every rule here is either built (with a test) or open.
- Each rule names the slice that builds it.

## The principle: a dungeon is a puzzle, not a grind

- **Puzzles come first.** A dungeon should feel like one big puzzle. Nobody should get through it by
  grinding until the character is strong enough to trivialise it.
- **Growth comes later.** Character growth comes only after a base dungeon and world feel alive and
  are satisfying to beat. Growth means carried weapons, worn armour and stat increases.
- **Growth is never necessary.** Correct puzzle solving and fighting strategy must always let the
  base character beat every dungeon. It may get increasingly harder without upgrades.
- **A test enforces it.** Every shipped level carries a solution test played by the base character
  (`tests/first_climb.loft` is the model).

## The world

- **Layers.** `z` is the floor level, unbounded both ways.
  - Each layer is an unbounded grid of sparse 32x32 chunks.
  - Terrain is 2 bytes a cell; movable things are per-cell stacks (slice 1).
- **Heights.** Height counts in steps.
  - A layer is 4 steps: the height of a wall.
  - A chair is 1 and a table 2, so a chair on a table is 3.
  - The hero steps up 1 and down any amount.
  - So a chair climbs a table, and a chair on a table climbs a wall: the wall's top is the next
    layer's floor.
  - Slice 1 builds this with a 3-step layer; slice 2 makes it 4.
- **Falling.** Falling a full wall height (4) costs the hero health. A bomb that drops a level
  explodes on landing.
- **Ceilings.** A cell has a ceiling when the layer above it is not void.
  - The ceiling is never drawn.
  - The hero cannot climb up through a ceiling, and neither can a spider.
- **Terrain kinds.**
  - Floor and rails.
  - Water: blocks walking, but bats fly over it.
  - Wall: rough walls are climbable by spiders; smooth walls (optional) are not.
  - Fence: the hero climbs over it like a height-1 step. Nothing is pushed or pulled across it,
    and it stops trains. Bats fly over it.
  - Tree: blocks walking.
  - Door.
  - Well.

## Moving things

- **Push and pull.** Walking pushes; `pull` drags what is behind the hero (slice 1).
- **Carts.**
  - Carts run on rails, and a line of carts is a train.
  - The base strength pushes 3 carts and pulls 2.
  - A green potion gives +2 to both for 60 s of the world clock. It is used by walking over it
    (slice 1).
- **Rocks and anvils.** Pushed only, never pulled, and never by a cart or another rock. On the
  rails they block trains (slice 1).
- **Chests.**
  - A chest is pushed like a rock: pushed only, never pulled, never by a cart or another
    rock, and on the rails it blocks trains.
  - It is climbed like a table (height 2), but nothing stands on it: a chair is not lifted
    onto it.
- **Chairs.**
  - A chair slides on the floor or along tables.
  - It is pulled up onto a table the hero stands on (slice 1).
  - A bomb breaks it.

## What the hero holds

- **One item, no inventory.** The hero holds at most one item, and it is always on screen.
  - Walking over an item swaps it with the one held.
  - `drop` puts the held item down.
- **Sword.** The hero fights with it. While holding it, the hero cannot push, pull or lift
  anything.
- **Axe.** Walking into a fence or a tree with it chops it down to floor.
- **Bow.**
  - Shoots enemies higher up on walls (bats, spiders), with no ammunition limit.
  - **Aim.** The aim locks onto an enemy in the general direction the hero faces. The lock is
    lost when that enemy goes out of the hero's view.
- **Bomb.**
  - It is thrown like a bowling ball: it rolls in a straight line and ignites on impact.
  - **The blast.**
    - It moves every movable thing beside it one cell away.
    - It breaks chairs and other fragile things.
    - It damages enemies.

## Goals: chests and points

- **A point per chest.**  Opening a chest gives its money and one point.  A region or a
  dungeon has a fixed number of chests, so the points it can give are limited.
- **A chest is never hidden.**  It may stand out of the direct route — a door behind a tree is
  a fair way to it — but never behind or beside a wall where it cannot be seen.
- **The signpost counts.**  Walking into a farm signpost lists how many of the region's chests
  are still unopened.

## Enemies

- **Kinds.** Rats, bats and spiders.
  - Walking into an enemy attacks it.
  - The hero has hearts, and losing them all returns the hero to the last safe point.
- **They do not fight to the death.**
  - A hurt enemy that is not cornered retreats to its nest and heals there over time.
  - Only a cornered enemy fights on and can be finished off.
  - **Cornered** means it has no step away from the hero and no path home that avoids the hero.
- **Spiders.**
  - They climb rough walls, and they flee to another level through openings (not through a
    ceiling).
  - Their nest is never on a wall.
- **Enemies act on the world clock**, so every behaviour is testable at exact times.

## Guards, doors and wells

- **The clear area.** Guards and wells share one rule: no enemy is within 10 cells in sight. A
  fleeing enemy that is still in sight counts.
- **Guards.** A guard blocks a door by standing in front of it. It steps aside when the hero
  approaches and the area is clear.
- **Wells and safe points.** Drinking from a well saves the whole situation.
  - A well gives water only when the area is clear.
  - A key returns to the saved situation, after a confirmation.
  - This is what makes the unsolvable states a wrong push or a bomb can create acceptable.

## What the player sees

- **The hero's position decides what is drawn**, never what is on screen.
- **Up.**
  - In a low corridor (under a ceiling) only the hero's own layer is drawn, even beside a taller
    room.
  - Stepping into a room with no ceiling draws the layer above it as well. The view itself tells
    the player they stand in a higher room.
- **Down.**
  - Standing one cell back from a ledge, the drop shows as plain dark.
  - Standing beside a pit, the layers below are drawn, darker with each level, up to about 4 deep.
- **Drawing.** It uses the `tiles` library's shaded palettes (100 / 75 / 50 / 35 %).

## Slices

| slice | content | state |
|---|---|---|
| 1 | layers and chunks, walls, rails, carts and trains, rocks, chairs and tables, potion, demo level | shipped (0.1.0) |
| 2 | 4-step layers and falls, ceilings, fence, tree, one held item (sword, axe), chests and the signpost, enemies that flee and heal, guards and doors, wells and safe points | open |
| 3 | bow and aim, bomb, the depth renderer with Kenney tiles | open |
| later | growth: carried weapons, armour, stats — never required (the principle above) | open |
