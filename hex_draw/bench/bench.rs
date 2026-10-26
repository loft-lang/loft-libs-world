// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_draw-reference — the pure-Rust twin of the `hex_draw` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workload with the
// SAME arithmetic, in the same order, and prints the same row, hash included: a row whose
// hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports: `surface_fitted_spread` with `surface_of`, `surface_span` and the mean / perp
// helpers from src/hexsurf.loft; `draw_floor` from src/housedraw.loft (setup only); and what
// they stand on — `side_edges`, `plan_to_local`, `cell_x`/`cell_y` from hex_form, the
// `HexSet` window, `nb_q`/`nb_r`, `lattice_*` and `corner_*` from hex_field, and
// `hex_neighbor_dir`/`hex_edge_corners` from hex_grid.
//
// The twin follows the library's composition, sweep count included (bench/README.md rule 1:
// the library as written is what is measured): `surface_fitted_spread` asks `surface_of`, and
// `surface_span` asks `surface_of` again and `side_edges` once more — three sweeps of the
// window per call, as in src/hexsurf.loft.  `black_box` guards each op's INPUT and the sink,
// never a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const WIN: i64 = 61;
const HALF: i64 = 30;
const SIDES: i64 = 4;

const SQ3_2: f64 = 0.8660254037844386;
const HEX_LEN: f64 = 1.7320508075688772;
const EPS: f64 = 0.000000001;

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

// ── hex_field: the window, the lattice ──────────────────────────────

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

const CORNER_K: [i64; 6] = [0, -1, -1, 0, 1, 1];
const CORNER_M: [i64; 6] = [2, 1, -1, -2, -1, 1];

// ── hex_grid: the direction and its two corners ─────────────────────

fn hex_neighbor_dir(q1: i64, r1: i64, q2: i64, r2: i64) -> i64 {
    let d = (q2 - q1, r2 - r1);
    if (r1 & 1) == 0 {
        match d {
            (1, 0) => 0,
            (0, -1) => 1,
            (-1, -1) => 2,
            (-1, 0) => 3,
            (-1, 1) => 4,
            (0, 1) => 5,
            _ => -1,
        }
    } else {
        match d {
            (1, 0) => 0,
            (1, -1) => 1,
            (0, -1) => 2,
            (-1, 0) => 3,
            (0, 1) => 4,
            (1, 1) => 5,
            _ => -1,
        }
    }
}

fn hex_edge_corners(dir: i64) -> (usize, usize) {
    match dir {
        0 => (4, 5),
        1 => (3, 4),
        2 => (2, 3),
        3 => (1, 2),
        4 => (0, 1),
        _ => (5, 0),
    }
}

fn edge_corners_of(qa: i64, ra: i64, qb: i64, rb: i64) -> (usize, usize) {
    hex_edge_corners(hex_neighbor_dir(qa, ra, qb, rb))
}

// ── hex_form: the plan and one side's edges ─────────────────────────

struct Plan {
    cq: i64,
    cr: i64,
    wid: i64,
    dep: i64,
    rot: i64,
    mir: bool,
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

impl Plan {
    fn new(cq: i64, cr: i64, wid: i64, dep: i64, rot: i64, mir: bool) -> Plan {
        Plan { cq, cr, wid, dep, rot: ((rot % 6) + 6) % 6, mir }
    }
    fn hw(&self) -> f64 {
        self.wid as f64 * HEX_LEN * 0.5
    }
    fn hd(&self) -> f64 {
        self.dep as f64 * HEX_LEN * 0.5
    }
    fn to_local(&self, px: f64, py: f64) -> (f64, f64) {
        let co = rot_cos(self.rot);
        let si = rot_sin(self.rot);
        let s = if self.mir { -1.0 } else { 1.0 };
        let dx = px - cell_x(self.cq, self.cr);
        let dy = py - cell_y(self.cr);
        (dx * s * co + dy * s * si, -dx * si + dy * co)
    }
}

struct SideEdge {
    qa: i64,
    ra: i64,
    qb: i64,
    rb: i64,
    #[allow(dead_code)]
    t: f64,
}

/// The boundary edges lying on one side of the plan, in window scan order.
fn side_edges(p: &Plan, cells: &HexSet, side: i64) -> Vec<SideEdge> {
    let hw = p.hw();
    let hd = p.hd();
    let half = if side == 0 || side == 2 { hd } else { hw };
    let mut sgn = if side == 1 || side == 2 { -1.0 } else { 1.0 };
    if p.mir {
        sgn = -sgn;
    }
    let mut out = Vec::new();
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
                let (lu, lv) = p.to_local(mx, my);
                let ex_u = lu.abs() - hw;
                let ex_v = lv.abs() - hd;
                let (got, along) = if ex_u > ex_v {
                    (if lu > 0.0 { 0 } else { 2 }, lv)
                } else {
                    (if lv > 0.0 { 1 } else { 3 }, lu)
                };
                if got == side {
                    out.push(SideEdge { qa: eq, ra: er, qb: mq, rb: mr,
                                        t: 0.5 + sgn * along / (2.0 * half) });
                }
            }
        }
    }
    out
}

// ── hex_draw: the floor, and the surface read back ──────────────────

fn draw_floor(p: &Plan, cells: &mut HexSet) -> i64 {
    let hw = p.hw();
    let hd = p.hd();
    let mut n = 0;
    for fr in cells.r0..cells.r0 + cells.h {
        for fq in cells.q0..cells.q0 + cells.w {
            let (lu, lv) = p.to_local(cell_x(fq, fr), cell_y(fr));
            if lu.abs() <= hw + EPS && lv.abs() <= hd + EPS {
                cells.set(fq, fr, true);
                n += 1;
            }
        }
    }
    n
}

struct WallSurface {
    dk: i64,
    dm: i64,
    num_k: i64,
    num_m: i64,
    den: i64,
}

fn surface_of(p: &Plan, cells: &HexSet, side: i64) -> WallSurface {
    let run = side_edges(p, cells, side);
    let (mut dk, mut dm, mut mk, mut mm) = (0i64, 0i64, 0i64, 0i64);
    for e in &run {
        let (c1, c2) = edge_corners_of(e.qa, e.ra, e.qb, e.rb);
        dk += CORNER_K[c2] - CORNER_K[c1];
        dm += CORNER_M[c2] - CORNER_M[c1];
        mk += 2 * lattice_k(e.qa, e.ra) + CORNER_K[c1] + CORNER_K[c2];
        mm += 2 * lattice_m(e.ra) + CORNER_M[c1] + CORNER_M[c2];
    }
    let n = run.len() as i64;
    WallSurface { dk, dm, num_k: mk, num_m: mm, den: 2 * n }
}

impl WallSurface {
    fn mean_x(&self) -> f64 {
        (self.num_k as f64 / self.den as f64) * 0.8660254037844386
    }
    fn mean_y(&self) -> f64 {
        (self.num_m as f64 / self.den as f64) * 0.5
    }
    fn dir(&self) -> (f64, f64, f64) {
        let wx = self.dk as f64 * 0.8660254037844386;
        let wy = self.dm as f64 * 0.5;
        (wx, wy, (wx * wx + wy * wy).sqrt())
    }
    fn perp(&self) -> (f64, f64) {
        let (wx, wy, ln) = self.dir();
        (0.0 - wy / ln, wx / ln)
    }
}

fn surface_span(p: &Plan, cells: &HexSet, side: i64) -> (f64, f64, f64, f64) {
    let w = surface_of(p, cells, side);
    let run = side_edges(p, cells, side);
    let (dx, dy, dl) = w.dir();
    let ux = dx / dl;
    let uy = dy / dl;
    let cx = w.mean_x();
    let cy = w.mean_y();
    let mut lo = 1.0e9;
    let mut hi = -1.0e9;
    for e in &run {
        let (c1, c2) = edge_corners_of(e.qa, e.ra, e.qb, e.rb);
        for cc in [c1, c2] {
            let ex = (lattice_k(e.qa, e.ra) + CORNER_K[cc]) as f64 * 0.8660254037844386;
            let ey = (lattice_m(e.ra) + CORNER_M[cc]) as f64 * 0.5;
            let tv = (ex - cx) * ux + (ey - cy) * uy;
            if tv < lo {
                lo = tv;
            }
            if tv > hi {
                hi = tv;
            }
        }
    }
    (cx + lo * ux, cy + lo * uy, cx + hi * ux, cy + hi * uy)
}

fn surface_fitted_spread(p: &Plan, cells: &HexSet, side: i64) -> f64 {
    let w = surface_of(p, cells, side);
    let (sx0, sy0, sx1, sy1) = surface_span(p, cells, side);
    let (px, py) = w.perp();
    let cx = w.mean_x();
    let cy = w.mean_y();
    let e0 = (sx0 - cx) * px + (sy0 - cy) * py;
    let e1 = (sx1 - cx) * px + (sy1 - cy) * py;
    e0.max(e1) - e0.min(e1)
}

// ── The row ─────────────────────────────────────────────────────────

fn plan_at(dq: i64) -> Plan {
    Plan::new(dq, 0, 24, 20, 1, false)
}

fn footprint(p: &Plan) -> HexSet {
    let mut c = HexSet::chunk(-HALF, -HALF, WIN, WIN);
    draw_floor(p, &mut c);
    c
}

fn spread_op(p: &Plan, cells: &HexSet) -> Vec<i64> {
    let mut out = Vec::with_capacity(2 * SIDES as usize);
    for side in 0..SIDES {
        let sp = surface_fitted_spread(p, cells, side);
        out.push((sp * 1000000.0) as i64);
        out.push((sp * 1000000000000000000.0) as i64);
    }
    out
}

fn bench_spread(n: i64) -> Row {
    let p0 = plan_at(0);
    let p1 = plan_at(1);
    let c0 = footprint(&p0);
    let c1 = footprint(&p1);
    let (us, sink) = timed(n, |r| {
        if (r & 1) == 0 { spread_op(&p0, &c0)[1] } else { spread_op(&p1, &c1)[1] }
    });
    let mut out = spread_op(black_box(&p0), &c0);
    out.push(c0.count);
    out.push(c1.count);
    Row { name: "surface_fitted_spread", iters: n, us, px: SIDES * WIN * WIN,
          hash: fnv(FNV_OFFSET, &out), sink }
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
    println!("routine\titers\tus\tns_op\tpx\tns_px\thash");
    let rows = [bench_spread(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
