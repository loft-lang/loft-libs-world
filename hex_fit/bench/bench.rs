// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_fit-reference — the pure-Rust twin of the `hex_fit` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workload with the
// SAME arithmetic, in the same order, and prints the same row, hash included: a row whose
// hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports: `draft_fit_p` and `run_is_inside` from src/hexdraft.loft; the vertex-anchored run
// from hex_shape (`tri_is_vertex`, `wall_run_ok`, `wall_from_run`, `wall_write`,
// `wall_separates` and the 24-direction step table under them, with hex_form's `head_step`);
// the edge layer's material slots (`edge_mat`, `edge_set_mat`, its slot index), the window
// and `hexdisk_into` from hex_field; `hex_neighbor` from hex_grid.
//
// ⚠ THE ONE DIFFERENCE IS THE EDGE SET.  The library builds a fresh `EdgeSet` for every
// trial — two parallel vectors (material and surface) filled element by element — and throws
// it away; the twin keeps ONE `Vec<u8>` of material slots and zeroes it per trial.  The
// slots, the rasterisation and the sweep are the same, so the offer is the same.
// `black_box` guards each op's INPUT and the sink, never a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const WIN: i64 = 41;
const HALF: i64 = 20;
const RADIUS: i64 = 16;
const WANT: i64 = 24;
const TRIALS: i64 = 89;

const WALL_EPS: f64 = 0.000000001;

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

// ── hex_field: the window, the lattice, the edge slots ──────────────

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

/// The material slots of an edge layer over a window, one reused buffer: three slots per
/// cell of the window plus a one-cell halo.
struct EdgeMats {
    q0: i64,
    r0: i64,
    gw: i64,
    gh: i64,
    mat: Vec<u8>,
}

impl EdgeMats {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> EdgeMats {
        let (gw, gh) = (w + 2, h + 2);
        EdgeMats { q0, r0, gw, gh, mat: vec![0; (gw * gh * 3) as usize] }
    }

    fn clear(&mut self) {
        self.mat.fill(0);
    }

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

    fn get(&self, qa: i64, ra: i64, qb: i64, rb: i64) -> i64 {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 { 0 } else { self.mat[i as usize] as i64 }
    }

    fn set(&mut self, qa: i64, ra: i64, qb: i64, rb: i64, mat: i64) {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 || !(0..=255).contains(&mat) {
            return;
        }
        self.mat[i as usize] = mat as u8;
    }
}

// ── hex_grid: the neighbour in its own direction order ──────────────

fn hex_neighbor(q: i64, r: i64, dir: i64) -> (i64, i64) {
    if (r & 1) == 0 {
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

// ── hex_form + hex_shape: the 24 directions and the run ─────────────

fn head_step(h: i64) -> (i64, i64) {
    const STEP: [(i64, i64); 12] = [
        (2, 0), (3, 3), (1, 3), (0, 6), (-1, 3), (-3, 3),
        (-2, 0), (-3, -3), (-1, -3), (0, -6), (1, -3), (3, -3),
    ];
    STEP[(((h % 12) + 12) % 12) as usize]
}

fn between(i: i64) -> (i64, i64) {
    const KM: [(i64, i64); 6] = [(7, 3), (5, 9), (2, 12), (-2, 12), (-5, 9), (-7, 3)];
    let (j, s) = if i >= 6 { (i - 6, -1) } else { (i, 1) };
    let (k, m) = KM[j as usize];
    (s * k, s * m)
}

fn wall_step(d24: i64) -> (i64, i64) {
    let n = ((d24 % 24) + 24) % 24;
    if n % 2 == 0 { head_step(n / 2) } else { between((n - 1) / 2) }
}

fn igcd(x: i64, y: i64) -> i64 {
    let (mut a, mut b) = (x.abs(), y.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The run's primitive step on the triangle lattice, reduced.
fn tri_ab(d24: i64) -> (i64, i64) {
    let (k, m) = wall_step(d24);
    let ta = 3 * k;
    let tb = 3 * (m - k) / 2;
    let g = igcd(ta, tb);
    if g == 0 { (ta, tb) } else { (ta / g, tb / g) }
}

fn imod3(v: i64) -> i64 {
    v.rem_euclid(3)
}

fn tri_is_vertex(a: i64, b: i64) -> bool {
    imod3(a) == 0 && imod3(b) == 0 && imod3((a - b) / 3) != 0
}

fn wall_run_ok(d24: i64, a0: i64, b0: i64, p: i64) -> bool {
    if p <= 0 || !tri_is_vertex(a0, b0) {
        return false;
    }
    let (ta, tb) = tri_ab(d24);
    tri_is_vertex(a0 + 3 * p * ta, b0 + 3 * p * tb)
}

fn tri_x(a: i64) -> f64 {
    a as f64 * 0.28867513459481287
}

fn tri_y(a: i64, b: i64) -> f64 {
    a as f64 / 6.0 + b as f64 / 3.0
}

struct Wall {
    ox: f64,
    oy: f64,
    dx: f64,
    dy: f64,
    half: f64,
    mat: i64,
}

impl Wall {
    fn from_run(d24: i64, a0: i64, b0: i64, p: i64, mat: i64) -> Wall {
        let (ta, tb) = tri_ab(d24);
        let (ea, eb) = (a0 + 3 * p * ta, b0 + 3 * p * tb);
        let (ax, ay) = (tri_x(a0), tri_y(a0, b0));
        let (bx, by) = (tri_x(ea), tri_y(ea, eb));
        let (vx, vy) = (bx - ax, by - ay);
        let ln = (vx * vx + vy * vy).sqrt();
        Wall { ox: (ax + bx) * 0.5, oy: (ay + by) * 0.5, dx: vx / ln, dy: vy / ln, half: ln * 0.5, mat }
    }

    fn offset(&self, px: f64, py: f64) -> f64 {
        (px - self.ox) * (0.0 - self.dy) + (py - self.oy) * self.dx
    }

    fn along(&self, px: f64, py: f64) -> f64 {
        (px - self.ox) * self.dx + (py - self.oy) * self.dy
    }

    /// Does the run cross the segment between two cell centres, within its length?
    fn separates(&self, cx: f64, cy: f64, nx: f64, ny: f64) -> bool {
        let oc = self.offset(cx, cy);
        let on = self.offset(nx, ny);
        if (oc > 0.0 - WALL_EPS) == (on > 0.0 - WALL_EPS) {
            return false;
        }
        let t = oc / (oc - on);
        let al = self.along(cx + t * (nx - cx), cy + t * (ny - cy));
        al >= (0.0 - self.half) - WALL_EPS && al <= self.half + WALL_EPS
    }
}

fn cell_x_w(q: i64, r: i64) -> f64 {
    lattice_k(q, r) as f64 * 0.8660254037844386
}

fn cell_y_w(r: i64) -> f64 {
    lattice_m(r) as f64 * 0.5
}

/// Mark every edge the run separates; the count includes a crossing whose slot lies off the
/// layer, exactly as the library counts it.
fn wall_write(w: &Wall, e: &mut EdgeMats, q0: i64, r0: i64, wq: i64, hq: i64) -> i64 {
    let mut n = 0;
    for r in r0..r0 + hq {
        for q in q0..q0 + wq {
            for d in 0..6 {
                let (nq, nr) = hex_neighbor(q, r, d);
                if w.separates(cell_x_w(q, r), cell_y_w(r), cell_x_w(nq, nr), cell_y_w(nr))
                    && e.get(q, r, nq, nr) == 0
                {
                    e.set(q, r, nq, nr, w.mat);
                    n += 1;
                }
            }
        }
    }
    n
}

// ── hex_fit: the doorstep's offer ───────────────────────────────────

fn run_is_inside(cells: &HexSet, e: &mut EdgeMats, d: i64, a: i64, b: i64, p: i64) -> bool {
    e.clear();
    let w = Wall::from_run(d, a, b, p, 1);
    let n = wall_write(&w, e, cells.q0, cells.r0, cells.w, cells.h);
    if n <= 0 {
        return false;
    }
    let mut ins = 0;
    for r in cells.r0..cells.r0 + cells.h {
        for q in cells.q0..cells.q0 + cells.w {
            for dd in 0..3 {
                let (nq, nr) = hex_neighbor(q, r, dd);
                if e.get(q, r, nq, nr) != 0 && cells.get(q, r) && cells.get(nq, nr) {
                    ins += 1;
                }
            }
        }
    }
    ins == n
}

struct Draft {
    d: i64,
    a: i64,
    b: i64,
    p: i64,
}

fn draft_fit_p(s: &Draft, cells: &HexSet, e: &mut EdgeMats) -> i64 {
    if s.d < 0 || s.d >= 24 || !tri_is_vertex(s.a, s.b) {
        return 0;
    }
    let want = s.p.max(1);
    for k in 0..want {
        let t = want - k;
        if t >= 1 && wall_run_ok(s.d, s.a, s.b, t) && run_is_inside(cells, e, s.d, s.a, s.b, t) {
            return t;
        }
    }
    0
}

// ── The row ─────────────────────────────────────────────────────────

fn footprint(dq: i64) -> HexSet {
    let mut c = HexSet::chunk(-HALF, -HALF, WIN, WIN);
    hexdisk_into(&mut c, dq, 0, RADIUS);
    c
}

fn fit_op(cells: &HexSet, e: &mut EdgeMats, dq: i64) -> Vec<i64> {
    [9, 7, 5, 3]
        .iter()
        .map(|&d| draft_fit_p(&Draft { d, a: -60 + 6 * dq, b: 33 - 3 * dq, p: WANT }, cells, e))
        .collect()
}

fn bench_fit(n: i64) -> Row {
    let c0 = footprint(0);
    let c1 = footprint(1);
    let mut e = EdgeMats::new(-HALF, -HALF, WIN, WIN);
    let (us, sink) = timed(n, |r| {
        if (r & 1) == 0 { fit_op(&c0, &mut e, 0)[0] } else { fit_op(&c1, &mut e, 1)[0] }
    });
    let mut out = fit_op(&c0, &mut e, black_box(0));
    out.push(c0.count);
    out.push(c1.count);
    Row { name: "draft_fit_p", iters: n, us, px: TRIALS, hash: fnv(FNV_OFFSET, &out), sink }
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
    let rows = [bench_fit(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
