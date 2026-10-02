// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_form-reference — the pure-Rust twin of the `hex_form` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports: `form_write`, `form_read`, `side_edges` and `boundary_loops` from src/, and what
// they stand on — `HexSet`, `nb_q`/`nb_r`, `lattice_k`/`lattice_m`, `corner_k`/`corner_m`
// and `hex_dist` from hex_field, `hex_neighbor` and `hex_edge_corners` from hex_grid.
// `form_write` is `write!` per side; `form_read` is the library's reader with its word helpers
// ported as they are: each one splits the whole line again, collected, for the one word it
// reads, and the checks run in the library's order on the same fields.
// `black_box` guards each op's INPUT (the repetition number) and the sink — never anything
// inside a kernel.
//
// FOR THE LIBRARY'S AUTHOR.  Bench rule 1 — the same algorithm in every lane — makes the twin
// pay for work an idiomatic implementation would not do.  One row used to skip it (the line
// split once), and it then charged loft for the library's algorithm.
//   `form_read`   `nth_word`, `word_int`, `word_is_int` and `word_count` each `split(' ')`
//                 the line and build the whole word vector for the ONE word they answer:
//                 5 splits on the header and 10 per `side` line (6 words), where one split
//                 per line serves every field.
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const POOL: i64 = 2000;
const WIN: i64 = 61;
const RING: i64 = 17;
const PASSES: i64 = 10;

const HEAD_N: i64 = 12;
const HEX_LEN: f64 = 1.7320508075688772;
const SQ3_2: f64 = 0.8660254037844386;

fn fnv(h0: i64, v: &[i64]) -> i64 {
    let mut h = h0;
    for &x in v {
        let w = x & 0xFFFF_FFFF;
        for sh in [24, 16, 8, 0] {
            h = ((h ^ ((w >> sh) & 255)) * FNV_PRIME) & 0xFFFF_FFFF;
        }
    }
    h
}

fn fnv_text(h0: i64, t: &str) -> i64 {
    let mut h = h0;
    for &b in t.as_bytes() {
        h = ((h ^ b as i64) * FNV_PRIME) & 0xFFFF_FFFF;
    }
    h
}

struct Row {
    name: &'static str,
    iters: i64,
    us: i64,
    px: i64,
    hash: i64,
    sink: i64,
}

fn print_row(r: &Row) {
    let ns_op = r.us * 1000 / r.iters;
    let ns_px = if r.px > 0 { (r.us * 1000) as f64 / (r.iters * r.px) as f64 } else { 0.0 };
    println!("{}\t{}\t{}\t{}\t{}\t{:.3}\t{:x}", r.name, r.iters, r.us, ns_op, r.px, ns_px, r.hash);
}

fn timed<F: FnMut(i64) -> i64>(n: i64, mut f: F) -> (i64, i64) {
    let t0 = Instant::now();
    let mut sink = 0i64;
    for r in 0..n {
        sink = sink.wrapping_add(f(black_box(r)));
    }
    (t0.elapsed().as_micros() as i64, black_box(sink))
}

// ── hex_field / hex_grid: cells and the lattice ─────────────────────

struct HexSet {
    q0: i64,
    r0: i64,
    w: i64,
    h: i64,
    cells: Vec<bool>,
    count: i64,
}

impl HexSet {
    fn chunk(q0: i64, r0: i64, w: i64, h: i64) -> HexSet {
        HexSet { q0, r0, w, h, cells: vec![false; (w * h) as usize], count: 0 }
    }
    fn index(&self, q: i64, r: i64) -> i64 {
        let dq = q - self.q0;
        let dr = r - self.r0;
        if dq < 0 || dq >= self.w || dr < 0 || dr >= self.h {
            return -1;
        }
        dr * self.w + dq
    }
    fn get(&self, q: i64, r: i64) -> bool {
        let i = self.index(q, r);
        i >= 0 && self.cells[i as usize]
    }
    fn set(&mut self, q: i64, r: i64, on: bool) {
        let i = self.index(q, r);
        if i < 0 {
            return;
        }
        let had = self.cells[i as usize];
        self.cells[i as usize] = on;
        if on && !had {
            self.count += 1;
        }
        if !on && had {
            self.count -= 1;
        }
    }
}

fn lattice_k(q: i64, r: i64) -> i64 {
    2 * q + (r & 1)
}

fn lattice_m(r: i64) -> i64 {
    3 * r
}

fn nb_q(q: i64, r: i64, d: i64) -> i64 {
    let odd = r & 1 == 1;
    match d {
        0 => q + 1,
        1 => q - 1,
        2 | 4 => if odd { q + 1 } else { q },
        _ => if odd { q } else { q - 1 },
    }
}

fn nb_r(r: i64, d: i64) -> i64 {
    match d {
        0 | 1 => r,
        2 | 3 => r - 1,
        _ => r + 1,
    }
}

fn hex_neighbor(q: i64, r: i64, dir: i64) -> (i64, i64) {
    if r & 1 == 0 {
        match dir {
            0 => (q + 1, r),
            1 => (q, r - 1),
            2 => (q - 1, r - 1),
            3 => (q - 1, r),
            4 => (q - 1, r + 1),
            _ => (q, r + 1),
        }
    } else {
        match dir {
            0 => (q + 1, r),
            1 => (q + 1, r - 1),
            2 => (q, r - 1),
            3 => (q - 1, r),
            4 => (q, r + 1),
            _ => (q + 1, r + 1),
        }
    }
}

fn hex_edge_corners(dir: i64) -> (i64, i64) {
    match dir {
        0 => (4, 5),
        1 => (3, 4),
        2 => (2, 3),
        3 => (1, 2),
        4 => (0, 1),
        _ => (5, 0),
    }
}

fn corner_k(i: i64) -> i64 {
    [0, -1, -1, 0, 1, 1][i as usize]
}

fn corner_m(i: i64) -> i64 {
    [2, 1, -1, -2, -1, 1][i as usize]
}

fn hex_dist(q1: i64, r1: i64, q2: i64, r2: i64) -> i64 {
    let x1 = q1 - (r1 - (r1 & 1)) / 2;
    let z1 = r1;
    let y1 = -x1 - z1;
    let x2 = q2 - (r2 - (r2 & 1)) / 2;
    let z2 = r2;
    let y2 = -x2 - z2;
    (x1 - x2).abs().max((y1 - y2).abs()).max((z1 - z2).abs())
}

fn hexdisk_into(s: &mut HexSet, cq: i64, cr: i64, n: i64) {
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if hex_dist(cq, cr, q, r) <= n {
                s.set(q, r, true);
            }
        }
    }
}

// ── hexform.loft / formtext.loft: the form and its text ─────────────

struct Form {
    h0: i64,
    lens: Vec<i64>,
    turns: Vec<i64>,
}

fn head_norm(h: i64) -> i64 {
    ((h % HEAD_N) + HEAD_N) % HEAD_N
}

fn form_new(h0: i64, lens: Vec<i64>, turns: Vec<i64>) -> Form {
    Form { h0: head_norm(h0), lens, turns }
}

fn form_write(f: &Form, name: &str) -> String {
    let mut s = String::new();
    write!(s, "stencil {} h0 {}", name, f.h0).unwrap();
    for (i, (l, t)) in f.lens.iter().zip(&f.turns).enumerate() {
        write!(s, "\nside {} len {} turn {}", i, l, t).unwrap();
    }
    s
}

/// Word `i` of a line, the library's way: the whole line split and collected for one word.
fn nth_word(line: &str, i: usize) -> &str {
    let parts: Vec<&str> = line.split(' ').collect();
    parts.get(i).copied().unwrap_or("")
}

fn word_int(line: &str, i: usize) -> i64 {
    nth_word(line, i).parse().unwrap_or(0)
}

/// How many space-separated fields the line has — a split, collected, for its length.
fn word_count(line: &str) -> usize {
    line.split(' ').collect::<Vec<&str>>().len()
}

/// Is field `i` an integer spelled exactly the way `form_write` spells one?
fn word_is_int(line: &str, i: usize) -> bool {
    let w = nth_word(line, i);
    let v: i64 = w.parse().unwrap_or(0);
    v.to_string() == w
}

fn refused() -> Form {
    form_new(0, Vec::new(), Vec::new())
}

/// The strict reader, its checks in the library's order, each through the word helpers above.
fn form_read(t: &str) -> Form {
    let lines: Vec<&str> = t.split('\n').collect();
    if lines.len() < 2 {
        return refused();
    }
    let head = lines[0];
    if word_count(head) != 4 {
        return refused();
    }
    if nth_word(head, 0) != "stencil" {
        return refused();
    }
    if nth_word(head, 2) != "h0" {
        return refused();
    }
    if !word_is_int(head, 3) {
        return refused();
    }
    let h0 = word_int(head, 3);
    if head_norm(h0) != h0 {
        return refused();
    }
    let mut lens = Vec::new();
    let mut turns = Vec::new();
    for (ri, ln) in lines.iter().enumerate().skip(1) {
        if word_count(ln) != 6 {
            return refused();
        }
        if nth_word(ln, 0) != "side" {
            return refused();
        }
        if nth_word(ln, 2) != "len" {
            return refused();
        }
        if nth_word(ln, 4) != "turn" {
            return refused();
        }
        if !word_is_int(ln, 1) {
            return refused();
        }
        if !word_is_int(ln, 3) {
            return refused();
        }
        if !word_is_int(ln, 5) {
            return refused();
        }
        if word_int(ln, 1) != ri as i64 - 1 {
            return refused();
        }
        lens.push(word_int(ln, 3));
        turns.push(word_int(ln, 5));
    }
    form_new(h0, lens, turns)
}

// ── the plan and its sides ──────────────────────────────────────────

struct Plan {
    cq: i64,
    cr: i64,
    wid: i64,
    dep: i64,
    rot: i64,
    mir: bool,
}

fn plan_new(cq: i64, cr: i64, wid: i64, dep: i64, rot: i64, mir: bool) -> Plan {
    Plan { cq, cr, wid, dep, rot: ((rot % 6) + 6) % 6, mir }
}

fn cell_x(q: i64, r: i64) -> f64 {
    lattice_k(q, r) as f64 * SQ3_2
}

fn cell_y(r: i64) -> f64 {
    lattice_m(r) as f64 * 0.5
}

fn rot_cos(n: i64) -> f64 {
    [1.0, 0.5, -0.5, -1.0, -0.5, 0.5][n as usize]
}

fn rot_sin(n: i64) -> f64 {
    [0.0, SQ3_2, SQ3_2, 0.0, -SQ3_2, -SQ3_2][n as usize]
}

fn plan_to_local(p: &Plan, px: f64, py: f64) -> (f64, f64) {
    let co = rot_cos(p.rot);
    let si = rot_sin(p.rot);
    let s = if p.mir { -1.0 } else { 1.0 };
    let dx = px - cell_x(p.cq, p.cr);
    let dy = py - cell_y(p.cr);
    (dx * s * co + dy * s * si, -dx * si + dy * co)
}

#[derive(Default)]
struct SideRun {
    qa: Vec<i64>,
    ra: Vec<i64>,
    qb: Vec<i64>,
    rb: Vec<i64>,
    t: Vec<f64>,
}

fn side_edges(p: &Plan, cells: &HexSet, side: i64) -> SideRun {
    let hw = p.wid as f64 * HEX_LEN * 0.5;
    let hd = p.dep as f64 * HEX_LEN * 0.5;
    let half = if side == 0 || side == 2 { hd } else { hw };
    let mut sgn = if side == 1 || side == 2 { -1.0 } else { 1.0 };
    if p.mir {
        sgn = -sgn;
    }
    let mut run = SideRun::default();
    for er in cells.r0..cells.r0 + cells.h {
        for eq in cells.q0..cells.q0 + cells.w {
            if !cells.get(eq, er) {
                continue;
            }
            for ed in 0..6 {
                let mq = nb_q(eq, er, ed);
                let mr = nb_r(er, ed);
                if cells.get(mq, mr) {
                    continue;
                }
                let mx = (cell_x(eq, er) + cell_x(mq, mr)) * 0.5;
                let my = (cell_y(er) + cell_y(mr)) * 0.5;
                let (lu, lv) = plan_to_local(p, mx, my);
                let ex_u = lu.abs() - hw;
                let ex_v = lv.abs() - hd;
                let (got, along) = if ex_u > ex_v {
                    (if lu > 0.0 { 0 } else { 2 }, lv)
                } else {
                    (if lv > 0.0 { 1 } else { 3 }, lu)
                };
                if got == side {
                    run.qa.push(eq);
                    run.ra.push(er);
                    run.qb.push(mq);
                    run.rb.push(mr);
                    run.t.push(0.5 + sgn * along / (2.0 * half));
                }
            }
        }
    }
    run
}

// ── boundary_loops ──────────────────────────────────────────────────

fn vkey(k: i64, m: i64) -> i64 {
    (k + 500) * 2000 + (m + 500)
}

fn bedge(q: i64, r: i64, c: i64) -> i64 {
    vkey(lattice_k(q, r) + corner_k(c), lattice_m(r) + corner_m(c))
}

fn boundary_loops(cells: &HexSet) -> i64 {
    let mut ea = Vec::new();
    let mut eb = Vec::new();
    for lr in cells.r0..cells.r0 + cells.h {
        for lq in cells.q0..cells.q0 + cells.w {
            if !cells.get(lq, lr) {
                continue;
            }
            for ld in 0..6 {
                let (nq, nr) = hex_neighbor(lq, lr, ld);
                if !cells.get(nq, nr) {
                    let (c1, c2) = hex_edge_corners(ld);
                    ea.push(bedge(lq, lr, c1));
                    eb.push(bedge(lq, lr, c2));
                }
            }
        }
    }
    let ne = ea.len();
    if ne == 0 {
        return 0;
    }
    let mut comp: Vec<i64> = (0..ne as i64).collect();
    let mut changed = true;
    while changed {
        changed = false;
        for pi in 0..ne {
            for pj in 0..ne {
                if pi != pj {
                    let shares =
                        ea[pi] == ea[pj] || ea[pi] == eb[pj] || eb[pi] == ea[pj] || eb[pi] == eb[pj];
                    if shares && comp[pj] < comp[pi] {
                        comp[pi] = comp[pj];
                        changed = true;
                    }
                }
            }
        }
    }
    let mut nloops = 0;
    for qi in 0..ne {
        if !comp[..qi].contains(&comp[qi]) {
            nloops += 1;
        }
    }
    nloops
}

// ── The rows ────────────────────────────────────────────────────────

fn form_pool() -> Vec<Form> {
    let mut out = Vec::new();
    let mut s: i64 = 20260924;
    let next = |s: &mut i64| {
        *s = (*s * 1103515245 + 12345) & 0x7FFF_FFFF;
        *s >> 16
    };
    for _ in 0..POOL {
        let sides = 3 + next(&mut s) % 4;
        let h0 = next(&mut s) % 12;
        let mut lens = Vec::new();
        let mut turns = Vec::new();
        for _ in 0..sides {
            lens.push(1 + next(&mut s) % 9);
            turns.push(1 + next(&mut s) % 11);
        }
        out.push(form_new(h0, lens, turns));
    }
    out
}

fn pool_name(r: i64) -> &'static str {
    if r & 1 == 0 { "towa" } else { "towb" }
}

fn write_op(pool: &[Form], r: i64) -> i64 {
    let name = pool_name(r);
    pool.iter().map(|f| form_write(f, name).len() as i64).sum()
}

fn bench_write(n: i64, pool: &[Form]) -> Row {
    let (us, sink) = timed(n, |r| write_op(pool, r));
    let hash = pool.iter().fold(FNV_OFFSET, |h, f| fnv_text(h, &form_write(f, pool_name(black_box(0)))));
    Row { name: "form_write", iters: n, us, px: POOL, hash, sink }
}

fn read_op(texts: &[String]) -> i64 {
    texts
        .iter()
        .map(|t| {
            let f = form_read(t);
            f.h0 + f.lens.len() as i64
        })
        .sum()
}

fn read_hash(texts: &[String]) -> i64 {
    let mut v = Vec::new();
    for t in texts {
        let f = form_read(t);
        v.push(f.h0);
        v.push(f.lens.len() as i64);
        v.extend_from_slice(&f.lens);
        v.extend_from_slice(&f.turns);
    }
    fnv(FNV_OFFSET, &v)
}

fn bench_read(n: i64, pool: &[Form]) -> Row {
    let a: Vec<String> = pool.iter().map(|f| form_write(f, "towa")).collect();
    let b: Vec<String> = pool.iter().map(|f| form_write(f, "towb")).collect();
    let (us, sink) = timed(n, |r| read_op(if r & 1 == 0 { &a } else { &b }));
    Row { name: "form_read", iters: n, us, px: POOL, hash: read_hash(black_box(&a)), sink }
}

fn side_cells() -> HexSet {
    let mut s = HexSet::chunk(-30, -30, WIN, WIN);
    hexdisk_into(&mut s, 0, 0, 24);
    s
}

fn sides_op(cells: &HexSet, r: i64) -> i64 {
    let mut n = 0;
    for pass in 0..PASSES {
        let p = plan_new(0, 0, 20, 16, (r + pass) % 6, false);
        for side in 0..4 {
            n += side_edges(&p, cells, side).t.len() as i64;
        }
    }
    n
}

fn sides_hash(cells: &HexSet) -> i64 {
    let p = plan_new(0, 0, 20, 16, black_box(0), false);
    let mut v = Vec::new();
    for side in 0..4 {
        let run = side_edges(&p, cells, side);
        for i in 0..run.t.len() {
            v.extend_from_slice(&[run.qa[i], run.ra[i], run.qb[i], run.rb[i], (run.t[i] * 1000000.0) as i64]);
        }
    }
    fnv(FNV_OFFSET, &v)
}

fn bench_sides(n: i64) -> Row {
    let cells = side_cells();
    let (us, sink) = timed(n, |r| sides_op(&cells, r));
    Row { name: "side_edges", iters: n, us, px: PASSES * 4 * WIN * WIN, hash: sides_hash(&cells), sink }
}

fn ring(cq: i64) -> HexSet {
    let mut s = HexSet::chunk(0, 0, 44, 42);
    for r in 0..42 {
        for q in 0..44 {
            if hex_dist(cq, 20, q, r) == RING {
                s.set(q, r, true);
            }
        }
    }
    s
}

fn ring_edges(s: &HexSet) -> i64 {
    let mut n = 0;
    for r in 0..42 {
        for q in 0..44 {
            if s.get(q, r) {
                for d in 0..6 {
                    let (nq, nr) = hex_neighbor(q, r, d);
                    if !s.get(nq, nr) {
                        n += 1;
                    }
                }
            }
        }
    }
    n
}

fn bench_loops(n: i64) -> Row {
    let s0 = ring(20);
    let s1 = ring(21);
    let (us, sink) = timed(n, |r| boundary_loops(if r & 1 == 0 { &s0 } else { &s1 }));
    Row { name: "boundary_loops", iters: n, us, px: ring_edges(&s0),
          hash: fnv(FNV_OFFSET, &[boundary_loops(black_box(&s0)), s0.count]), sink }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut n: i64 = 20;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--n" && i + 1 < args.len() {
            n = args[i + 1].parse().unwrap_or(20);
            i += 1;
        }
        i += 1;
    }
    if n < 1 {
        n = 1;
    }
    let t0 = Instant::now();
    let pool = form_pool();
    println!("routine\titers\tus\tns_op\tpx\tns_px\thash");
    let rows = [bench_write(n, &pool), bench_read(n, &pool), bench_sides(n), bench_loops(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
