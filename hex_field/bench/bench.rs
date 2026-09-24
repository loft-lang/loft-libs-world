// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_field-reference — the pure-Rust twin of the `hex_field` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports, all from src/hex_field.loft: the `HexSet` and `EdgeSet` storage, `nb_q`/`nb_r`,
// the six-way direction search `eg_dir_from` behind `eg_index`, `edge_set_mat`/`edge_mat`,
// `edgeset_count`, `hexdisk_into`/`hex_dist`, and `trace` — its O(n^2) stitch over four
// `Vec<i64>` plus a `Vec<bool>`, and its canonical-start rotation.  `black_box` guards
// each op's INPUT and the sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const FIELD: i64 = 64;
const COUNTS: i64 = 100;
const TRACES: i64 = 50;

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

// ── hex_field.loft ──────────────────────────────────────────────────

struct HexSet {
    q0: i64,
    r0: i64,
    w: i64,
    h: i64,
    cells: Vec<bool>,
}

impl HexSet {
    fn chunk(q0: i64, r0: i64, w: i64, h: i64) -> HexSet {
        HexSet { q0, r0, w, h, cells: vec![false; (w * h) as usize] }
    }
    fn centred(w: i64) -> HexSet {
        HexSet::chunk(-w, -w, 2 * w + 1, 2 * w + 1)
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
        if i >= 0 {
            self.cells[i as usize] = on;
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

fn corner_k(i: i64) -> i64 {
    [0, -1, -1, 0, 1, 1][i as usize]
}

fn corner_m(i: i64) -> i64 {
    [2, 1, -1, -2, -1, 1][i as usize]
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
    w: i64,
    h: i64,
    gw: i64,
    gh: i64,
    mat: Vec<u8>,
}

impl EdgeSet {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> EdgeSet {
        let (gw, gh) = (w + 2, h + 2);
        EdgeSet { q0, r0, w, h, gw, gh, mat: vec![0; (gw * gh * 3) as usize] }
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
        if i < 0 || !(0..=EDGE_MAT_MAX).contains(&mat) {
            return;
        }
        self.mat[i as usize] = mat as u8;
    }

    fn mat_at(&self, qa: i64, ra: i64, qb: i64, rb: i64) -> i64 {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 {
            return 0;
        }
        self.mat[i as usize] as i64
    }

    /// How many edges carry a material: the canonical slots of every in-chunk cell.
    fn count(&self) -> i64 {
        let mut n = 0;
        for iq in 0..self.w {
            for ir in 0..self.h {
                let cq = self.q0 + iq;
                let cr = self.r0 + ir;
                for s in 0..3 {
                    let d = [0, 2, 3][s];
                    if self.mat_at(cq, cr, nb_q(cq, cr, d), nb_r(cr, d)) != 0 {
                        n += 1;
                    }
                }
            }
        }
        n
    }
}

struct VecMap {
    k: Vec<i64>,
    m: Vec<i64>,
    start: Vec<i64>,
    loops: i64,
}

fn trace(s: &HexSet) -> VecMap {
    let mut ak: Vec<i64> = Vec::new();
    let mut am: Vec<i64> = Vec::new();
    let mut bk: Vec<i64> = Vec::new();
    let mut bm: Vec<i64> = Vec::new();
    let mut used: Vec<bool> = Vec::new();
    for r in s.r0..s.r0 + s.h {
        for q in s.q0..s.q0 + s.w {
            if s.get(q, r) {
                let k0 = lattice_k(q, r);
                let m0 = lattice_m(r);
                for d in 0..6 {
                    let nq = nb_q(q, r, d);
                    let nr = nb_r(r, d);
                    if !s.get(nq, nr) {
                        let i = edge_of(lattice_k(nq, nr) - k0, lattice_m(nr) - m0);
                        if i >= 0 {
                            let j = if i + 1 > 5 { 0 } else { i + 1 };
                            ak.push(k0 + corner_k(i));
                            am.push(m0 + corner_m(i));
                            bk.push(k0 + corner_k(j));
                            bm.push(m0 + corner_m(j));
                            used.push(false);
                        }
                    }
                }
            }
        }
    }
    let ne = ak.len();

    // Stitch: each step scans ALL edges for the unused one leaving the current vertex.
    let mut vk: Vec<i64> = Vec::new();
    let mut vm: Vec<i64> = Vec::new();
    let mut st: Vec<i64> = Vec::new();
    let mut nloops = 0i64;
    let mut done = 0usize;
    let mut budget = ne as i64 + 1;
    while done < ne {
        let mut seed: i64 = -1;
        for e in 0..ne {
            if seed < 0 && !used[e] {
                seed = e as i64;
            }
        }
        if seed < 0 {
            done = ne;
        }
        if seed >= 0 {
            st.push(vk.len() as i64);
            nloops += 1;
            let (sk, sm) = (ak[seed as usize], am[seed as usize]);
            let (mut ck, mut cm) = (sk, sm);
            let mut going = true;
            while going {
                budget -= 1;
                if budget < 0 {
                    going = false;
                    done = ne;
                }
                let mut pick: i64 = -1;
                for e in 0..ne {
                    if pick < 0 && !used[e] && ak[e] == ck && am[e] == cm {
                        pick = e as i64;
                    }
                }
                if pick < 0 {
                    going = false;
                }
                if pick >= 0 && budget >= 0 {
                    let p = pick as usize;
                    used[p] = true;
                    done += 1;
                    vk.push(ck);
                    vm.push(cm);
                    ck = bk[p];
                    cm = bm[p];
                    if ck == sk && cm == sm {
                        going = false;
                    }
                }
            }
        }
    }
    st.push(vk.len() as i64);

    // Canonical start: rotate each loop to begin at its lexicographically smallest vertex.
    for li in 0..nloops as usize {
        let a = st[li] as usize;
        let n = st[li + 1] as usize - a;
        if n > 1 {
            let mut best = 0;
            for cj in 1..n {
                let (bk2, bm2) = (vk[a + best], vm[a + best]);
                let (jk, jm) = (vk[a + cj], vm[a + cj]);
                if jk < bk2 || (jk == bk2 && jm < bm2) {
                    best = cj;
                }
            }
            if best > 0 {
                vk[a..a + n].rotate_left(best);
                vm[a..a + n].rotate_left(best);
            }
        }
    }
    VecMap { k: vk, m: vm, start: st, loops: nloops }
}

// ── The rows ────────────────────────────────────────────────────────

fn material_field(salt: i64) -> EdgeSet {
    let mut e = EdgeSet::new(0, 0, FIELD, FIELD);
    for r in 0..FIELD {
        for q in 0..FIELD {
            for d in [0, 2, 3] {
                let m = (q * 7 + r * 3 + d + salt) % 5;
                e.set_mat(q, r, nb_q(q, r, d), nb_r(r, d), m);
            }
        }
    }
    e
}

fn count_op(e: &EdgeSet) -> [i64; 2] {
    let mut sum = 0;
    let mut last = 0;
    for _ in 0..COUNTS {
        last = black_box(e).count();
        sum += last;
    }
    [last, sum]
}

fn bench_count(n: i64) -> Row {
    let e0 = material_field(0);
    let e1 = material_field(1);
    let (us, sink) = timed(n, |r| count_op(if (r & 1) == 0 { &e0 } else { &e1 })[0]);
    Row { name: "edgeset_count", iters: n, us, px: COUNTS * FIELD * FIELD * 3,
          hash: fnv(FNV_OFFSET, &count_op(black_box(&e0))), sink }
}

fn disk(cq: i64) -> HexSet {
    let mut s = HexSet::centred(26);
    hexdisk_into(&mut s, cq, 0, 24);
    s
}

fn trace_op(s: &HexSet) -> Vec<i64> {
    let mut ints: Vec<i64> = Vec::new();
    let mut verts = 0i64;
    for t in 0..TRACES {
        let v = trace(s);
        verts += v.k.len() as i64;
        if t == TRACES - 1 {
            ints = vec![v.loops, v.k.len() as i64];
            ints.extend(&v.k);
            ints.extend(&v.m);
            ints.extend(&v.start);
        }
    }
    ints.push(verts);
    ints
}

fn bench_trace(n: i64) -> Row {
    let s0 = disk(0);
    let s1 = disk(1);
    let (us, sink) = timed(n, |r| trace_op(if (r & 1) == 0 { &s0 } else { &s1 })[1]);
    let one = trace_op(black_box(&s0));
    Row { name: "trace", iters: n, us, px: TRACES * one[1], hash: fnv(FNV_OFFSET, &one), sink }
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
    let rows = [bench_count(n), bench_trace(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
