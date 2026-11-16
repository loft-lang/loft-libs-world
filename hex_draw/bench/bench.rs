// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_draw-reference — the pure-Rust twin of the `hex_draw` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workload with the
// SAME arithmetic, in the same order, and prints the same rows, hash included: a row whose
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
// And the walls from cells: `cells_built` and `cells_outlines` from src/cellwalls.loft with all
// they stand on there (the runs, the two wall solvers, the miter, the step and the slant), on
// hex_field's `trace_edges` and its edge layer.  The boundary is walked straight into edges with
// their corners, where the library walks it into an `EdgeMap` and then into `WallEdge`s.
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

#[derive(Clone, Copy)]
struct WallSurface {
    dk: i64,
    dm: i64,
    num_k: i64,
    num_m: i64,
    den: i64,
    n: i64,
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
    WallSurface { dk, dm, num_k: mk, num_m: mm, den: 2 * n, n }
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

// ── hex_field: the edge layer, and trace_edges ──────────────────────

const EDGE_MAT_MAX: i64 = 255;

struct EdgeSet {
    q0: i64,
    r0: i64,
    gw: i64,
    gh: i64,
    mat: Vec<u8>,
    surf: Vec<i32>,
}

impl EdgeSet {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> EdgeSet {
        let (gw, gh) = (w + 2, h + 2);
        let n = (gw * gh * 3) as usize;
        EdgeSet { q0, r0, gw, gh, mat: vec![0; n], surf: vec![0; n] }
    }

    /// The storage slot for the edge between two adjacent cells, or -1.
    fn index(&self, qa: i64, ra: i64, qb: i64, rb: i64) -> i64 {
        let mut d = -1;
        for i in 0..6 {
            if nb_q(qa, ra, i) == qb && nb_r(ra, i) == rb {
                d = i;
            }
        }
        if d < 0 {
            return -1;
        }
        let (cq, cr, slot) = match d {
            2 => (qa, ra, 1),
            3 => (qa, ra, 2),
            1 => (qb, rb, 0),
            5 => (qb, rb, 1),
            4 => (qb, rb, 2),
            _ => (qa, ra, 0),
        };
        let dq = cq - self.q0 + 1;
        let dr = cr - self.r0 + 1;
        if dq < 0 || dq >= self.gw || dr < 0 || dr >= self.gh {
            return -1;
        }
        (dr * self.gw + dq) * 3 + slot
    }

    fn set_mat(&mut self, qa: i64, ra: i64, qb: i64, rb: i64, mat: i64) {
        let i = self.index(qa, ra, qb, rb);
        if i >= 0 && (0..=EDGE_MAT_MAX).contains(&mat) {
            self.mat[i as usize] = mat as u8;
        }
    }

    fn count_all(&self) -> i64 {
        self.mat.iter().filter(|&&m| m != 0).count() as i64
    }

    fn digest(&self) -> i64 {
        let mut h: i64 = 1469598103;
        for (&m, &s) in self.mat.iter().zip(&self.surf) {
            h = (h * 31 + m as i64) % 1000000007;
            h = (h * 31 + s as i64) % 1000000007;
        }
        h
    }
}

fn edge_of(dk: i64, dm: i64) -> i64 {
    match (dk, dm) {
        (-1, 3) => 0,
        (-2, 0) => 1,
        (-1, -3) => 2,
        (1, -3) => 3,
        (2, 0) => 4,
        (1, 3) => 5,
        _ => -1,
    }
}

fn lattice_to_cell(k: i64, m: i64) -> (i64, i64) {
    let lr = m / 3;
    ((k - (lr & 1)) / 2, lr)
}

// The inverse of `edge_of`: the lattice step to the neighbour across edge i.
const EDGE_DK: [i64; 6] = [-1, -2, -1, 1, 2, 1];
const EDGE_DM: [i64; 6] = [3, 0, -3, -3, 0, 3];

/// One edge of a boundary: the structure's cell, the outside one, and its corners from a to b.
#[derive(Clone, Copy)]
struct WallEdge {
    qa: i64,
    ra: i64,
    qb: i64,
    rb: i64,
    ak: i64,
    am: i64,
    bk: i64,
    bm: i64,
}

// `cells_loops` on `trace_edges`: the walk a step at a time, each loop from its smallest corner.
fn cells_loops(s: &HexSet) -> Vec<Vec<WallEdge>> {
    let mut used = vec![0u8; (s.w * s.h) as usize];
    let mut out: Vec<Vec<WallEdge>> = Vec::new();
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if !s.cells[((r - s.r0) * s.w + (q - s.q0)) as usize] {
                continue;
            }
            for d in 0..6 {
                let nq = nb_q(q, r, d);
                let nr = nb_r(r, d);
                if s.get(nq, nr) {
                    continue;
                }
                let i0 = edge_of(lattice_k(nq, nr) - lattice_k(q, r), lattice_m(nr) - lattice_m(r));
                if i0 < 0 || used[((r - s.r0) * s.w + (q - s.q0)) as usize] & (1 << i0) != 0 {
                    continue;
                }
                // (corner k, corner m, cell, outside cell)
                let mut lp: Vec<[i64; 6]> = Vec::new();
                let (mut cq, mut cr, mut ci) = (q, r, i0);
                loop {
                    let at = ((cr - s.r0) * s.w + (cq - s.q0)) as usize;
                    if used[at] & (1 << ci) != 0 {
                        break;
                    }
                    used[at] |= 1 << ci;
                    let k0 = lattice_k(cq, cr);
                    let m0 = lattice_m(cr);
                    let (bq, br) = lattice_to_cell(k0 + EDGE_DK[ci as usize], m0 + EDGE_DM[ci as usize]);
                    lp.push([k0 + CORNER_K[ci as usize], m0 + CORNER_M[ci as usize], cq, cr, bq, br]);
                    let ni = (ci + 1) % 6;
                    let (tq, tr) = lattice_to_cell(k0 + EDGE_DK[ni as usize], m0 + EDGE_DM[ni as usize]);
                    if s.get(tq, tr) {
                        ci = edge_of(lattice_k(bq, br) - lattice_k(tq, tr), lattice_m(br) - lattice_m(tr));
                        cq = tq;
                        cr = tr;
                    } else {
                        ci = ni;
                    }
                }
                let n = lp.len();
                let mut best = 0;
                for j in 1..n {
                    if lp[j][0] < lp[best][0] || (lp[j][0] == lp[best][0] && lp[j][1] < lp[best][1]) {
                        best = j;
                    }
                }
                let mut walk = Vec::with_capacity(n);
                for j in 0..n {
                    let e = lp[(best + j) % n];
                    let nx = lp[(best + j + 1) % n];
                    walk.push(WallEdge { qa: e[2], ra: e[3], qb: e[4], rb: e[5], ak: e[0], am: e[1], bk: nx[0], bm: nx[1] });
                }
                out.push(walk);
            }
        }
    }
    out
}

// ── cellwalls.loft ──────────────────────────────────────────────────

const AROUND: i64 = 2;
const SLANT: usize = 6;
const NO_LINE: i64 = -1000000;

fn head_step(h: i64) -> (i64, i64) {
    const STEPS: [(i64, i64); 12] = [(2, 0), (3, 3), (1, 3), (0, 6), (-1, 3), (-3, 3),
                                     (-2, 0), (-3, -3), (-1, -3), (0, -6), (1, -3), (3, -3)];
    STEPS[h.rem_euclid(12) as usize]
}

fn dot4(ak: i64, am: i64, bk: i64, bm: i64) -> i64 {
    3 * ak * bk + am * bm
}

/// The boundary's travel round edge i: from AROUND edges before it to AROUND after.
fn span(b: &[WallEdge], i: usize) -> (i64, i64) {
    let n = b.len() as i64;
    let a = b[(i as i64 - AROUND).rem_euclid(n) as usize];
    let z = b[((i as i64 + AROUND) % n) as usize];
    (z.bk - a.ak, z.bm - a.am)
}

fn edge_travel(turn: i64, b: &[WallEdge], i: usize) -> i64 {
    let (wk, wm) = span(b, i);
    let (uk, um) = head_step(turn);
    let (vk, vm) = head_step(turn + 3);
    let u = dot4(wk, wm, uk, um);
    let v = dot4(wk, wm, vk, vm);
    let au = if turn % 2 == 0 { 3 * u * u } else { u * u };
    let av = if turn % 2 == 0 { v * v } else { 3 * v * v };
    if au > av {
        if u > 0 { 0 } else { 2 }
    } else if av > au {
        if v > 0 { 1 } else { 3 }
    } else {
        -1
    }
}

fn edge_travels(turn: i64, b: &[WallEdge]) -> Vec<i64> {
    (0..b.len()).map(|i| edge_travel(turn, b, i)).collect()
}

fn runs_groups(label: &[i64]) -> Vec<Vec<usize>> {
    let n = label.len();
    let mut start = 0;
    for i in 0..n {
        if start == 0 && label[i] != label[(i + n - 1) % n] {
            start = i;
        }
    }
    let mut out: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    for j in 0..n {
        let i = (start + j) % n;
        if !cur.is_empty() && label[i] != label[cur[0]] {
            out.push(std::mem::take(&mut cur));
        }
        cur.push(i);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

// The runs of equal labels, a run under two edges joined to the one before it: kept as the
// places where the label changes.
fn runs_merged(given: &[i64]) -> Vec<Vec<usize>> {
    let n = given.len();
    if n == 0 {
        return Vec::new();
    }
    let mut label = given.to_vec();
    for i in 0..n {
        if label[i] < 0 {
            label[i] = label[(i + n - 1) % n];
        }
    }
    let mut cp: Vec<usize> = (0..n).filter(|&i| label[i] != label[(i + n - 1) % n]).collect();
    for _ in 0..n {
        let g = cp.len();
        if g < 2 {
            break;
        }
        let first = if cp[0] == 0 { 1 } else { 0 };
        let mut short = usize::MAX;
        for j in 0..g {
            let at = (first + j) % g;
            let size = (cp[(at + 1) % g] + n - cp[at]) % n;
            if size < 2 {
                short = at;
                break;
            }
        }
        if short == usize::MAX {
            break;
        }
        let before = (short + g - 1) % g;
        let after = (short + 1) % g;
        let prev = label[cp[before]];
        let from = cp[short];
        let size = (cp[after] + n - from) % n;
        for j in 0..size {
            label[(from + j) % n] = prev;
        }
        let gone_after = label[cp[after]] == prev && after != before;
        let whole = g == 2 && label[cp[after]] == prev;
        cp = (0..g).filter(|&j| !(j == short || (j == after && gone_after) || whole)).map(|j| cp[j]).collect();
    }
    runs_groups(&label)
}

fn lattice_turned(k: i64, m: i64, steps: i64) -> (i64, i64) {
    let (mut tk, mut tm) = (k, m);
    for _ in 0..steps.rem_euclid(6) {
        (tk, tm) = ((tk - tm) / 2, (3 * tk + tm) / 2);
    }
    (tk, tm)
}

fn rational_turned(nk: i64, nm: i64, d: i64, steps: i64) -> (i64, i64, i64) {
    let (mut tk, mut tm, mut td) = (nk, nm, d);
    for _ in 0..steps.rem_euclid(6) {
        (tk, tm) = (tk - tm, 3 * tk + tm);
        td *= 2;
    }
    (tk, tm, td)
}

// The most common value, ties to the one nearest the mean, then to the one met first.
fn most_common(vals: &[i64], sum: i64, n: i64) -> i64 {
    let mut seen: Vec<(i64, i64)> = Vec::new();
    for &v in vals {
        match seen.iter_mut().find(|e| e.0 == v) {
            Some(e) => e.1 += 1,
            None => seen.push((v, 1)),
        }
    }
    let (mut best, mut best_n) = (0i64, 0i64);
    for &(v, c) in &seen {
        let far = (v * n - sum).abs();
        let best_far = (best * n - sum).abs();
        if c > best_n || c == best_n && far < best_far {
            best = v;
            best_n = c;
        }
    }
    best
}

fn wall_tops(b: &[WallEdge], run: &[usize], h: i64) -> WallSurface {
    let s = (6 - h / 2) % 6;
    let mut mids = Vec::with_capacity(run.len());
    let (mut sk, mut sm) = (0i64, 0i64);
    for &i in run {
        let e = b[i];
        let (ak, am) = lattice_turned(e.ak, e.am, s);
        let (bk, bm) = lattice_turned(e.bk, e.bm, s);
        mids.push(am + bm);
        sk += ak + bk;
        sm += am + bm;
    }
    let n = run.len() as i64;
    let line_m = most_common(&mids, sm, n);
    let (nk, nm, d) = rational_turned(sk, n * line_m, 2 * n, 6 - s);
    let (dk, dm) = head_step(h);
    WallSurface { dk, dm, num_k: nk, num_m: nm, den: d, n }
}

fn wall_sides(b: &[WallEdge], run: &[usize], h: i64) -> WallSurface {
    let s = ((3 - h) / 2) % 6;
    let mut links = Vec::new();
    let (mut sk, mut sm, mut lk) = (0i64, 0i64, 0i64);
    for &i in run {
        let e = b[i];
        let (ak, am) = lattice_turned(e.ak, e.am, s);
        let (bk, bm) = lattice_turned(e.bk, e.bm, s);
        if ak != bk {
            links.push(ak + bk);
            lk += ak + bk;
        }
        sk += ak + bk;
        sm += am + bm;
    }
    let n = run.len() as i64;
    let line_k = if links.is_empty() { if n != 0 { sk / n } else { 0 } } else { most_common(&links, lk, links.len() as i64) };
    let (nk, nm, d) = rational_turned(n * line_k, sm, 2 * n, 6 - s);
    let (dk, dm) = head_step(h);
    WallSurface { dk, dm, num_k: nk, num_m: nm, den: d, n }
}

fn cells_wall(b: &[WallEdge], run: &[usize], h: i64) -> WallSurface {
    let hn = h.rem_euclid(12);
    if hn % 2 == 0 { wall_tops(b, run, hn) } else { wall_sides(b, run, hn) }
}

fn run_heading_by(turn: i64, travel: &[i64], run: &[usize]) -> i64 {
    let mut count = [0i64; 4];
    for &i in run {
        if travel[i] >= 0 {
            count[travel[i] as usize] += 1;
        }
    }
    let mut best = 0;
    for t in 1..4 {
        if count[t] > count[best] {
            best = t;
        }
    }
    (turn + 3 * best as i64).rem_euclid(12)
}

fn edge_nearest_heading(b: &[WallEdge], i: usize) -> i64 {
    let (wk, wm) = span(b, i);
    let (mut best, mut best_d, mut best_l) = (-1i64, 0i64, 1i64);
    for h in 0..12 {
        let (sk, sm) = head_step(h);
        let d = dot4(wk, wm, sk, sm);
        if d <= 0 {
            continue;
        }
        let l = if h % 2 == 0 { 3 } else { 9 };
        if best < 0 || d * d * best_l > best_d * best_d * l {
            best = h;
            best_d = d;
            best_l = l;
        }
    }
    best
}

fn axes_by(near: &[i64]) -> i64 {
    let mut votes = [0i64; 3];
    for &h in near {
        if h >= 0 {
            votes[(h % 3) as usize] += 1;
        }
    }
    if votes[0] > votes[1] && votes[0] > votes[2] {
        0
    } else if votes[1] > votes[0] && votes[1] > votes[2] {
        1
    } else if votes[2] > votes[0] && votes[2] > votes[1] {
        2
    } else {
        -1
    }
}

fn surface_heading(w: &WallSurface) -> i64 {
    for sh in 0..12 {
        let st = head_step(sh);
        if w.dk * st.1 - w.dm * st.0 == 0 && w.dk * st.0 + w.dm * st.1 > 0 {
            return sh;
        }
    }
    -1
}

// Where two surfaces' lines meet; (0, 0) when they are parallel.
fn surface_meet(wa: &WallSurface, wb: &WallSurface) -> (f64, f64) {
    let ax = wa.mean_x();
    let ay = wa.mean_y();
    let bx = wb.mean_x();
    let by = wb.mean_y();
    let aux = wa.dk as f64 * 0.8660254037844386;
    let auy = wa.dm as f64 * 0.5;
    let bux = wb.dk as f64 * 0.8660254037844386;
    let buy = wb.dm as f64 * 0.5;
    let den = aux * buy - auy * bux;
    if den < 0.000000001 && den > 0.0 - 0.000000001 {
        return (0.0, 0.0);
    }
    let tt = ((bx - ax) * buy - (by - ay) * bux) / den;
    (ax + tt * aux, ay + tt * auy)
}

struct CellWalls {
    edges: EdgeSet,
    boundary: Vec<WallEdge>,
    runs: Vec<Vec<usize>>,
    walls: Vec<WallSurface>,
    corners: Vec<(f64, f64)>,
}

fn cells_built(cells: &HexSet, wall_mat: i64, hint: i64) -> CellWalls {
    let mut edges = EdgeSet::new(cells.q0 - 1, cells.r0 - 1, cells.w + 2, cells.h + 2);
    let mut loops = cells_loops(cells);
    for lp in &loops {
        for e in lp {
            edges.set_mat(e.qa, e.ra, e.qb, e.rb, wall_mat);
        }
    }
    let b = if loops.is_empty() { Vec::new() } else { loops.swap_remove(0) };
    let near: Vec<i64> = (0..b.len()).map(|i| edge_nearest_heading(&b, i)).collect();
    let mut turn = axes_by(&near);
    if turn < 0 {
        turn = if hint >= 0 { hint % 3 } else { 0 };
    }
    let travel = edge_travels(turn, &b);
    let runs = runs_merged(&travel);
    let walls: Vec<WallSurface> = runs.iter().map(|run| cells_wall(&b, run, run_heading_by(turn, &travel, run))).collect();
    let nw = walls.len();
    let corners = (0..nw).map(|i| surface_meet(&walls[i], &walls[(i + 1) % nw])).collect();
    CellWalls { edges, boundary: b, runs, walls, corners }
}

/// The turn that brings a wall along heading `hn` to its family's frame, and which family.
fn frame_of(hn: i64) -> (bool, i64) {
    let tops = hn % 2 == 0;
    (tops, if tops { (6 - hn / 2) % 6 } else { ((3 - hn) / 2) % 6 })
}

fn split_by_line(b: &[WallEdge], run: &[usize], h: i64) -> Vec<Vec<usize>> {
    let (tops, s) = frame_of(h.rem_euclid(12));
    let at: Vec<i64> = run
        .iter()
        .map(|&i| {
            let e = b[i];
            let (ak, am) = lattice_turned(e.ak, e.am, s);
            let (bk, bm) = lattice_turned(e.bk, e.bm, s);
            if tops { am + bm } else if ak != bk { ak + bk } else { NO_LINE }
        })
        .collect();
    let mut parts: Vec<Vec<usize>> = Vec::new();
    let mut cur: Vec<usize> = Vec::new();
    let mut line = NO_LINE;
    for j in 0..run.len() {
        let v = at[j];
        if v != NO_LINE && line != NO_LINE && v != line {
            let nxt = at[j + 1..].iter().copied().find(|&a| a != NO_LINE).unwrap_or(NO_LINE);
            if nxt == v && !cur.is_empty() {
                parts.push(std::mem::take(&mut cur));
            }
        }
        if v != NO_LINE && (line == NO_LINE || cur.is_empty() || v == line) {
            line = v;
        }
        cur.push(run[j]);
    }
    if !cur.is_empty() {
        parts.push(cur);
    }
    let n = parts.len();
    if n < 2 {
        return parts;
    }
    let mut out: Vec<Vec<usize>> = Vec::new();
    for i in 0..n {
        if i == 1 && parts[0].len() < 3 {
            let mut joined = parts[0].clone();
            joined.extend(&parts[1]);
            out = vec![joined];
            continue;
        }
        if i == 0 && parts[0].len() < 3 {
            continue;
        }
        if i == n - 1 && parts[i].len() < 3 && !out.is_empty() {
            let last = out.len() - 1;
            out[last].extend(&parts[i]);
            continue;
        }
        out.push(parts[i].clone());
    }
    out
}

fn edge_headings(b: &[WallEdge], turn: i64, free: &[i64]) -> Vec<i64> {
    let n = b.len();
    let mut out: Vec<i64> = (0..n)
        .map(|i| {
            let t = edge_travel(turn, b, i);
            if t < 0 { -1 } else { (turn + 3 * t).rem_euclid(12) }
        })
        .collect();
    let mut i = 0;
    while i < n {
        let h = free[i];
        let mut j = i;
        while j < n && free[j] == h {
            j += 1;
        }
        if h >= 0 && (h - turn).rem_euclid(3) != 0 && j - i >= SLANT {
            for o in &mut out[i..j] {
                *o = h;
            }
        }
        i = if j > i { j } else { i + 1 };
    }
    out
}

fn step_edge(b: &[WallEdge], ra: &[usize], rb: &[usize], wa: &WallSurface, wb: &WallSurface, h: i64) -> WallEdge {
    let nk = -wa.dm;
    let nm = wa.dk;
    let oa = (3 * wa.num_k * nk + wa.num_m * nm) as f64 / wa.den as f64;
    let ob = (3 * wb.num_k * nk + wb.num_m * nm) as f64 / wb.den as f64;
    let lo = if oa < ob { oa } else { ob };
    let hi = if oa < ob { ob } else { oa };
    let (tops, s) = frame_of(h.rem_euclid(12));
    let mut cand: Vec<usize> = Vec::new();
    for j in 0..4 {
        if ra.len() > j {
            cand.push(ra[ra.len() - 1 - j]);
        }
    }
    for j in 0..4 {
        if j < rb.len() {
            cand.push(rb[j]);
        }
    }
    let mut best = usize::MAX;
    for &i in &cand {
        let e = b[i];
        let p = (3 * (e.ak + e.bk) * nk + (e.am + e.bm) * nm) as f64 * 0.5;
        if p <= lo + 0.001 || p >= hi - 0.001 {
            continue;
        }
        if !tops {
            let (ak, _) = lattice_turned(e.ak, e.am, s);
            let (bk, _) = lattice_turned(e.bk, e.bm, s);
            if ak == bk {
                continue;
            }
        }
        if best == usize::MAX {
            best = i;
        }
    }
    if best == usize::MAX {
        best = ra[ra.len() - 1];
    }
    b[best]
}

struct CellOutline {
    boundary: usize,
    walls: Vec<WallSurface>,
    corners: Vec<(f64, f64)>,
}

fn cells_outline(b: &[WallEdge], hint: i64) -> CellOutline {
    let near: Vec<i64> = (0..b.len()).map(|i| edge_nearest_heading(b, i)).collect();
    let mut turn = axes_by(&near);
    if turn < 0 {
        turn = if hint >= 0 { hint % 3 } else { 0 };
    }
    let heads = edge_headings(b, turn, &near);
    let mut rs: Vec<Vec<usize>> = Vec::new();
    let mut hs: Vec<i64> = Vec::new();
    for run in runs_merged(&heads) {
        let h = heads[run[0]];
        for part in split_by_line(b, &run, h) {
            rs.push(part);
            hs.push(h);
        }
    }
    let solved: Vec<WallSurface> = (0..rs.len()).map(|i| cells_wall(b, &rs[i], hs[i])).collect();
    let n = solved.len();
    let mut walls: Vec<WallSurface> = Vec::new();
    for i in 0..n {
        let w = solved[i];
        walls.push(w);
        let nx = solved[(i + 1) % n];
        if n > 1 && w.dk * nx.dm - w.dm * nx.dk == 0 {
            let h = (surface_heading(&w) + 3).rem_euclid(12);
            let e = step_edge(b, &rs[i], &rs[(i + 1) % n], &w, &nx, h);
            let (dk, dm) = head_step(h);
            walls.push(WallSurface { dk, dm, num_k: e.ak + e.bk, num_m: e.am + e.bm, den: 2, n: 0 });
        }
    }
    let nw = walls.len();
    if nw < 3 {
        return CellOutline { boundary: b.len(), walls: Vec::new(), corners: Vec::new() };
    }
    let corners = (0..nw).map(|i| surface_meet(&walls[i], &walls[(i + 1) % nw])).collect();
    CellOutline { boundary: b.len(), walls, corners }
}

fn cells_outlines(cells: &HexSet, hint: i64) -> Vec<CellOutline> {
    cells_loops(cells).iter().map(|lp| cells_outline(lp, hint)).collect()
}

// ── the cellwalls rows ──────────────────────────────────────────────

const BUILDS: i64 = 20;
const OUTLINES: i64 = 10;

fn ell(dq: i64) -> HexSet {
    let mut c = HexSet::chunk(-HALF, -HALF, WIN, WIN);
    draw_floor(&Plan::new(dq, 0, 20, 12, 0, false), &mut c);
    draw_floor(&Plan::new(dq - 6, 9, 8, 8, 0, false), &mut c);
    c
}

fn surface_ints(w: &WallSurface, out: &mut Vec<i64>) {
    out.extend([w.dk, w.dm, w.num_k, w.num_m, w.den, w.n]);
}

fn built_op(cells: &HexSet) -> Vec<i64> {
    let mut ints: Vec<i64> = Vec::new();
    let mut edges = 0i64;
    for t in 0..BUILDS {
        let b = cells_built(cells, 9, -1);
        edges += b.boundary.len() as i64;
        if t == BUILDS - 1 {
            ints = vec![b.walls.len() as i64, b.boundary.len() as i64];
            for w in &b.walls {
                surface_ints(w, &mut ints);
            }
            for run in &b.runs {
                ints.push(run.len() as i64);
                ints.push(run.first().map_or(-1, |&i| i as i64));
            }
            for c in &b.corners {
                ints.push((c.0 * 1000000.0) as i64);
                ints.push((c.1 * 1000000.0) as i64);
            }
            ints.push(b.edges.count_all());
            ints.push(b.edges.digest());
        }
    }
    ints.push(edges);
    ints
}

fn bench_built(n: i64) -> Row {
    let c0 = ell(0);
    let c1 = ell(1);
    let (us, sink) = timed(n, |r| built_op(if (r & 1) == 0 { &c0 } else { &c1 })[1]);
    let one = built_op(black_box(&c0));
    Row { name: "cells_built", iters: n, us, px: BUILDS * one[1], hash: fnv(FNV_OFFSET, &one), sink }
}

fn street(dq: i64) -> HexSet {
    let mut c = HexSet::chunk(-HALF, -HALF, WIN, WIN);
    for q in -24..25 {
        let off = ((q + 27) / 9) % 3;
        for r in 0..3 {
            c.set(q + dq, 14 + off + r, true);
            c.set(q + dq, -18 + off + r, true);
        }
    }
    for r in -18..19 {
        for w in 0..3 {
            c.set(-24 + w + dq, r, true);
            c.set(22 + w + dq, r, true);
        }
    }
    c
}

fn outline_op(cells: &HexSet) -> Vec<i64> {
    let mut ints: Vec<i64> = Vec::new();
    let mut walls = 0i64;
    for t in 0..OUTLINES {
        let os = cells_outlines(cells, 0);
        for o in &os {
            walls += o.walls.len() as i64;
        }
        if t == OUTLINES - 1 {
            ints = vec![os.len() as i64, 0];
            for o in &os {
                ints.push(o.walls.len() as i64);
                ints.push(o.boundary as i64);
                ints[1] += o.boundary as i64;
                for w in &o.walls {
                    surface_ints(w, &mut ints);
                }
                for c in &o.corners {
                    ints.push((c.0 * 1000000.0) as i64);
                    ints.push((c.1 * 1000000.0) as i64);
                }
            }
        }
    }
    ints.push(walls);
    ints
}

fn bench_outlines(n: i64) -> Row {
    let c0 = street(0);
    let c1 = street(1);
    let (us, sink) = timed(n, |r| outline_op(if (r & 1) == 0 { &c0 } else { &c1 })[1]);
    let one = outline_op(black_box(&c0));
    Row { name: "cells_outlines", iters: n, us, px: OUTLINES * one[1], hash: fnv(FNV_OFFSET, &one), sink }
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
    let rows = [bench_spread(n), bench_built(n), bench_outlines(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
