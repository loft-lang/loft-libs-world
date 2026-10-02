// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_roof-reference — the pure-Rust twin of the `hex_roof` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports: `roof_match` from src/hex_roof.loft and everything it calls — the plane fit,
// the cone / dome / ridge hill-climbs and their per-candidate sweeps (`cone_at`, `dome_at`,
// `ridge_at`) — plus `roof_hip` (the input) and `roof_cone`; from hex_way the straight
// segment of `track_distance`, over the same 6-float segment record; from hex_field the
// `HexSet` / `Heights` storage, `nb_q`/`nb_r`, the lattice and `hexdisk_into`.  Every sweep
// runs over `Vec<f64>` in the window's row-major order.  `black_box` guards each op's INPUT
// and the sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const HIP_W: i64 = 30;
const HIP_N: i64 = 12;
const CONE_W: i64 = 128;
const CONES: i64 = 100;

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

fn micro(x: f64) -> i64 {
    (x * 1000000.0) as i64
}

// ── hex_field.loft ──────────────────────────────────────────────────

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

struct Heights {
    q0: i64,
    r0: i64,
    w: i64,
    h: i64,
    z: Vec<f64>,
}

impl Heights {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> Heights {
        Heights { q0, r0, w, h, z: vec![0.0; (w * h) as usize] }
    }
    fn index(&self, q: i64, r: i64) -> i64 {
        let dq = q - self.q0;
        let dr = r - self.r0;
        if dq < 0 || dr < 0 || dq >= self.w || dr >= self.h {
            return -1;
        }
        dr * self.w + dq
    }
    fn set(&mut self, q: i64, r: i64, z: f64) {
        let i = self.index(q, r);
        if i >= 0 {
            self.z[i as usize] = z;
        }
    }
    fn get(&self, q: i64, r: i64) -> f64 {
        let i = self.index(q, r);
        if i < 0 {
            return 0.0;
        }
        self.z[i as usize]
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

// ── hex_way.loft: the straight segment of `track_distance` ──────────

struct Track {
    p: Vec<[f64; 6]>,
}

impl Track {
    fn straight(x0: f64, y0: f64, x1: f64, y1: f64) -> Track {
        Track { p: vec![[x0, y0, x1, y1, 0.0, 0.0]] }
    }
}

fn seg_distance(p: &[f64; 6], px: f64, py: f64) -> f64 {
    let (ax, ay) = (p[0], p[1]);
    let (vx, vy) = (p[2] - ax, p[3] - ay);
    let len2 = vx * vx + vy * vy;
    let mut u = 0.0;
    if len2 > 0.000000001 {
        u = ((px - ax) * vx + (py - ay) * vy) / len2;
    }
    if u < 0.0 {
        u = 0.0;
    }
    if u > 1.0 {
        u = 1.0;
    }
    let dx = px - (ax + u * vx);
    let dy = py - (ay + u * vy);
    (dx * dx + dy * dy).sqrt()
}

fn track_distance(t: &Track, px: f64, py: f64) -> f64 {
    let mut best = 1000000.0;
    for p in &t.p {
        let d = seg_distance(p, px, py);
        if d < best {
            best = d;
        }
    }
    best
}

// ── hex_roof.loft ───────────────────────────────────────────────────

const ROOF_UNKNOWN: i64 = 0;
const ROOF_PLANE: i64 = 1;
const ROOF_CONE: i64 = 2;
const ROOF_DOME: i64 = 3;
const ROOF_RIDGE: i64 = 4;

fn cellx(q: i64, r: i64) -> f64 {
    (lattice_k(q, r) as f64) * 0.8660254037844386
}

fn celly(r: i64) -> f64 {
    (lattice_m(r) as f64) * 0.5
}

/// The occupied cells of a footprint in scan order, with their centres.  Every sweep below
/// walks the window exactly as the loft routines do and skips the empty cells the same way.
fn roof_cone(s: &HexSet, f: &mut Heights, cx: f64, cy: f64, peak: f64, slope: f64, dmax: f64) {
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let dx = cellx(q, r) - cx;
                let dy = celly(r) - cy;
                let mut d = (dx * dx + dy * dy).sqrt();
                if d > dmax {
                    d = dmax;
                }
                f.set(q, r, peak - slope * d);
            }
        }
    }
}

fn roof_hip(s: &HexSet, f: &mut Heights, eave: f64, per_ring: f64) -> i64 {
    let cap = s.w * s.h;
    let mut bq = vec![0i64; cap as usize];
    let mut br = vec![0i64; cap as usize];
    let mut bd = vec![0i64; cap as usize];
    let mut vis = HexSet::chunk(s.q0, s.r0, s.w, s.h);
    let mut n = 0i64;
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let mut edge = false;
                for i in 0..6 {
                    if !s.get(nb_q(q, r, i), nb_r(r, i)) {
                        edge = true;
                    }
                }
                if edge && n < cap {
                    f.set(q, r, eave);
                    vis.set(q, r, true);
                    bq[n as usize] = q;
                    br[n as usize] = r;
                    bd[n as usize] = 0;
                    n += 1;
                }
            }
        }
    }
    let mut head = 0i64;
    let mut top = 0i64;
    while head < n {
        let cq = bq[head as usize];
        let cr = br[head as usize];
        let cd = bd[head as usize];
        head += 1;
        for i in 0..6 {
            let nq = nb_q(cq, cr, i);
            let nr = nb_r(cr, i);
            if s.get(nq, nr) && !vis.get(nq, nr) && n < cap {
                vis.set(nq, nr, true);
                f.set(nq, nr, eave + ((cd + 1) as f64) * per_ring);
                bq[n as usize] = nq;
                br[n as usize] = nr;
                bd[n as usize] = cd + 1;
                n += 1;
                if cd + 1 > top {
                    top = cd + 1;
                }
            }
        }
    }
    top
}

#[derive(Clone, Copy)]
struct RoofFit {
    kind: i64,
    x: f64,
    y: f64,
    z: f64,
    slope: f64,
    res: f64,
    x2: f64,
    y2: f64,
}

impl RoofFit {
    fn new(kind: i64, x: f64, y: f64, z: f64, slope: f64, res: f64) -> RoofFit {
        RoofFit { kind, x, y, z, slope, res, x2: 0.0, y2: 0.0 }
    }
}

fn roof_plane_fit(s: &HexSet, f: &Heights) -> RoofFit {
    let (mut sxx, mut sxy, mut sx, mut syy, mut sy, mut sn) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut sxz, mut syz, mut sz) = (0.0, 0.0, 0.0);
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let (x, y, z) = (cellx(q, r), celly(r), f.get(q, r));
                sxx = sxx + x * x;
                sxy = sxy + x * y;
                sx = sx + x;
                syy = syy + y * y;
                sy = sy + y;
                sn = sn + 1.0;
                sxz = sxz + x * z;
                syz = syz + y * z;
                sz = sz + z;
            }
        }
    }
    let det = sxx * (syy * sn - sy * sy) - sxy * (sxy * sn - sy * sx) + sx * (sxy * sy - syy * sx);
    let mut ad = det;
    if ad < 0.0 {
        ad = 0.0 - ad;
    }
    if ad < 0.000000001 {
        return RoofFit::new(ROOF_UNKNOWN, 0.0, 0.0, 0.0, 0.0, 1000000.0);
    }
    let da = sxz * (syy * sn - sy * sy) - sxy * (syz * sn - sy * sz) + sx * (syz * sy - syy * sz);
    let db = sxx * (syz * sn - sy * sz) - sxz * (sxy * sn - sy * sx) + sx * (sxy * sz - syz * sx);
    let dc = sxx * (syy * sz - syz * sy) - sxy * (sxy * sz - syz * sx) + sxz * (sxy * sy - syy * sx);
    let a = da / det;
    let b = db / det;
    let c = dc / det;
    let mut res = 0.0;
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let mut e = a * cellx(q, r) + b * celly(r) + c - f.get(q, r);
                if e < 0.0 {
                    e = 0.0 - e;
                }
                if e > res {
                    res = e;
                }
            }
        }
    }
    RoofFit::new(ROOF_PLANE, a, b, c, 0.0, res)
}

fn cone_at(s: &HexSet, f: &Heights, cx: f64, cy: f64) -> (f64, f64, f64) {
    let (mut sr, mut szz, mut srr, mut srz, mut sn) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let dx = cellx(q, r) - cx;
                let dy = celly(r) - cy;
                let rad = (dx * dx + dy * dy).sqrt();
                let z = f.get(q, r);
                sr = sr + rad;
                szz = szz + z;
                srr = srr + rad * rad;
                srz = srz + rad * z;
                sn = sn + 1.0;
            }
        }
    }
    let den = sn * srr - sr * sr;
    let mut ad = den;
    if ad < 0.0 {
        ad = 0.0 - ad;
    }
    if ad < 0.000000001 {
        return (0.0, 0.0, 1000000.0);
    }
    let grad = (sn * srz - sr * szz) / den;
    let apex = (szz - grad * sr) / sn;
    let mut res = 0.0;
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let dx = cellx(q, r) - cx;
                let dy = celly(r) - cy;
                let rad = (dx * dx + dy * dy).sqrt();
                let mut e = apex + grad * rad - f.get(q, r);
                if e < 0.0 {
                    e = 0.0 - e;
                }
                if e > res {
                    res = e;
                }
            }
        }
    }
    (apex, 0.0 - grad, res)
}

fn ridge_at(s: &HexSet, f: &Heights, ax: f64, ay: f64, bx: f64, by: f64) -> (f64, f64, f64) {
    let t = Track::straight(ax, ay, bx, by);
    let (mut sd, mut szz, mut sdd, mut sdz, mut sn) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let d = track_distance(&t, cellx(q, r), celly(r));
                let z = f.get(q, r);
                sd = sd + d;
                szz = szz + z;
                sdd = sdd + d * d;
                sdz = sdz + d * z;
                sn = sn + 1.0;
            }
        }
    }
    let den = sn * sdd - sd * sd;
    let mut ad = den;
    if ad < 0.0 {
        ad = 0.0 - ad;
    }
    if ad < 0.000000001 {
        return (0.0, 0.0, 1000000.0);
    }
    let grad = (sn * sdz - sd * szz) / den;
    let apex = (szz - grad * sd) / sn;
    let mut res = 0.0;
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let d = track_distance(&t, cellx(q, r), celly(r));
                let mut e = apex + grad * d - f.get(q, r);
                if e < 0.0 {
                    e = 0.0 - e;
                }
                if e > res {
                    res = e;
                }
            }
        }
    }
    (apex, 0.0 - grad, res)
}

/// The eight compass offsets the climbs share, on one step size.
fn offset(k: i64, step: f64) -> (f64, f64) {
    match k {
        0 => (step, 0.0),
        1 => (0.0 - step, 0.0),
        2 => (0.0, step),
        3 => (0.0, 0.0 - step),
        4 => (step, step),
        5 => (step, 0.0 - step),
        6 => (0.0 - step, step),
        _ => (0.0 - step, 0.0 - step),
    }
}

fn roof_ridge_fit(s: &HexSet, f: &Heights) -> RoofFit {
    let mut hi = 0.0 - 1000000.0;
    let mut lo = 1000000.0;
    let (mut hx, mut hy) = (0.0, 0.0);
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let z = f.get(q, r);
                if z > hi {
                    hi = z;
                    hx = cellx(q, r);
                    hy = celly(r);
                }
                if z < lo {
                    lo = z;
                }
            }
        }
    }
    let mut band = (hi - lo) * 0.000001;
    if band < 0.0 {
        band = 0.0 - band;
    }
    let (mut ex1, mut ey1, mut ex2, mut ey2) = (hx, hy, hx, hy);
    let mut far = 0.0 - 1.0;
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) && f.get(q, r) >= hi - band {
                for r2 in s.r0..s.r0 + s.h {
                    for q2 in s.q0..s.q0 + s.w {
                        if s.get(q2, r2) && f.get(q2, r2) >= hi - band {
                            let ddx = cellx(q2, r2) - cellx(q, r);
                            let ddy = celly(r2) - celly(r);
                            let dd = ddx * ddx + ddy * ddy;
                            if dd > far {
                                far = dd;
                                ex1 = cellx(q, r);
                                ey1 = celly(r);
                                ex2 = cellx(q2, r2);
                                ey2 = celly(r2);
                            }
                        }
                    }
                }
            }
        }
    }
    let pdx = ex2 - ex1;
    let pdy = ey2 - ey1;
    let plen = (pdx * pdx + pdy * pdy).sqrt();
    if plen > 0.000001 {
        let ux = pdx / plen;
        let uy = pdy / plen;
        let mut tmin = 1000000.0;
        let mut tmax = 0.0 - 1000000.0;
        for r in s.r0..s.r0 + s.h {
            for q in s.q0..s.q0 + s.w {
                if s.get(q, r) {
                    let tp = (cellx(q, r) - ex1) * ux + (celly(r) - ey1) * uy;
                    if tp < tmin {
                        tmin = tp;
                    }
                    if tp > tmax {
                        tmax = tp;
                    }
                }
            }
        }
        ex2 = ex1 + ux * tmax;
        ey2 = ey1 + uy * tmax;
        ex1 = ex1 + ux * tmin;
        ey1 = ey1 + uy * tmin;
    }
    let c = roof_cone_fit(s, f);
    let mut best = ridge_climb(s, f, ex1, ey1, ex2, ey2);
    for t in [
        ridge_climb(s, f, hx, hy, hx, hy),
        ridge_climb(s, f, c.x, c.y, c.x, c.y),
        ridge_climb(s, f, hx, hy, c.x, c.y),
    ] {
        if t.res < best.res {
            best = t;
        }
    }
    best
}

fn ridge_climb(s: &HexSet, f: &Heights, sax: f64, say: f64, sbx: f64, sby: f64) -> RoofFit {
    let (mut ax, mut ay, mut bx, mut by) = (sax, say, sbx, sby);
    let (mut ba, mut bs, mut br) = ridge_at(s, f, ax, ay, bx, by);
    let mut step = 0.4;
    for _ in 0..5 {
        let mut moved = true;
        while moved {
            moved = false;
            for i in 0..16 {
                let (ox, oy) = offset(i % 8, step);
                let (mut nax, mut nay, mut nbx, mut nby) = (ax, ay, bx, by);
                if i < 8 {
                    nax = ax + ox;
                    nay = ay + oy;
                } else {
                    nbx = bx + ox;
                    nby = by + oy;
                }
                let (ca, cs, cr) = ridge_at(s, f, nax, nay, nbx, nby);
                if cr < br - 0.0000000001 {
                    br = cr;
                    ba = ca;
                    bs = cs;
                    ax = nax;
                    ay = nay;
                    bx = nbx;
                    by = nby;
                    moved = true;
                }
            }
        }
        step = step * 0.25;
    }
    RoofFit { kind: ROOF_RIDGE, x: ax, y: ay, z: ba, slope: bs, res: br, x2: bx, y2: by }
}

/// The highest cell's centre, the seed both point climbs start from.
fn highest(s: &HexSet, f: &Heights) -> (f64, f64) {
    let mut hi = -1000000.0;
    let (mut bx, mut by) = (0.0, 0.0);
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let z = f.get(q, r);
                if z > hi {
                    hi = z;
                    bx = cellx(q, r);
                    by = celly(r);
                }
            }
        }
    }
    (bx, by)
}

/// The four-level, eight-direction climb `roof_cone_fit` and `roof_dome_fit` share.
fn point_climb(s: &HexSet, f: &Heights, at: fn(&HexSet, &Heights, f64, f64) -> (f64, f64, f64))
    -> (f64, f64, f64, f64, f64) {
    let (mut bx, mut by) = highest(s, f);
    let (mut ba, mut bs, mut br) = at(s, f, bx, by);
    let mut step = 0.4;
    for _ in 0..4 {
        let mut moved = true;
        while moved {
            moved = false;
            for i in 0..8 {
                let (ox, oy) = offset(i, step);
                let (ca, cs, cr) = at(s, f, bx + ox, by + oy);
                if cr < br - 0.0000000001 {
                    br = cr;
                    ba = ca;
                    bs = cs;
                    bx = bx + ox;
                    by = by + oy;
                    moved = true;
                }
            }
        }
        step = step * 0.25;
    }
    (bx, by, ba, bs, br)
}

fn roof_cone_fit(s: &HexSet, f: &Heights) -> RoofFit {
    let (bx, by, ba, bs, br) = point_climb(s, f, cone_at);
    RoofFit::new(ROOF_CONE, bx, by, ba, bs, br)
}

fn dome_at(s: &HexSet, f: &Heights, cx: f64, cy: f64) -> (f64, f64, f64) {
    let (mut sv, mut su, mut svv, mut suv, mut sn) = (0.0, 0.0, 0.0, 0.0, 0.0);
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let dx = cellx(q, r) - cx;
                let dy = celly(r) - cy;
                let z = f.get(q, r);
                let v = z;
                let u = z * z + dx * dx + dy * dy;
                sv = sv + v;
                su = su + u;
                svv = svv + v * v;
                suv = suv + u * v;
                sn = sn + 1.0;
            }
        }
    }
    let den = sn * svv - sv * sv;
    let mut ad = den;
    if ad < 0.0 {
        ad = 0.0 - ad;
    }
    if ad < 0.000000001 {
        return (0.0, 0.0, 1000000.0);
    }
    let grad = (sn * suv - sv * su) / den;
    let icept = (su - grad * sv) / sn;
    let base = grad * 0.5;
    let rr2 = icept + base * base;
    if rr2 <= 0.0 {
        return (0.0, 0.0, 1000000.0);
    }
    let rad = rr2.sqrt();
    let mut res = 0.0;
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let dx = cellx(q, r) - cx;
                let dy = celly(r) - cy;
                let d2 = dx * dx + dy * dy;
                let mut pred = base;
                if d2 < rad * rad {
                    pred = base + (rad * rad - d2).sqrt();
                }
                let mut e = pred - f.get(q, r);
                if e < 0.0 {
                    e = 0.0 - e;
                }
                if e > res {
                    res = e;
                }
            }
        }
    }
    (base, rad, res)
}

fn roof_dome_fit(s: &HexSet, f: &Heights) -> RoofFit {
    let (bx, by, bb, brad, bres) = point_climb(s, f, dome_at);
    RoofFit::new(ROOF_DOME, bx, by, bb, brad, bres)
}

fn roof_match(s: &HexSet, f: &Heights, tol: f64) -> RoofFit {
    let p = roof_plane_fit(s, f);
    if p.res <= tol {
        return p;
    }
    let c = roof_cone_fit(s, f);
    if c.res <= tol {
        return c;
    }
    let d = roof_dome_fit(s, f);
    if d.res <= tol {
        return d;
    }
    let g = roof_ridge_fit(s, f);
    if g.res <= tol {
        return g;
    }
    let mut best = p;
    for cand in [c, d, g] {
        if cand.res < best.res {
            best = cand;
        }
    }
    RoofFit::new(ROOF_UNKNOWN, best.x, best.y, best.z, best.slope, best.res)
}

// ── The rows ────────────────────────────────────────────────────────

fn hip_disk() -> HexSet {
    let mut s = HexSet::chunk(0, 0, HIP_W, HIP_W);
    hexdisk_into(&mut s, 15, 15, HIP_N);
    s
}

fn hip_heights(s: &HexSet, eave: f64) -> Heights {
    let mut f = Heights::new(0, 0, HIP_W, HIP_W);
    roof_hip(s, &mut f, eave, 0.5);
    f
}

fn match_op(s: &HexSet, f: &Heights) -> [i64; 8] {
    let fit = roof_match(s, f, 0.000001);
    [fit.kind, micro(fit.x), micro(fit.y), micro(fit.z), micro(fit.slope), micro(fit.res),
     micro(fit.x2), micro(fit.y2)]
}

fn bench_match(n: i64) -> Row {
    let s = hip_disk();
    let f0 = hip_heights(&s, 3.0);
    let f1 = hip_heights(&s, 3.25);
    let (us, sink) = timed(n, |r| match_op(&s, if (r & 1) == 0 { &f0 } else { &f1 })[5]);
    Row { name: "roof_match", iters: n, us, px: s.count,
          hash: fnv(FNV_OFFSET, &match_op(&s, black_box(&f0))), sink }
}

fn full_window() -> HexSet {
    let mut s = HexSet::chunk(0, 0, CONE_W, CONE_W);
    for r in 0..CONE_W {
        for q in 0..CONE_W {
            s.set(q, r, true);
        }
    }
    s
}

fn cone_op(s: &HexSet, f: &mut Heights, r: i64) -> i64 {
    let salt = ((r & 1) as f64) * 0.25;
    for c in 0..CONES {
        roof_cone(s, f, 100.0 + salt + (c as f64) * 0.5, 96.0, 20.0, 0.3, 60.0);
    }
    micro(f.get(0, 0)) + micro(f.get(CONE_W - 1, CONE_W - 1))
}

fn bench_cone(n: i64) -> Row {
    let s = full_window();
    let mut f = Heights::new(0, 0, CONE_W, CONE_W);
    let (us, sink) = timed(n, |r| cone_op(&s, &mut f, r));
    cone_op(&s, &mut f, black_box(0));
    let v: Vec<i64> = f.z.iter().map(|&z| micro(z)).collect();
    Row { name: "roof_cone", iters: n, us, px: CONES * CONE_W * CONE_W, hash: fnv(FNV_OFFSET, &v), sink }
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
    let rows = [bench_match(n), bench_cone(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
