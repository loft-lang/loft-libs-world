// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_ground-reference — the pure-Rust twin of the `hex_ground` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports, from src/hex_ground.loft: `ground_terraces` with `terrace_height` and
// `steps_level`; `ground_hill` (setup only); `ground_height_at` with `ground_cell_near`,
// `ground_centre` and `ground_corner_height`; on a plain `Vec<f64>` of heights for hex_field's
// `Heights`, and its `lattice_*`, `corner_*` and `lattice_to_cell`.  And `ground_dig` with
// `sides_distance`, `ground_cliffs` with its clipping, and `ground_height_on`: the first two on
// hex_draw's `cells_outlines`, ported whole with all it stands on.  `black_box` guards each
// op's INPUT (the repetition number) and the sink — never anything inside a kernel.
// The outline solver below is hex_draw's whole, pasted: it carries a few of `cells_built`'s
// helpers that the outlines do not call.
#![allow(dead_code)]

use std::f64::consts::PI;
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const HALF: i64 = 48;
const WIN: i64 = 97;
const TERRACES: i64 = 3;
const READS: i64 = 4;
const DIGS: i64 = 3;
const CLIFFS: i64 = 3;

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

// ── hex_field ───────────────────────────────────────────────────────

fn lattice_k(q: i64, r: i64) -> i64 {
    2 * q + (r & 1)
}

fn lattice_m(r: i64) -> i64 {
    3 * r
}

const CORNER_K: [i64; 6] = [0, -1, -1, 0, 1, 1];
const CORNER_M: [i64; 6] = [2, 1, -1, -2, -1, 1];

fn lattice_to_cell(k: i64, m: i64) -> (i64, i64) {
    let lr = m / 3;
    ((k - (lr & 1)) / 2, lr)
}

// ── hex_field's cell set and hex_draw's outlines (src/cellwalls.loft) ─

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

#[derive(Clone, Copy)]
struct WallSurface {
    dk: i64,
    dm: i64,
    num_k: i64,
    num_m: i64,
    den: i64,
    n: i64,
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

// ── hex_ground.loft ─────────────────────────────────────────────────

/// The ground: a window of heights and its step.  (The surface kinds are not read here.)
struct Ground {
    q0: i64,
    r0: i64,
    w: i64,
    h: i64,
    step: f64,
    z: Vec<f64>,
}

impl Ground {
    fn new(q0: i64, r0: i64, w: i64, h: i64, step: f64) -> Ground {
        Ground { q0, r0, w, h, step, z: vec![0.0; (w * h) as usize] }
    }
    fn holds(&self, q: i64, r: i64) -> bool {
        q >= self.q0 && q < self.q0 + self.w && r >= self.r0 && r < self.r0 + self.h
    }
    fn get(&self, q: i64, r: i64) -> f64 {
        if self.holds(q, r) { self.z[((r - self.r0) * self.w + (q - self.q0)) as usize] } else { 0.0 }
    }
    fn set(&mut self, q: i64, r: i64, z: f64) {
        if self.holds(q, r) {
            self.z[((r - self.r0) * self.w + (q - self.q0)) as usize] = z;
        }
    }
}

fn ground_centre(step: f64, q: i64, r: i64) -> (f64, f64) {
    (lattice_k(q, r) as f64 * (step * 0.5), lattice_m(r) as f64 * (step * 0.28867513459481287))
}

fn ground_cell_near(step: f64, x: f64, y: f64) -> (i64, i64) {
    let r = (y / (3.0 * (step * 0.28867513459481287))).round() as i64;
    let q = (((x / (step * 0.5)) - (r & 1) as f64) * 0.5).round() as i64;
    let (mut best_q, mut best_r, mut best) = (q, r, -1.0);
    for dr in -1..2 {
        for dq in -1..2 {
            let (cx, cy) = ground_centre(step, q + dq, r + dr);
            let d = (cx - x) * (cx - x) + (cy - y) * (cy - y);
            if best < 0.0 || d < best {
                best = d;
                best_q = q + dq;
                best_r = r + dr;
            }
        }
    }
    (best_q, best_r)
}

struct GroundSteps {
    x0: f64,
    y0: f64,
    size: f64,
    nx: i64,
    ny: i64,
    level: Vec<i64>,
}

fn steps_level(s: &GroundSteps, i: i64, j: i64) -> i64 {
    let ci = i.clamp(0, s.nx - 1);
    let cj = j.clamp(0, s.ny - 1);
    s.level[(cj * s.nx + ci) as usize]
}

fn ground_smooth(t: f64) -> f64 {
    let c = if t < 0.0 { 0.0 } else if t > 1.0 { 1.0 } else { t };
    c * c * (3.0 - 2.0 * c)
}

/// The measurements: a storey, a hillside's width, a bank's.
struct Measures {
    storey: f64,
    hillside: f64,
    bank: f64,
}

fn terrace_height(s: &GroundSteps, x: f64, y: f64, m: &Measures, lo: i64, hi: i64) -> f64 {
    let i = ((x - s.x0) / s.size).floor() as i64;
    let j = ((y - s.y0) / s.size).floor() as i64;
    let mut h = lo as f64 * m.storey;
    for step in lo + 1..hi + 1 {
        let wide = if step <= 0 { m.bank } else { m.hillside };
        let reach = ((wide * 0.5) / s.size).ceil() as i64 + 1;
        let inside = steps_level(s, i, j) >= step;
        let mut near = wide;
        for dj in -reach..reach + 1 {
            for di in -reach..reach + 1 {
                if (steps_level(s, i + di, j + dj) >= step) == inside {
                    continue;
                }
                let ax = s.x0 + (i + di) as f64 * s.size;
                let ay = s.y0 + (j + dj) as f64 * s.size;
                let dx = if x < ax { ax - x } else if x > ax + s.size { x - ax - s.size } else { 0.0 };
                let dy = if y < ay { ay - y } else if y > ay + s.size { y - ay - s.size } else { 0.0 };
                let d = (dx * dx + dy * dy).sqrt();
                if d < near {
                    near = d;
                }
            }
        }
        let signed = if inside { near } else { -near };
        h += m.storey * ground_smooth((signed + wide * 0.5) / wide);
    }
    h
}

fn ground_terraces(g: &mut Ground, s: &GroundSteps, m: &Measures, lo: i64, hi: i64) {
    for r in g.r0..g.r0 + g.h {
        for q in g.q0..g.q0 + g.w {
            let (x, y) = ground_centre(g.step, q, r);
            g.set(q, r, terrace_height(s, x, y, m, lo, hi));
        }
    }
}

fn ground_hill(g: &mut Ground, x: f64, y: f64, radius: f64, height: f64) {
    for r in g.r0..g.r0 + g.h {
        for q in g.q0..g.q0 + g.w {
            let (cx, cy) = ground_centre(g.step, q, r);
            let d = ((cx - x) * (cx - x) + (cy - y) * (cy - y)).sqrt();
            if d >= radius {
                continue;
            }
            let bump = height * 0.5 * (1.0 + (PI * d / radius).cos());
            g.set(q, r, g.get(q, r) + bump);
        }
    }
}

// The mean of the cells round corner (k, m) that the window holds, summed in row order.
fn ground_corner_height(g: &Ground, k: i64, m: i64) -> f64 {
    let mut cells = [(0i64, 0i64); 3];
    let mut n = 0;
    for i in 0..6 {
        let ck = k - CORNER_K[i];
        let cm = m - CORNER_M[i];
        if cm.rem_euclid(3) != 0 {
            continue;
        }
        let (q, r) = lattice_to_cell(ck, cm);
        if lattice_k(q, r) != ck || lattice_m(r) != cm || !g.holds(q, r) {
            continue;
        }
        let mut at = n;
        while at > 0 && (r < cells[at - 1].1 || (r == cells[at - 1].1 && q < cells[at - 1].0)) {
            cells[at] = cells[at - 1];
            at -= 1;
        }
        cells[at] = (q, r);
        n += 1;
    }
    if n == 0 {
        return 0.0;
    }
    let mut sum = g.get(cells[0].0, cells[0].1);
    for c in &cells[1..n] {
        sum += g.get(c.0, c.1);
    }
    sum / n as f64
}

fn ground_height_at(g: &Ground, x: f64, y: f64) -> f64 {
    let (q, r) = ground_cell_near(g.step, x, y);
    let (cx, cy) = ground_centre(g.step, q, r);
    let k = lattice_k(q, r);
    let m = lattice_m(r);
    let a = (y - cy).atan2(x - cx) * 180.0 / PI;
    let j0 = (((a - 90.0) / 60.0).floor() as i64).rem_euclid(6) as usize;
    let j1 = (j0 + 1) % 6;
    let kx = g.step * 0.5;
    let my = g.step * 0.28867513459481287;
    let ax = CORNER_K[j0] as f64 * kx;
    let ay = CORNER_M[j0] as f64 * my;
    let bx = CORNER_K[j1] as f64 * kx;
    let by = CORNER_M[j1] as f64 * my;
    let px = x - cx;
    let py = y - cy;
    let det = ax * by - ay * bx;
    let wa = (px * by - py * bx) / det;
    let wb = (ax * py - ay * px) / det;
    let wc = 1.0 - wa - wb;
    wc * g.get(q, r) + wa * ground_corner_height(g, k + CORNER_K[j0], m + CORNER_M[j0])
        + wb * ground_corner_height(g, k + CORNER_K[j1], m + CORNER_M[j1])
}

fn ground_unit(step: f64) -> f64 {
    2.0 * (step * 0.28867513459481287)
}

/// A structure's straight sides in the ground's unit: every loop's walls, corner to corner.
fn straight_sides(cells: &HexSet, unit: f64) -> Vec<(f64, f64, f64, f64)> {
    let mut sides = Vec::new();
    for o in cells_outlines(cells, 0) {
        let n = o.corners.len();
        for i in 0..n {
            let (x1, y1) = o.corners[(i + n - 1) % n];
            let (x2, y2) = o.corners[i];
            sides.push((x1 * unit, y1 * unit, x2 * unit, y2 * unit));
        }
    }
    sides
}

fn sides_distance(sides: &[(f64, f64, f64, f64)], x: f64, y: f64) -> f64 {
    let mut inside = false;
    let mut near = 1000000.0;
    for &(x1, y1, x2, y2) in sides {
        if (y1 > y) != (y2 > y) {
            let cx = x1 + (y - y1) * (x2 - x1) / (y2 - y1);
            if x < cx {
                inside = !inside;
            }
        }
        let dx = x2 - x1;
        let dy = y2 - y1;
        let l2 = dx * dx + dy * dy;
        let mut t = if l2 > 0.0 { ((x - x1) * dx + (y - y1) * dy) / l2 } else { 0.0 };
        if t < 0.0 {
            t = 0.0;
        }
        if t > 1.0 {
            t = 1.0;
        }
        let ex = x - x1 - t * dx;
        let ey = y - y1 - t * dy;
        let e = ex * ex + ey * ey;
        if e < near {
            near = e;
        }
    }
    let far = f64::sqrt(near);
    if inside { far } else { -far }
}

fn ground_dig(g: &mut Ground, cells: &HexSet, depth: f64, bank: f64) {
    let sides = straight_sides(cells, ground_unit(g.step));
    if sides.is_empty() {
        return;
    }
    for r in g.r0..g.r0 + g.h {
        for q in g.q0..g.q0 + g.w {
            let (x, y) = ground_centre(g.step, q, r);
            let d = sides_distance(&sides, x, y);
            let z = g.get(q, r);
            g.set(q, r, z - depth * ground_smooth((d + bank * 0.5) / bank));
        }
    }
}

/// Levels over a window, as hex_field's `Labels`: -1 beyond it.
struct Labels {
    q0: i64,
    r0: i64,
    w: i64,
    h: i64,
    v: Vec<i64>,
}

impl Labels {
    fn get(&self, q: i64, r: i64) -> i64 {
        let dq = q - self.q0;
        let dr = r - self.r0;
        if dq < 0 || dr < 0 || dq >= self.w || dr >= self.h { -1 } else { self.v[(dr * self.w + dq) as usize] }
    }
}

/// An area where fronts are cliffs: x0, y0, x1, y1.
type GroundBox = [f64; 4];

/// The part of a segment inside a box, or nothing.
fn box_clip(ax: f64, ay: f64, bx: f64, by: f64, b: &GroundBox) -> Option<[f64; 4]> {
    let (mut t0, mut t1) = (0.0, 1.0);
    let dx = bx - ax;
    let dy = by - ay;
    for (p, qv) in [(-dx, ax - b[0]), (dx, b[2] - ax), (-dy, ay - b[1]), (dy, b[3] - ay)] {
        if p == 0.0 {
            if qv < 0.0 {
                return None;
            }
            continue;
        }
        let t = qv / p;
        if p < 0.0 {
            if t > t0 {
                t0 = t;
            }
        } else if t < t1 {
            t1 = t;
        }
    }
    if t0 >= t1 {
        return None;
    }
    Some([ax + dx * t0, ay + dy * t0, ax + dx * t1, ay + dy * t1])
}

/// (level, ax, ay, bx, by) for each stretch of cliff.
fn ground_cliffs(levels: &Labels, lo: i64, hi: i64, areas: &[GroundBox], step: f64) -> Vec<(i64, [f64; 4])> {
    let unit = ground_unit(step);
    let mut out = Vec::new();
    if areas.is_empty() {
        return out;
    }
    for level in lo + 1..hi + 1 {
        let mut cells = HexSet::chunk(levels.q0, levels.r0, levels.w, levels.h);
        for r in levels.r0..levels.r0 + levels.h {
            for q in levels.q0..levels.q0 + levels.w {
                if levels.get(q, r) >= level {
                    cells.set(q, r, true);
                }
            }
        }
        if cells.count == 0 {
            continue;
        }
        for (x1, y1, x2, y2) in straight_sides(&cells, unit) {
            for a in areas {
                if let Some(c) = box_clip(x1, y1, x2, y2, a) {
                    if (c[2] - c[0]) * (c[2] - c[0]) + (c[3] - c[1]) * (c[3] - c[1]) > 0.0001 {
                        out.push((level, c));
                    }
                }
            }
        }
    }
    out
}

fn ground_level(g: &Ground, levels: &Labels, q: i64, r: i64) -> i64 {
    if g.holds(q, r) { levels.get(q, r) } else { 0 }
}

fn corner_on_side(g: &Ground, levels: &Labels, k: i64, m: i64, j: usize, level: i64, high: bool, own: f64) -> f64 {
    let mut sum = 0.0;
    let mut n = 0;
    for t in 0..3 {
        let i = (j + 2 * t) % 6;
        let (q, r) = lattice_to_cell(k - CORNER_K[i], m - CORNER_M[i]);
        if (ground_level(g, levels, q, r) >= level) == high {
            sum += g.get(q, r);
            n += 1;
        }
    }
    if n == 0 { own } else { sum / n as f64 }
}

fn ground_height_on(g: &Ground, levels: &Labels, x: f64, y: f64, level: i64, high: bool, storey: f64) -> f64 {
    let (q, r) = ground_cell_near(g.step, x, y);
    let (cx, cy) = ground_centre(g.step, q, r);
    let k = lattice_k(q, r);
    let m = lattice_m(r);
    let a = (y - cy).atan2(x - cx) * 180.0 / PI;
    let j0 = (((a - 90.0) / 60.0).floor() as i64).rem_euclid(6) as usize;
    let j1 = (j0 + 1) % 6;
    let own = (if high { level } else { level - 1 }) as f64 * storey;
    let hc = if (ground_level(g, levels, q, r) >= level) == high { g.get(q, r) } else { own };
    let ha = corner_on_side(g, levels, k + CORNER_K[j0], m + CORNER_M[j0], j0, level, high, own);
    let hb = corner_on_side(g, levels, k + CORNER_K[j1], m + CORNER_M[j1], j1, level, high, own);
    let kx = g.step * 0.5;
    let my = g.step * 0.28867513459481287;
    let ax = CORNER_K[j0] as f64 * kx;
    let ay = CORNER_M[j0] as f64 * my;
    let bx = CORNER_K[j1] as f64 * kx;
    let by = CORNER_M[j1] as f64 * my;
    let px = x - cx;
    let py = y - cy;
    let det = ax * by - ay * bx;
    let wa = (px * by - py * bx) / det;
    let wb = (ax * py - ay * px) / det;
    (1.0 - wa - wb) * hc + wa * ha + wb * hb
}

// ── the rows ────────────────────────────────────────────────────────

fn micro(x: f64) -> i64 {
    (x * 1000000.0) as i64
}

fn step_map(front: i64) -> GroundSteps {
    let mut level = Vec::with_capacity(64 * 64);
    for j in 0..64 {
        for i in 0..64 {
            level.push((i >= front) as i64 + (i + j >= 70) as i64);
        }
    }
    GroundSteps { x0: -48.0, y0: -48.0, size: 1.5, nx: 64, ny: 64, level }
}

fn terrace_op(s: &GroundSteps) -> Vec<i64> {
    let m = Measures { storey: 2.6, hillside: 6.0, bank: 2.5 };
    let mut ints: Vec<i64> = vec![0];
    for t in 0..TERRACES {
        let mut g = Ground::new(-HALF, -HALF, WIN, WIN, 1.5);
        ground_terraces(&mut g, s, &m, 0, 2);
        ints[0] += micro(g.get(t, -t));
        if t == TERRACES - 1 {
            for r in -HALF..HALF + 1 {
                let mut sum = 0.0;
                for q in -HALF..HALF + 1 {
                    sum += g.get(q, r);
                }
                ints.push(micro(sum));
            }
        }
    }
    ints
}

fn bench_terraces(n: i64) -> Row {
    let s0 = step_map(30);
    let s1 = step_map(31);
    let (us, sink) = timed(n, |r| terrace_op(if r & 1 == 0 { &s0 } else { &s1 })[0]);
    let one = terrace_op(black_box(&s0));
    Row { name: "ground_terraces", iters: n, us, px: TERRACES * WIN * WIN, hash: fnv(FNV_OFFSET, &one), sink }
}

fn hilly() -> Ground {
    let mut g = Ground::new(-HALF, -HALF, WIN, WIN, 1.5);
    ground_hill(&mut g, 6.0, -9.0, 40.0, 9.0);
    ground_hill(&mut g, -20.0, 15.0, 18.0, -4.0);
    g
}

fn read_op(g: &Ground, shift: i64) -> Vec<i64> {
    let mut ints: Vec<i64> = vec![0];
    let off = shift as f64 * 0.25;
    for rj in 0..READS {
        for j in 0..120 {
            let mut sum = 0.0;
            for i in 0..120 {
                sum += ground_height_at(g, -22.0 + off + i as f64 * 0.37, -21.0 + j as f64 * 0.37);
            }
            if rj == READS - 1 {
                ints.push(micro(sum));
            }
            if j == 60 {
                ints[0] += micro(sum);
            }
        }
    }
    ints
}

fn bench_read(n: i64) -> Row {
    let g = hilly();
    let (us, sink) = timed(n, |r| read_op(&g, r & 1)[0]);
    let one = read_op(black_box(&g), 0);
    Row { name: "ground_height_at", iters: n, us, px: READS * 120 * 120, hash: fnv(FNV_OFFSET, &one), sink }
}

fn bed(shift: i64) -> HexSet {
    let mut s = HexSet::chunk(-HALF, -HALF, WIN, WIN);
    for q in -36..37 {
        let off = ((q + 45) / 9) % 3;
        for r in 0..5 {
            s.set(q + shift, off - 2 + r, true);
        }
    }
    s
}

fn row_sums(g: &Ground, ints: &mut Vec<i64>) {
    for r in -HALF..HALF + 1 {
        let mut sum = 0.0;
        for q in -HALF..HALF + 1 {
            sum += g.get(q, r);
        }
        ints.push(micro(sum));
    }
}

fn dig_op(cells: &HexSet) -> Vec<i64> {
    let mut ints: Vec<i64> = vec![0];
    for dj in 0..DIGS {
        let mut g = Ground::new(-HALF, -HALF, WIN, WIN, 1.5);
        ground_dig(&mut g, cells, 2.0, 2.5);
        ints[0] += micro(g.get(dj, 2));
        if dj == DIGS - 1 {
            row_sums(&g, &mut ints);
        }
    }
    ints
}

fn bench_dig(n: i64) -> Row {
    let c0 = bed(0);
    let c1 = bed(1);
    let (us, sink) = timed(n, |r| dig_op(if r & 1 == 0 { &c0 } else { &c1 })[0]);
    let one = dig_op(black_box(&c0));
    Row { name: "ground_dig", iters: n, us, px: DIGS * WIN * WIN, hash: fnv(FNV_OFFSET, &one), sink }
}

fn terrace_levels(front: i64) -> Labels {
    let mut v = Vec::with_capacity((WIN * WIN) as usize);
    for r in -HALF..HALF + 1 {
        for q in -HALF..HALF + 1 {
            let k = lattice_k(q, r);
            let m = lattice_m(r);
            v.push((k >= front) as i64 + ((-30..=30).contains(&k) && m * m <= 300) as i64);
        }
    }
    Labels { q0: -HALF, r0: -HALF, w: WIN, h: WIN, v }
}

fn cliff_op(levels: &Labels) -> Vec<i64> {
    let areas: [GroundBox; 2] = [[-30.0, -30.0, 10.0, 40.0], [0.0, -50.0, 60.0, 0.0]];
    let mut ints: Vec<i64> = vec![0];
    for cj in 0..CLIFFS {
        let cs = ground_cliffs(levels, 0, 2, &areas, 1.5);
        ints[0] += cs.len() as i64;
        if cj == CLIFFS - 1 {
            for (level, c) in &cs {
                ints.extend([*level, micro(c[0]), micro(c[1]), micro(c[2]), micro(c[3])]);
            }
        }
    }
    ints
}

fn bench_cliffs(n: i64) -> Row {
    let l0 = terrace_levels(-10);
    let l1 = terrace_levels(-8);
    let (us, sink) = timed(n, |r| cliff_op(if r & 1 == 0 { &l0 } else { &l1 })[0]);
    let one = cliff_op(black_box(&l0));
    Row { name: "ground_cliffs", iters: n, us, px: CLIFFS * 2 * WIN * WIN, hash: fnv(FNV_OFFSET, &one), sink }
}

fn on_op(g: &Ground, levels: &Labels, shift: i64) -> Vec<i64> {
    let mut ints: Vec<i64> = vec![0];
    let off = shift as f64 * 0.25;
    for rj in 0..READS {
        for j in 0..120i64 {
            let mut sum = 0.0;
            for i in 0..120i64 {
                sum += ground_height_on(g, levels, -22.0 + off + i as f64 * 0.37, -21.0 + j as f64 * 0.37, 1, ((i + j) & 1) == 0, 2.6);
            }
            if rj == READS - 1 {
                ints.push(micro(sum));
            }
            if j == 60 {
                ints[0] += micro(sum);
            }
        }
    }
    ints
}

fn bench_on(n: i64) -> Row {
    let g = hilly();
    let levels = terrace_levels(-10);
    let (us, sink) = timed(n, |r| on_op(&g, &levels, r & 1)[0]);
    let one = on_op(black_box(&g), &levels, 0);
    Row { name: "ground_height_on", iters: n, us, px: READS * 120 * 120, hash: fnv(FNV_OFFSET, &one), sink }
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
    let rows = [bench_terraces(n), bench_read(n), bench_dig(n), bench_cliffs(n), bench_on(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
