// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_edge-reference — the pure-Rust twin of the `hex_edge` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports: `passable`, `edge_block` and `edges_cut` from src/hex_edge.loft, and the
// storage they sit on from hex_field (`HexSet`, `EdgeSet`, `nb_q`/`nb_r`, the six-way
// direction search `eg_dir_from`, `eg_index`, `edge_surf`/`edge_set_surf`,
// `edgeset_digest`, `hexdisk_into`/`hex_dist`) — the same slot layout, the same search.
// `black_box` guards each op's INPUT (the repetition number) and the sink — never anything
// inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const FIELD: i64 = 64;
const WALK: i64 = 1000000;
const WIN: i64 = 96;
const CUTS: i64 = 20;

const SURF_NONE: i64 = 65535;

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

// ── hex_field.loft: cells, lattice, the edge layer ──────────────────

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
        if i < 0 {
            return false;
        }
        self.cells[i as usize]
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

struct EdgeSet {
    q0: i64,
    r0: i64,
    gw: i64,
    gh: i64,
    surf: Vec<i32>,
    count: i64,
}

impl EdgeSet {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> EdgeSet {
        let (gw, gh) = (w + 2, h + 2);
        EdgeSet { q0, r0, gw, gh, surf: vec![0; (gw * gh * 3) as usize], count: 0 }
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

    /// The order-sensitive checksum over both slots; the material slot is all zero here.
    fn digest(&self) -> i64 {
        let mut h: i64 = 1469598103;
        for &s in &self.surf {
            h = (h * 31) % 1000000007;
            h = (h * 31 + s as i64) % 1000000007;
        }
        h
    }
}

// ── hex_edge.loft ───────────────────────────────────────────────────

fn edge_block(e: &mut EdgeSet, qa: i64, ra: i64, qb: i64, rb: i64) {
    if e.surf_at(qa, ra, qb, rb) == 0 {
        e.set_surf(qa, ra, qb, rb, SURF_NONE);
    }
}

fn passable(e: &EdgeSet, qa: i64, ra: i64, qb: i64, rb: i64) -> bool {
    e.surf_at(qa, ra, qb, rb) == 0
}

fn edges_cut(s: &HexSet, e: &mut EdgeSet) {
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                for d in 0..6 {
                    let nq = nb_q(q, r, d);
                    let nr = nb_r(r, d);
                    if !s.get(nq, nr) {
                        edge_block(e, q, r, nq, nr);
                    }
                }
            }
        }
    }
}

// ── The rows ────────────────────────────────────────────────────────

fn walled_field() -> EdgeSet {
    let mut s = HexSet::chunk(0, 0, FIELD, FIELD);
    hexdisk_into(&mut s, 16, 16, 8);
    hexdisk_into(&mut s, 44, 20, 10);
    hexdisk_into(&mut s, 30, 46, 12);
    let mut e = EdgeSet::new(0, 0, FIELD, FIELD);
    edges_cut(&s, &mut e);
    e
}

fn walk_op(e: &EdgeSet, r: i64) -> [i64; 4] {
    let mut q = 2 + (r & 1);
    let mut rr = 2i64;
    let mut s = 12345i64;
    let mut moves = 0i64;
    let mut blocked = 0i64;
    for _ in 0..WALK {
        s = (s * 1103515245 + 12345) & 0x7FFF_FFFF;
        let d = (s >> 16) % 6;
        let nq = nb_q(q, rr, d);
        let nr = nb_r(rr, d);
        if passable(e, q, rr, nq, nr) {
            if (0..FIELD).contains(&nq) && (0..FIELD).contains(&nr) {
                q = nq;
                rr = nr;
                moves += 1;
            }
        } else {
            blocked += 1;
        }
    }
    [q, rr, moves, blocked]
}

fn bench_passable(n: i64) -> Row {
    let e = walled_field();
    let (us, sink) = timed(n, |r| walk_op(&e, r)[3]);
    Row { name: "passable", iters: n, us, px: WALK, hash: fnv(FNV_OFFSET, &walk_op(&e, black_box(0))), sink }
}

fn disk(cq: i64) -> HexSet {
    let mut s = HexSet::chunk(0, 0, WIN, WIN);
    hexdisk_into(&mut s, cq, 48, 45);
    s
}

fn cut_op(s: &HexSet) -> [i64; 2] {
    let mut e = EdgeSet::new(0, 0, WIN, WIN);
    for _ in 0..CUTS {
        edges_cut(s, &mut e);
    }
    [e.count, e.digest()]
}

fn bench_cut(n: i64) -> Row {
    let s0 = disk(47);
    let s1 = disk(48);
    let (us, sink) = timed(n, |r| cut_op(if (r & 1) == 0 { &s0 } else { &s1 })[0]);
    Row { name: "edges_cut", iters: n, us, px: CUTS * WIN * WIN,
          hash: fnv(FNV_OFFSET, &cut_op(black_box(&s0))), sink }
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
    let rows = [bench_passable(n), bench_cut(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
