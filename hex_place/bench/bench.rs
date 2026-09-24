// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_place-reference — the pure-Rust twin of the `hex_place` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports: `combine_cut` and `field_union` from src/hexcombine.loft, and the storage they
// sit on from hex_field (`HexSet`, `EdgeSet` with its material and surface slots, `nb_q`/
// `nb_r`, the six-way direction search `eg_dir_from`, `eg_index`, `edge_set_mat`,
// `edgeset_count_all`, `edgeset_digest`, `hexdisk_into`/`hex_dist`).  Idiomatic where the
// library allocates: the union is `vec![false; w*h]` filled by one OR over the two cell
// vectors, and `combine_cut` reads the union on the fly in one fused pass over the two
// `Vec<bool>` instead of building it first.  `black_box` guards each op's INPUT (the
// repetition number) and the sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const CUT_W: i64 = 64;
const CUTS: i64 = 20;
const UNION_W: i64 = 128;
const UNIONS: i64 = 10;

const EDGE_MAT_MAX: i64 = 255;

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

// ── hex_field: cells, lattice, the edge layer ───────────────────────

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
    mat: Vec<u8>,
    surf: Vec<i32>,
    refused: i64,
}

impl EdgeSet {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> EdgeSet {
        let (gw, gh) = (w + 2, h + 2);
        let n = (gw * gh * 3) as usize;
        EdgeSet { q0, r0, gw, gh, mat: vec![0; n], surf: vec![0; n], refused: 0 }
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
        if i < 0 {
            return;
        }
        if !(0..=EDGE_MAT_MAX).contains(&mat) {
            self.refused += 1;
            return;
        }
        self.mat[i as usize] = mat as u8;
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

// ── hexcombine.loft ─────────────────────────────────────────────────

/// `a ∪ b` over `a`'s window (the bench's two sets share it): one OR over the cell vectors.
fn field_union(a: &HexSet, b: &HexSet) -> HexSet {
    let cells: Vec<bool> = a.cells.iter().zip(&b.cells).map(|(&x, &y)| x || y).collect();
    let count = cells.iter().filter(|&&c| c).count() as i64;
    HexSet { q0: a.q0, r0: a.r0, w: a.w, h: a.h, cells, count }
}

/// Mark all, cut once — the union read on the fly rather than built.
fn combine_cut(a: &HexSet, ma: i64, b: &HexSet, mb: i64, e: &mut EdgeSet) -> i64 {
    let mut n = 0;
    for r in a.r0..a.r0 + a.h {
        for q in a.q0..a.q0 + a.w {
            let ina = a.get(q, r);
            let inb = b.get(q, r);
            if !(ina || inb) {
                continue;
            }
            for d in 0..6 {
                let nq = nb_q(q, r, d);
                let nr = nb_r(r, d);
                if !(a.get(nq, nr) || b.get(nq, nr)) {
                    let mut m = ma;
                    if inb && !ina {
                        m = mb;
                    }
                    if ina && inb && mb < ma {
                        m = mb;
                    }
                    e.set_mat(q, r, nq, nr, m);
                    n += 1;
                }
            }
        }
    }
    n
}

// ── The rows ────────────────────────────────────────────────────────

fn footprint(w: i64, cq: i64, cr: i64, n: i64) -> HexSet {
    let mut s = HexSet::chunk(0, 0, w, w);
    hexdisk_into(&mut s, cq, cr, n);
    s
}

fn cut_op(a: &HexSet, b: &HexSet, e: &mut EdgeSet) -> i64 {
    let mut n = 0;
    for _ in 0..CUTS {
        n += combine_cut(a, 3, b, 5, e);
    }
    n
}

fn bench_cut(n: i64) -> Row {
    let a0 = footprint(CUT_W, 26, 32, 20);
    let b0 = footprint(CUT_W, 38, 32, 20);
    let a1 = footprint(CUT_W, 27, 32, 20);
    let b1 = footprint(CUT_W, 39, 32, 20);
    let mut e = EdgeSet::new(0, 0, CUT_W, CUT_W);
    let (us, sink) = timed(n, |r| if r & 1 == 0 { cut_op(&a0, &b0, &mut e) } else { cut_op(&a1, &b1, &mut e) });
    let mut he = EdgeSet::new(0, 0, CUT_W, CUT_W);
    let hn = combine_cut(black_box(&a0), 3, &b0, 5, &mut he);
    Row { name: "combine_cut", iters: n, us, px: CUTS * CUT_W * CUT_W,
          hash: fnv(FNV_OFFSET, &[hn, he.count_all(), he.digest()]), sink }
}

fn union_op(a: &HexSet, b: &HexSet) -> i64 {
    let mut n = 0;
    for _ in 0..UNIONS {
        n += field_union(a, b).count;
    }
    n
}

fn union_hash(a: &HexSet, b: &HexSet) -> i64 {
    let u = field_union(a, b);
    let mut v: Vec<i64> = (0..UNION_W).map(|r| (0..UNION_W).filter(|&q| u.get(q, r)).count() as i64).collect();
    v.push(u.count);
    fnv(FNV_OFFSET, &v)
}

fn bench_union(n: i64) -> Row {
    let a0 = footprint(UNION_W, 50, 64, 40);
    let b0 = footprint(UNION_W, 76, 64, 40);
    let a1 = footprint(UNION_W, 51, 64, 40);
    let b1 = footprint(UNION_W, 77, 64, 40);
    let (us, sink) = timed(n, |r| if r & 1 == 0 { union_op(&a0, &b0) } else { union_op(&a1, &b1) });
    Row { name: "field_union", iters: n, us, px: UNIONS * UNION_W * UNION_W,
          hash: union_hash(black_box(&a0), &b0), sink }
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
    let rows = [bench_cut(n), bench_union(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
