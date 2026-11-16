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
// And the turn and the blended seat: `cells_turn` (by walls), `turned_point` and `marked_turn`
// (with hex_draw's `cells_place_opening` and `cells_openings`) from
// src/cellturn.loft, with hex_draw's `cells_built` and all it stands on (the boundary walked,
// the runs, the two wall solvers, the miter); `seat_blend` from src/hexseat.loft on a plain
// `Vec<f64>` of heights.
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

    fn get_mat(&self, qa: i64, ra: i64, qb: i64, rb: i64) -> i64 {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 { 0 } else { self.mat[i as usize] as i64 }
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
/// The library's algorithm (rule 1): a fresh chunk on `a`'s window, then per cell two
/// windowed reads and a windowed write.  For the library's author: where both sets share one
/// window — every caller in this bench, and `combine_cut`'s — the union is an element-wise OR
/// of the two cell vectors and a count, which rustc vectorises (~0.15 ns a cell against this
/// form's per-cell index arithmetic).
fn field_union(a: &HexSet, b: &HexSet) -> HexSet {
    let mut u = HexSet::chunk(a.q0, a.r0, a.w, a.h);
    for r in a.r0..a.r0 + a.h {
        for q in a.q0..a.q0 + a.w {
            if a.get(q, r) || b.get(q, r) {
                u.set(q, r, true);
            }
        }
    }
    u
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

// ── hex_field and hex_draw, as far as the turn stands on them ───────

fn lattice_k(q: i64, r: i64) -> i64 {
    2 * q + (r & 1)
}

fn lattice_m(r: i64) -> i64 {
    3 * r
}

const CORNER_K: [i64; 6] = [0, -1, -1, 0, 1, 1];
const CORNER_M: [i64; 6] = [2, 1, -1, -2, -1, 1];

#[derive(Clone, Copy)]
struct WallSurface {
    dk: i64,
    dm: i64,
    num_k: i64,
    num_m: i64,
    den: i64,
    #[allow(dead_code)]
    n: i64,
}

impl WallSurface {
    fn mean_x(&self) -> f64 {
        (self.num_k as f64 / self.den as f64) * 0.8660254037844386
    }
    fn mean_y(&self) -> f64 {
        (self.num_m as f64 / self.den as f64) * 0.5
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

#[allow(dead_code)]
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

// ── cellturn.loft ───────────────────────────────────────────────────

const TURN_ROW_ODD: f64 = 1.5;
const TURN_ROW_EVEN: f64 = 0.8660254037844386;

struct Turned {
    cells: HexSet,
    pk: i64,
    pm: i64,
    steps: i64,
    sx: f64,
    sy: f64,
}

fn turn_offsets(cells: &HexSet, pk: i64, pm: i64) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    for r in cells.r0..cells.r0 + cells.h {
        for q in cells.q0..cells.q0 + cells.w {
            if cells.get(q, r) {
                out.push((lattice_k(q, r) - pk, lattice_m(r) - pm));
            }
        }
    }
    out
}

fn turn_reach(cells: &HexSet, pq: i64, pr: i64) -> i64 {
    let mut reach = 0;
    for r in cells.r0..cells.r0 + cells.h {
        for q in cells.q0..cells.q0 + cells.w {
            if cells.get(q, r) {
                reach = reach.max((q - pq).abs() + (r - pr).abs());
            }
        }
    }
    reach + 3
}

fn to_line(x: f64, row: f64, off: f64) -> f64 {
    let k = ((x + off) / row).floor();
    k * row + row * 0.5
}

fn best_offset(lines: &[f64], row: f64) -> f64 {
    let mut best = 0.0;
    let mut best_err = -1.0;
    for o in [0.0, row * 0.5] {
        let mut err = 0.0;
        for &x in lines {
            err += (to_line(x, row, o) - o - x).abs();
        }
        if best_err < 0.0 || err < best_err - 0.000001 {
            best = o;
            best_err = err;
        }
    }
    best
}

fn turn_in_poly(poly: &[(f64, f64)], x: f64, y: f64) -> bool {
    let n = poly.len();
    let mut inside = false;
    for i in 0..n {
        let (x1, y1) = poly[(i + n - 1) % n];
        let (x2, y2) = poly[i];
        if (y1 > y) != (y2 > y) {
            let cx = x1 + (y - y1) * (x2 - x1) / (y2 - y1);
            if x < cx {
                inside = !inside;
            }
        }
    }
    inside
}

// One odd step, from the drawn walls: each wall to the nearest line it can stand on in the
// turned lattice, the cells inside, and the offset the structure was moved by.
fn turn_thirty(cells: &HexSet, pq: i64, pr: i64, r: i64) -> (Vec<(i64, i64)>, f64, f64) {
    let b = cells_built(cells, 1, -1);
    let near: Vec<i64> = (0..b.boundary.len()).map(|i| edge_nearest_heading(&b.boundary, i)).collect();
    let mut c = axes_by(&near);
    if c < 0 {
        c = 0;
    }
    let pk = lattice_k(pq, pr);
    let pm = lattice_m(pr);
    let px = pk as f64 * 0.8660254037844386;
    let py = pm as f64 * 0.5;
    let a0 = c as f64 * std::f64::consts::PI / 6.0;
    let (co0, si0) = (a0.cos(), a0.sin());
    let corners: Vec<(f64, f64)> = b.corners.iter().map(|p| ((p.0 - px) * co0 + (p.1 - py) * si0, -(p.0 - px) * si0 + (p.1 - py) * co0)).collect();
    let row_x = if (c + 1) % 2 == 1 { TURN_ROW_ODD } else { TURN_ROW_EVEN };
    let row_y = if (c + 4) % 2 == 1 { TURN_ROW_ODD } else { TURN_ROW_EVEN };
    let a1 = (c + 1) as f64 * std::f64::consts::PI / 6.0;
    let (co1, si1) = (a1.cos(), a1.sin());
    // each wall to the nearest line; the structure on its pivot or half a row beside it
    let n = corners.len();
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for i in 0..n {
        let (ax, ay) = corners[(i + n - 1) % n];
        let (bx, by) = corners[i];
        if (bx - ax).abs() > (by - ay).abs() {
            ys.push((ay + by) * 0.5);
        } else {
            xs.push((ax + bx) * 0.5);
        }
    }
    let off_x = best_offset(&xs, row_x);
    let off_y = best_offset(&ys, row_y);
    let poly: Vec<(f64, f64)> = corners.iter().map(|p| (to_line(p.0, row_x, off_x), to_line(p.1, row_y, off_y))).collect();
    let sx = off_x * co1 - off_y * si1;
    let sy = off_x * si1 + off_y * co1;
    let mut out = Vec::new();
    for fr in pr - r..pr + r + 1 {
        for fq in pq - r..pq + r + 1 {
            let dk = lattice_k(fq, fr) - pk;
            let dm = lattice_m(fr) - pm;
            let dx = dk as f64 * 0.8660254037844386;
            let dy = dm as f64 * 0.5;
            if turn_in_poly(&poly, dx * co1 + dy * si1, -dx * si1 + dy * co1) {
                out.push((dk, dm));
            }
        }
    }
    (out, sx, sy)
}

fn cells_turn(cells: &HexSet, pq: i64, pr: i64, steps: i64) -> Turned {
    let s = steps.rem_euclid(12);
    let r = turn_reach(cells, pq, pr);
    let pk = lattice_k(pq, pr);
    let pm = lattice_m(pr);
    let (base, sx, sy) = if s % 2 == 1 { turn_thirty(cells, pq, pr, r) } else { (turn_offsets(cells, pk, pm), 0.0, 0.0) };
    let mut out = HexSet::chunk(pq - r, pr - r, 2 * r + 1, 2 * r + 1);
    for &(dk, dm) in &base {
        let (k, m) = lattice_turned(dk, dm, s / 2);
        let (q, rr) = lattice_to_cell(pk + k, pm + m);
        out.set(q, rr, true);
    }
    Turned { cells: out, pk, pm, steps: s, sx, sy }
}

fn turned_point(t: &Turned, x: f64, y: f64) -> (f64, f64) {
    let px = t.pk as f64 * 0.8660254037844386;
    let py = t.pm as f64 * 0.5;
    let a = t.steps as f64 * std::f64::consts::PI / 6.0;
    let rest = (t.steps / 2) as f64 * std::f64::consts::PI / 3.0;
    let dx = (x - px) * a.cos() - (y - py) * a.sin() + (t.sx * rest.cos() - t.sy * rest.sin());
    let dy = (x - px) * a.sin() + (y - py) * a.cos() + (t.sx * rest.sin() + t.sy * rest.cos());
    (px + dx, py + dy)
}

// ── hexseat.loft: seat_blend ────────────────────────────────────────

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
    fn get(&self, q: i64, r: i64) -> f64 {
        let i = self.index(q, r);
        if i < 0 { 0.0 } else { self.z[i as usize] }
    }
    fn set(&mut self, q: i64, r: i64, z: f64) {
        let i = self.index(q, r);
        if i >= 0 {
            self.z[i as usize] = z;
        }
    }
}

fn seat_smooth(t: f64) -> f64 {
    let c = if t < 0.0 { 0.0 } else if t > 1.0 { 1.0 } else { t };
    c * c * (3.0 - 2.0 * c)
}

fn seat_blend(cells: &HexSet, terr: &mut Heights, z: f64, blend: f64, step: f64) -> f64 {
    let kx = step * 0.5;
    let my = step * 0.28867513459481287;
    let mut foot: Vec<(f64, f64)> = Vec::new();
    let mut low = 0.0;
    for r in cells.r0..cells.r0 + cells.h {
        for q in cells.q0..cells.q0 + cells.w {
            if cells.get(q, r) {
                let h = terr.get(q, r);
                if foot.is_empty() || h < low {
                    low = h;
                }
                foot.push((lattice_k(q, r) as f64 * kx, lattice_m(r) as f64 * my));
            }
        }
    }
    if foot.is_empty() {
        return z;
    }
    for r in cells.r0..cells.r0 + cells.h {
        for q in cells.q0..cells.q0 + cells.w {
            if cells.get(q, r) {
                terr.set(q, r, z);
            }
        }
    }
    let pad = (blend / (3.0 * my)).ceil() as i64 + 2;
    for r in cells.r0 - pad..cells.r0 + cells.h + pad {
        for q in cells.q0 - pad..cells.q0 + cells.w + pad {
            if cells.get(q, r) {
                continue;
            }
            let x = lattice_k(q, r) as f64 * kx;
            let y = lattice_m(r) as f64 * my;
            let mut near = 1000000.0;
            for &(fx, fy) in &foot {
                let dx = fx - x;
                let dy = fy - y;
                let fd = dx * dx + dy * dy;
                if fd < near {
                    near = fd;
                }
            }
            let d = f64::sqrt(near) - kx;
            if d >= blend {
                continue;
            }
            let t = seat_smooth(d / blend);
            let own = terr.get(q, r);
            terr.set(q, r, z + (own - z) * t);
        }
    }
    low
}

// ── the cells_turn and seat_blend rows ──────────────────────────────

const TURN_HALF: i64 = 40;
const TURN_W: i64 = 81;
const TURN_STEPS: [i64; 7] = [1, 2, 3, 5, 7, 10, 11];
const SEATS: i64 = 5;

fn hall(cq: i64) -> HexSet {
    let mut s = HexSet::chunk(-TURN_HALF, -TURN_HALF, TURN_W, TURN_W);
    for r in -TURN_HALF..TURN_HALF + 1 {
        for q in -TURN_HALF..TURN_HALF + 1 {
            let k = lattice_k(q, r) - lattice_k(cq, 0);
            let m = lattice_m(r);
            if (-20..=20).contains(&k) && m * m <= 588 {
                s.set(q, r, true);
            }
        }
    }
    s
}

fn micro(x: f64) -> i64 {
    (x * 1000000.0) as i64
}

fn turn_op(cells: &HexSet, cq: i64) -> Vec<i64> {
    let mut ints: Vec<i64> = vec![0];
    for (ti, &step) in TURN_STEPS.iter().enumerate() {
        let t = cells_turn(cells, cq, 0, step);
        let (x, y) = turned_point(&t, 3.0, 4.0);
        ints[0] += t.cells.count;
        ints.extend([t.cells.count, micro(t.sx), micro(t.sy), micro(x), micro(y)]);
        if ti == TURN_STEPS.len() - 1 {
            for r in t.cells.r0..t.cells.r0 + t.cells.h {
                ints.push((t.cells.q0..t.cells.q0 + t.cells.w).filter(|&q| t.cells.get(q, r)).count() as i64);
            }
        }
    }
    ints
}

fn bench_turn(n: i64) -> Row {
    let c0 = hall(0);
    let c1 = hall(1);
    let (us, sink) = timed(n, |r| if r & 1 == 0 { turn_op(&c0, 0)[0] } else { turn_op(&c1, 1)[0] });
    let one = turn_op(black_box(&c0), 0);
    Row { name: "cells_turn", iters: n, us, px: TURN_STEPS.len() as i64 * c0.count, hash: fnv(FNV_OFFSET, &one), sink }
}

fn slope(salt: i64) -> Heights {
    let mut h = Heights::new(-TURN_HALF, -TURN_HALF, TURN_W, TURN_W);
    for r in -TURN_HALF..TURN_HALF + 1 {
        for q in -TURN_HALF..TURN_HALF + 1 {
            h.set(q, r, (lattice_k(q, r) + salt) as f64 * 0.05 + lattice_m(r) as f64 * 0.03);
        }
    }
    h
}

fn seat_op(cells: &HexSet, salt: i64) -> Vec<i64> {
    let mut ints: Vec<i64> = vec![0];
    for sj in 0..SEATS {
        let mut h = slope(salt);
        let low = seat_blend(cells, &mut h, 2.0, 3.0, 1.5);
        ints[0] += micro(low);
        ints.push(micro(low));
        if sj == SEATS - 1 {
            for r in -TURN_HALF..TURN_HALF + 1 {
                let mut sum = 0.0;
                for q in -TURN_HALF..TURN_HALF + 1 {
                    sum += h.get(q, r);
                }
                ints.push(micro(sum));
            }
        }
    }
    ints
}

fn bench_seat(n: i64) -> Row {
    let c0 = hall(0);
    let c1 = hall(1);
    let (us, sink) = timed(n, |r| if r & 1 == 0 { seat_op(&c0, 0)[0] } else { seat_op(&c1, 1)[0] });
    let one = seat_op(black_box(&c0), 0);
    Row { name: "seat_blend", iters: n, us, px: SEATS * TURN_W * TURN_W, hash: fnv(FNV_OFFSET, &one), sink }
}

// ── cellopen.loft ───────────────────────────────────────────────────

const OPEN_DOOR: i64 = 1;
const OPEN_WINDOW: i64 = 2;

/// A door's width, a window's, the corner clearance.
struct OpeningSizes {
    door: f64,
    window: f64,
    corner: f64,
}

fn opening_size(kind: i64, sz: &OpeningSizes) -> f64 {
    if kind == OPEN_DOOR { sz.door } else { sz.window }
}

/// A surface's unit direction.
fn surface_dir(w: &WallSurface) -> (f64, f64) {
    let wx = w.dk as f64 * 0.8660254037844386;
    let wy = w.dm as f64 * 0.5;
    let ln = (wx * wx + wy * wy).sqrt();
    (wx / ln, wy / ln)
}

#[allow(dead_code)]
struct CellOpening {
    kind: i64,
    wall: usize,
    edges: Vec<usize>,
    c: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
}

/// An edge's midpoint, in world units.
fn edge_mid(e: &WallEdge) -> (f64, f64) {
    ((e.ak + e.bk) as f64 * 0.5 * 0.8660254037844386, (e.am + e.bm) as f64 * 0.25)
}

fn opening_over(b: &[WallEdge], w: &WallSurface, wi: usize, kind: i64, group: Vec<usize>, sz: &OpeningSizes) -> CellOpening {
    let (mut px, mut py) = (0.0, 0.0);
    for &i in &group {
        let (ex, ey) = edge_mid(&b[i]);
        px += ex;
        py += ey;
    }
    let n = group.len() as f64;
    px /= n;
    py /= n;
    let mx = w.mean_x();
    let my = w.mean_y();
    let (dx, dy) = surface_dir(w);
    let t = (px - mx) * dx + (py - my) * dy;
    let cx = mx + t * dx;
    let cy = my + t * dy;
    let half = opening_size(kind, sz) * 0.5;
    CellOpening { kind, wall: wi, edges: group, c: (cx, cy), a: (cx - half * dx, cy - half * dy), b: (cx + half * dx, cy + half * dy) }
}

fn cells_openings(b: &[WallEdge], rs: &[Vec<usize>], walls: &[WallSurface], marks: &EdgeSet, sz: &OpeningSizes) -> Vec<CellOpening> {
    let mut out = Vec::new();
    for (wi, run) in rs.iter().enumerate() {
        let mut i = 0;
        while i < run.len() {
            let e = b[run[i]];
            let kind = marks.get_mat(e.qa, e.ra, e.qb, e.rb);
            if kind != OPEN_DOOR && kind != OPEN_WINDOW {
                i += 1;
                continue;
            }
            let mut group = Vec::new();
            while i < run.len() {
                let f = b[run[i]];
                if marks.get_mat(f.qa, f.ra, f.qb, f.rb) != kind {
                    break;
                }
                group.push(run[i]);
                i += 1;
            }
            out.push(opening_over(b, &walls[wi], wi, kind, group, sz));
        }
    }
    out
}

fn cells_place_opening(cw: &CellWalls, marks: &mut EdgeSet, kind: i64, x: f64, y: f64, sz: &OpeningSizes) -> bool {
    let n = cw.walls.len();
    let mut best = usize::MAX;
    let (mut best_d, mut best_t, mut best_lo, mut best_hi) = (0.0, 0.0, 0.0, 0.0);
    for i in 0..n {
        let w = &cw.walls[i];
        let (dx, dy) = surface_dir(w);
        let mx = w.mean_x();
        let my = w.mean_y();
        let (ax, ay) = cw.corners[(i + n - 1) % n];
        let (bx, by) = cw.corners[i];
        let t0 = (ax - mx) * dx + (ay - my) * dy;
        let t1 = (bx - mx) * dx + (by - my) * dy;
        let lo = if t0 < t1 { t0 } else { t1 };
        let hi = if t0 > t1 { t0 } else { t1 };
        let t = (x - mx) * dx + (y - my) * dy;
        let across = ((x - mx) * -dy + (y - my) * dx).abs();
        let beyond = if t < lo { lo - t } else if t > hi { t - hi } else { 0.0 };
        let dist = across + beyond;
        if best == usize::MAX || dist < best_d {
            best = i;
            best_d = dist;
            best_t = t;
            best_lo = lo;
            best_hi = hi;
        }
    }
    if best == usize::MAX {
        return false;
    }
    let half = opening_size(kind, sz) * 0.5;
    let clear = half + sz.corner;
    let t = if best_lo + clear > best_hi - clear {
        (best_lo + best_hi) * 0.5
    } else {
        let up = if best_t > best_lo + clear { best_t } else { best_lo + clear };
        if up < best_hi - clear { up } else { best_hi - clear }
    };
    let w = &cw.walls[best];
    let (dx, dy) = surface_dir(w);
    let mx = w.mean_x();
    let my = w.mean_y();
    let mut picks: Vec<usize> = Vec::new();
    let mut nearest = usize::MAX;
    let mut near_d = 0.0;
    for &i in &cw.runs[best] {
        let (ex, ey) = edge_mid(&cw.boundary[i]);
        let off = ((ex - mx) * dx + (ey - my) * dy - t).abs();
        if off <= half + 0.000000001 {
            picks.push(i);
        }
        if nearest == usize::MAX || off < near_d {
            nearest = i;
            near_d = off;
        }
    }
    if picks.is_empty() {
        picks.push(nearest);
    }
    for &i in &picks {
        let e = cw.boundary[i];
        marks.set_mat(e.qa, e.ra, e.qb, e.rb, kind);
    }
    true
}

// ── cellturn.loft: marked_turn ──────────────────────────────────────

fn no_marks(cells: &HexSet) -> EdgeSet {
    EdgeSet::new(cells.q0 - 1, cells.r0 - 1, cells.w + 2, cells.h + 2)
}

// The structure and its marks turned: through 60-degree steps the marked edges with their cells,
// through an odd step each opening's centre carried and placed again on the turned walls.
fn marked_turn(cells: &HexSet, marks: &EdgeSet, pq: i64, pr: i64, steps: i64, hint: i64, sz: &OpeningSizes) -> (HexSet, EdgeSet) {
    let s = steps.rem_euclid(12);
    let t = cells_turn(cells, pq, pr, s);
    let mut out = no_marks(&t.cells);
    let drawn = cells_built(cells, 1, hint);
    let ops = cells_openings(&drawn.boundary, &drawn.runs, &drawn.walls, marks, sz);
    if s % 2 == 0 {
        for o in &ops {
            for &i in &o.edges {
                let e = drawn.boundary[i];
                let (ak, am) = lattice_turned(lattice_k(e.qa, e.ra) - t.pk, lattice_m(e.ra) - t.pm, s / 2);
                let (bk, bm) = lattice_turned(lattice_k(e.qb, e.rb) - t.pk, lattice_m(e.rb) - t.pm, s / 2);
                let (qa, ra) = lattice_to_cell(t.pk + ak, t.pm + am);
                let (qb, rb) = lattice_to_cell(t.pk + bk, t.pm + bm);
                out.set_mat(qa, ra, qb, rb, o.kind);
            }
        }
        return (t.cells, out);
    }
    let next_hint = if hint >= 0 { hint + s } else { -1 };
    let turned = cells_built(&t.cells, 1, next_hint);
    for o in &ops {
        let (x, y) = turned_point(&t, o.c.0, o.c.1);
        cells_place_opening(&turned, &mut out, o.kind, x, y, sz);
    }
    (t.cells, out)
}

fn default_sizes() -> OpeningSizes {
    OpeningSizes { door: 1.0, window: 1.0, corner: 0.8660254037844386 }
}

fn marked_hall(cq: i64) -> (HexSet, EdgeSet) {
    let cells = hall(cq);
    let b = cells_built(&cells, 1, 0);
    let mut marks = no_marks(&cells);
    let sz = default_sizes();
    let n = b.walls.len();
    for i in 0..n {
        let a = b.corners[(i + n - 1) % n];
        let c = b.corners[i];
        for k in 1..4 {
            let f = k as f64 * 0.25;
            cells_place_opening(&b, &mut marks, if k == 2 { OPEN_DOOR } else { OPEN_WINDOW },
                                a.0 + (c.0 - a.0) * f, a.1 + (c.1 - a.1) * f, &sz);
        }
    }
    (cells, marks)
}

fn marked_op(d: &(HexSet, EdgeSet), cq: i64) -> Vec<i64> {
    let mut ints: Vec<i64> = vec![0];
    let sz = default_sizes();
    for &step in &TURN_STEPS {
        let (cells, marks) = marked_turn(&d.0, &d.1, cq, 0, step, 0, &sz);
        ints[0] += marks.count_all();
        ints.extend([cells.count, marks.count_all(), marks.digest()]);
    }
    ints
}

fn bench_marked(n: i64) -> Row {
    let d0 = marked_hall(0);
    let d1 = marked_hall(1);
    let (us, sink) = timed(n, |r| if r & 1 == 0 { marked_op(&d0, 0)[0] } else { marked_op(&d1, 1)[0] });
    let one = marked_op(black_box(&d0), 0);
    Row { name: "marked_turn", iters: n, us, px: TURN_STEPS.len() as i64 * d0.0.count, hash: fnv(FNV_OFFSET, &one), sink }
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
    let rows = [bench_cut(n), bench_union(n), bench_turn(n), bench_seat(n), bench_marked(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
