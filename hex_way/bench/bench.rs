// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_way-reference — the pure-Rust twin of the `hex_way` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workload with
// the SAME arithmetic, in the same order, and prints the same row, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The port of src/hex_way.loft's `track_distance` and what it calls (`seg_distance`,
// `seg_point`, `seg_len`, `ang_wrap`), over a `Vec<[f64; 6]>` segment table; and of
// `way_stamp` with `nearest_seg`, hex_edge's `edge_point` / `edge_block_full`, and the
// hex_field storage they write (`HexSet`, the `EdgeSet` material and surface slots with
// the blocked count, `edgeset_digest`).  `black_box` guards each op's INPUT (the
// repetition number) and the sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const QUERIES: i64 = 100000;
const WIN: i64 = 96;
const STAMPS: i64 = 4;

const WAY_STRAIGHT: i64 = 1;
const WAY_ARC: i64 = 2;
const TAU: f64 = 6.283185307179586;

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

// ── hex_way.loft ────────────────────────────────────────────────────

struct Track {
    kind: Vec<i64>,
    p: Vec<[f64; 6]>,
}

impl Track {
    fn new() -> Track {
        Track { kind: Vec::new(), p: Vec::new() }
    }
    fn straight(&mut self, x0: f64, y0: f64, x1: f64, y1: f64) {
        self.kind.push(WAY_STRAIGHT);
        self.p.push([x0, y0, x1, y1, 0.0, 0.0]);
    }
    fn arc(&mut self, cx: f64, cy: f64, r: f64, a0: f64, a1: f64) {
        self.kind.push(WAY_ARC);
        self.p.push([cx, cy, r, a0, a1, 0.0]);
    }
}

fn ang_wrap(d: f64) -> f64 {
    let mut v = d;
    while v < 0.0 {
        v += TAU;
    }
    while v >= TAU {
        v -= TAU;
    }
    v
}

fn seg_len(t: &Track, i: usize) -> f64 {
    let p = &t.p[i];
    if t.kind[i] == WAY_STRAIGHT {
        let dx = p[2] - p[0];
        let dy = p[3] - p[1];
        return (dx * dx + dy * dy).sqrt();
    }
    let d = (p[4] - p[3]).abs();
    d * p[2]
}

fn seg_point(t: &Track, i: usize, s: f64) -> (f64, f64) {
    let p = &t.p[i];
    if t.kind[i] == WAY_STRAIGHT {
        let seg_l = seg_len(t, i);
        if seg_l < 0.000001 {
            return (p[0], p[1]);
        }
        let u = s / seg_l;
        return (p[0] + (p[2] - p[0]) * u, p[1] + (p[3] - p[1]) * u);
    }
    let r = p[2];
    let (a0, a1) = (p[3], p[4]);
    let dir = if a1 < a0 { -1.0 } else { 1.0 };
    let a = a0 + (s / r) * dir;
    (p[0] + r * a.cos(), p[1] + r * a.sin())
}

fn seg_distance(t: &Track, i: usize, px: f64, py: f64) -> f64 {
    let p = &t.p[i];
    if t.kind[i] == WAY_STRAIGHT {
        let (ax, ay) = (p[0], p[1]);
        let (vx, vy) = (p[2] - ax, p[3] - ay);
        let len2 = vx * vx + vy * vy;
        let mut u = 0.0;
        if len2 > 0.000000001 {
            u = ((px - ax) * vx + (py - ay) * vy) / len2;
        }
        u = u.clamp(0.0, 1.0);
        let dx = px - (ax + u * vx);
        let dy = py - (ay + u * vy);
        return (dx * dx + dy * dy).sqrt();
    }
    let (cx, cy, r) = (p[0], p[1], p[2]);
    let (dx, dy) = (px - cx, py - cy);
    let dd = (dx * dx + dy * dy).sqrt();
    let a = dy.atan2(dx);
    let (mut lo, mut hi) = (p[3], p[4]);
    if hi < lo {
        std::mem::swap(&mut lo, &mut hi);
    }
    if ang_wrap(a - lo) <= hi - lo {
        return (dd - r).abs();
    }
    let (e0x, e0y) = seg_point(t, i, 0.0);
    let (e1x, e1y) = seg_point(t, i, seg_len(t, i));
    let d0 = ((px - e0x) * (px - e0x) + (py - e0y) * (py - e0y)).sqrt();
    let d1 = ((px - e1x) * (px - e1x) + (py - e1y) * (py - e1y)).sqrt();
    if d0 < d1 { d0 } else { d1 }
}

fn track_distance(t: &Track, px: f64, py: f64) -> f64 {
    let mut best = 1000000.0;
    for i in 0..t.kind.len() {
        let d = seg_distance(t, i, px, py);
        if d < best {
            best = d;
        }
    }
    best
}

fn nearest_seg(t: &Track, px: f64, py: f64) -> i64 {
    let mut best = -1;
    let mut bd = 1000000.0;
    for i in 0..t.kind.len() {
        let d = seg_distance(t, i, px, py);
        if d < bd - 0.000000001 {
            bd = d;
            best = i as i64;
        }
    }
    best
}

// ── hex_field.loft: the cell and edge layers ────────────────────────

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

struct EdgeSet {
    q0: i64,
    r0: i64,
    gw: i64,
    gh: i64,
    mat: Vec<u8>,
    surf: Vec<i32>,
    count: i64,
    refused: i64,
}

impl EdgeSet {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> EdgeSet {
        let (gw, gh) = (w + 2, h + 2);
        let n = (gw * gh * 3) as usize;
        EdgeSet { q0, r0, gw, gh, mat: vec![0; n], surf: vec![0; n], count: 0, refused: 0 }
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

    fn set_mat(&mut self, qa: i64, ra: i64, qb: i64, rb: i64, mat: i64) {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 {
            return;
        }
        if !(0..=255).contains(&mat) {
            self.refused += 1;
            return;
        }
        self.mat[i as usize] = mat as u8;
    }

    fn surf_at(&self, qa: i64, ra: i64, qb: i64, rb: i64) -> i64 {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 {
            return 0;
        }
        self.surf[i as usize] as i64
    }

    fn set_surf(&mut self, qa: i64, ra: i64, qb: i64, rb: i64, surf: i64) {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 {
            return;
        }
        let was = self.surf[i as usize] as i64;
        if was == 0 && surf != 0 {
            self.count += 1;
        }
        if was != 0 && surf == 0 {
            self.count -= 1;
        }
        self.surf[i as usize] = surf as i32;
    }

    fn digest(&self) -> i64 {
        let mut h: i64 = 1469598103;
        for i in 0..self.surf.len() {
            h = (h * 31 + self.mat[i] as i64) % 1000000007;
            h = (h * 31 + self.surf[i] as i64) % 1000000007;
        }
        h
    }
}

// ── hex_edge.loft, and the stamp ────────────────────────────────────

fn edge_point(qa: i64, ra: i64, qb: i64, rb: i64) -> (f64, f64) {
    (((lattice_k(qa, ra) + lattice_k(qb, rb)) as f64) * 0.4330127018922193,
     ((lattice_m(ra) + lattice_m(rb)) as f64) * 0.25)
}

fn edge_block_full(e: &mut EdgeSet, qa: i64, ra: i64, qb: i64, rb: i64, surf: i64, mat: i64) {
    if e.surf_at(qa, ra, qb, rb) == 0 {
        e.set_surf(qa, ra, qb, rb, surf);
    }
    e.set_mat(qa, ra, qb, rb, mat);
}

fn way_stamp(t: &Track, halfwidth: f64, s: &mut HexSet, e: &mut EdgeSet, first_surf: i64, mat: i64) {
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            let px = (lattice_k(q, r) as f64) * 0.8660254037844386;
            let py = (lattice_m(r) as f64) * 0.5;
            if track_distance(t, px, py) <= halfwidth {
                s.set(q, r, true);
            }
        }
    }
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                for d in 0..6 {
                    let nq = nb_q(q, r, d);
                    let nr = nb_r(r, d);
                    if !s.get(nq, nr) {
                        let (ex, ey) = edge_point(q, r, nq, nr);
                        let si = nearest_seg(t, ex, ey);
                        if si >= 0 {
                            edge_block_full(e, q, r, nq, nr, first_surf + si, mat);
                        }
                    }
                }
            }
        }
    }
}

// ── The rows ────────────────────────────────────────────────────────

fn road() -> Track {
    let mut t = Track::new();
    for k in 0..8 {
        let x = (k as f64) * 20.0;
        t.straight(x, 0.0, x + 10.0, 5.0);
        if (k & 1) == 0 {
            t.arc(x + 14.0, 10.0, 6.0, -1.2, 1.9);
        } else if k == 7 {
            t.arc(x + 14.0, 10.0, 6.0, -6.9, -9.4);
        } else {
            t.arc(x + 14.0, 10.0, 6.0, 2.6, -0.4);
        }
    }
    t
}

fn distance_op(t: &Track, r: i64) -> [i64; 2] {
    let salt = ((r & 1) as f64) * 0.25;
    let mut sum = 0.0;
    let mut near = 0i64;
    for i in 0..QUERIES {
        let x = ((i * 7919) % 20011) as f64 * 0.009 - 10.0 + salt;
        let y = ((i * 104729) % 19997) as f64 * 0.0025 - 15.0;
        let d = track_distance(t, x, y);
        sum += d;
        if d < 1.0 {
            near += 1;
        }
    }
    [(sum * 1000000.0) as i64, near]
}

fn bench_distance(n: i64) -> Row {
    let t = road();
    let (us, sink) = timed(n, |r| distance_op(&t, r)[1]);
    Row { name: "track_distance", iters: n, us, px: QUERIES,
          hash: fnv(FNV_OFFSET, &distance_op(&t, black_box(0))), sink }
}

fn stamp_road(dx: f64) -> Track {
    let mut t = Track::new();
    for k in 0..6 {
        let x = (k as f64) * 20.0 + 10.0 + dx;
        t.straight(x, 40.0, x + 10.0, 45.0);
        if (k & 1) == 0 {
            t.arc(x + 14.0, 50.0, 6.0, -1.2, 1.9);
        } else {
            t.arc(x + 14.0, 50.0, 6.0, 2.6, -0.4);
        }
    }
    t
}

fn stamp_op(t: &Track) -> [i64; 4] {
    let mut s = HexSet::chunk(0, 0, WIN, WIN);
    let mut e = EdgeSet::new(0, 0, WIN, WIN);
    for _ in 0..STAMPS {
        way_stamp(t, 2.5, &mut s, &mut e, 1, 3);
    }
    [s.count, e.count, e.refused, e.digest()]
}

fn bench_stamp(n: i64) -> Row {
    let t0 = stamp_road(0.0);
    let t1 = stamp_road(1.7320508075688772);
    let (us, sink) = timed(n, |r| stamp_op(if (r & 1) == 0 { &t0 } else { &t1 })[1]);
    Row { name: "way_stamp", iters: n, us, px: STAMPS * WIN * WIN,
          hash: fnv(FNV_OFFSET, &stamp_op(black_box(&t0))), sink }
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
    let rows = [bench_distance(n), bench_stamp(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
