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
// `Vec<i64>` plus a `Vec<bool>`, and its canonical-start rotation.  Then `stencil_rotate`
// over a struct-of-Vec stencil (`Heights`, `Labels`, `Layers`, both edge slots, the exact
// lattice turn `cell_rot`), and `doc_write` / `doc_read` for the HXF document: the reader
// takes the whole file with ONE `fs::read` and decodes the slice, where the loft reader
// crosses into a file-read builtin per value.  `black_box` guards each op's INPUT and the
// sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const FIELD: i64 = 64;
const COUNTS: i64 = 100;
const TRACES: i64 = 50;
const STENCIL_R: i64 = 16;
const ROTATIONS: i64 = 10;
const DOC_W: i64 = 128;
const DOCS: i64 = 3;

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
    count: i64,
}

impl HexSet {
    fn chunk(q0: i64, r0: i64, w: i64, h: i64) -> HexSet {
        HexSet { q0, r0, w, h, cells: vec![false; (w * h) as usize], count: 0 }
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
    surf: Vec<i32>,
    count: i64,
    refused: i64,
}

impl EdgeSet {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> EdgeSet {
        let (gw, gh) = (w + 2, h + 2);
        let n = (gw * gh * 3) as usize;
        EdgeSet { q0, r0, w, h, gw, gh, mat: vec![0; n], surf: vec![0; n], count: 0, refused: 0 }
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
        if !(0..=EDGE_MAT_MAX).contains(&mat) {
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

    fn set_both(&mut self, qa: i64, ra: i64, qb: i64, rb: i64, surf: i64, mat: i64) {
        self.set_surf(qa, ra, qb, rb, surf);
        self.set_mat(qa, ra, qb, rb, mat);
    }

    fn digest(&self) -> i64 {
        let mut h: i64 = 1469598103;
        for i in 0..self.mat.len() {
            h = (h * 31 + self.mat[i] as i64) % 1000000007;
            h = (h * 31 + self.surf[i] as i64) % 1000000007;
        }
        h
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

/// The per-cell window every payload layer shares: origin, extent, row-major slot.
#[derive(Clone, Copy)]
struct Window {
    q0: i64,
    r0: i64,
    w: i64,
    h: i64,
}

impl Window {
    fn slot(&self, q: i64, r: i64) -> i64 {
        let dq = q - self.q0;
        let dr = r - self.r0;
        if dq < 0 || dr < 0 || dq >= self.w || dr >= self.h {
            return -1;
        }
        dr * self.w + dq
    }
}

struct Heights {
    win: Window,
    z: Vec<f64>,
}

impl Heights {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> Heights {
        Heights { win: Window { q0, r0, w, h }, z: vec![0.0; (w * h) as usize] }
    }
    fn set(&mut self, q: i64, r: i64, z: f64) {
        let i = self.win.slot(q, r);
        if i >= 0 {
            self.z[i as usize] = z;
        }
    }
    fn get(&self, q: i64, r: i64) -> f64 {
        let i = self.win.slot(q, r);
        if i < 0 { 0.0 } else { self.z[i as usize] }
    }
}

struct Labels {
    win: Window,
    v: Vec<i64>,
}

impl Labels {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> Labels {
        Labels { win: Window { q0, r0, w, h }, v: vec![-1; (w * h) as usize] }
    }
    fn set(&mut self, q: i64, r: i64, v: i64) {
        let i = self.win.slot(q, r);
        if i >= 0 {
            self.v[i as usize] = v;
        }
    }
    fn get(&self, q: i64, r: i64) -> i64 {
        let i = self.win.slot(q, r);
        if i < 0 { -1 } else { self.v[i as usize] }
    }
}

struct Layers {
    win: Window,
    names: Vec<String>,
    vals: Vec<i64>,
}

impl Layers {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> Layers {
        Layers { win: Window { q0, r0, w, h }, names: Vec::new(), vals: Vec::new() }
    }
    fn add(&mut self, name: &str) -> i64 {
        if let Some(i) = self.names.iter().rposition(|n| n == name) {
            return i as i64;
        }
        self.names.push(name.to_string());
        self.vals.resize(self.vals.len() + (self.win.w * self.win.h) as usize, 0);
        self.names.len() as i64 - 1
    }
    fn slot(&self, li: i64, q: i64, r: i64) -> i64 {
        let i = self.win.slot(q, r);
        if li < 0 || i < 0 {
            return -1;
        }
        li * self.win.w * self.win.h + i
    }
    fn get_at(&self, li: i64, q: i64, r: i64) -> i64 {
        let i = self.slot(li, q, r);
        if i < 0 { 0 } else { self.vals[i as usize] }
    }
    fn set_at(&mut self, li: i64, q: i64, r: i64, v: i64) {
        let i = self.slot(li, q, r);
        if i >= 0 {
            self.vals[i as usize] = v;
        }
    }
}

// ── Stencils: the exact 60-degree turn ──────────────────────────────

fn lattice_to_cell(k: i64, m: i64) -> (i64, i64) {
    let lr = m / 3;
    ((k - (lr & 1)) / 2, lr)
}

fn cell_rot(q: i64, r: i64, n: i64) -> (i64, i64) {
    let (mut rk, mut rm) = (lattice_k(q, r), lattice_m(r));
    let mut steps = n % 6;
    if steps < 0 {
        steps += 6;
    }
    for _ in 0..steps {
        let (nk, nm) = ((rk - rm) / 2, (3 * rk + rm) / 2);
        rk = nk;
        rm = nm;
    }
    lattice_to_cell(rk, rm)
}

struct Stencil {
    cells: HexSet,
    heights: Heights,
    labels: Labels,
    edges: EdgeSet,
    layers: Layers,
    has_h: bool,
    has_l: bool,
    has_e: bool,
}

fn stencil_rotate(st: &Stencil, n: i64) -> Stencil {
    let s = &st.cells;
    let (q0, r0, w, h) = (s.q0, s.r0, s.w, s.h);
    let mut seen = false;
    let (mut mnq, mut mxq, mut mnr, mut mxr) = (0, 0, 0, 0);
    for iq in 0..w {
        for ir in 0..h {
            if s.get(q0 + iq, r0 + ir) {
                let (tq, tr) = cell_rot(q0 + iq, r0 + ir, n);
                if !seen {
                    mnq = tq;
                    mxq = tq;
                    mnr = tr;
                    mxr = tr;
                    seen = true;
                }
                mnq = mnq.min(tq);
                mxq = mxq.max(tq);
                mnr = mnr.min(tr);
                mxr = mxr.max(tr);
            }
        }
    }
    let mut turns = n;
    if !seen {
        mnq = q0;
        mnr = r0;
        mxq = q0 + w - 1;
        mxr = r0 + h - 1;
        turns = 0;
    }
    let nw = mxq - mnq + 1;
    let nh = mxr - mnr + 1;
    let mut out = Stencil {
        cells: HexSet::chunk(mnq, mnr, nw, nh),
        heights: Heights::new(mnq, mnr, nw, nh),
        labels: Labels::new(mnq, mnr, nw, nh),
        edges: EdgeSet::new(mnq, mnr, nw, nh),
        layers: Layers::new(mnq, mnr, nw, nh),
        has_h: st.has_h,
        has_l: st.has_l,
        has_e: st.has_e,
    };
    for name in &st.layers.names {
        out.layers.add(name);
    }
    let nlayers = st.layers.names.len() as i64;
    for iq in 0..w {
        for ir in 0..h {
            if s.get(q0 + iq, r0 + ir) {
                let (tq, tr) = cell_rot(q0 + iq, r0 + ir, turns);
                out.cells.set(tq, tr, true);
                if st.has_h {
                    out.heights.set(tq, tr, st.heights.get(q0 + iq, r0 + ir));
                }
                if st.has_l {
                    out.labels.set(tq, tr, st.labels.get(q0 + iq, r0 + ir));
                }
                for li in 0..nlayers {
                    out.layers.set_at(li, tq, tr, st.layers.get_at(li, q0 + iq, r0 + ir));
                }
            }
        }
    }
    if st.has_e {
        for iq in -1..w + 1 {
            for ir in -1..h + 1 {
                let (cq, cr) = (q0 + iq, r0 + ir);
                for d in [0, 2, 3] {
                    let nq = nb_q(cq, cr, d);
                    let nr = nb_r(cr, d);
                    let mv = st.edges.mat_at(cq, cr, nq, nr);
                    let sv = st.edges.surf_at(cq, cr, nq, nr);
                    if mv != 0 || sv != 0 {
                        let (aq, ar) = cell_rot(cq, cr, turns);
                        let (bq, br) = cell_rot(nq, nr, turns);
                        out.edges.set_both(aq, ar, bq, br, sv, mv);
                    }
                }
            }
        }
    }
    out
}

// ── The HXF document ────────────────────────────────────────────────

const HXF_MAGIC: i32 = 0x31465848;
const HXF_SCHEMA: i32 = 1;
const HXF_HEADER: i64 = 32;
const TAG_OCCU: i32 = 0x5543434F;
const TAG_HGHT: i32 = 0x54484748;
const TAG_LABL: i32 = 0x4C42414C;
const HXF_OK: i64 = 0;
const HXF_BAD_MAGIC: i64 = 1;
const HXF_BAD_SCHEMA: i64 = 2;
const HXF_BAD_RESERVED: i64 = 3;
const HXF_BAD_SECTION: i64 = 4;
const HXF_TRUNCATED: i64 = 5;
const HXF_MISSING_OCCU: i64 = 6;
const HXF_BAD_EXTENT: i64 = 7;

/// `doc_write` with heights and labels: the header, then OCCU / HGHT / LABL, each a q-major
/// walk of the window.
fn doc_write(path: &str, s: &HexSet, hz: &Heights, lb: &Labels) {
    let (w, h) = (s.w, s.h);
    let n = w * h;
    let mut b: Vec<u8> = Vec::new();
    for v in [HXF_MAGIC, HXF_SCHEMA, s.q0 as i32, s.r0 as i32, w as i32, h as i32, 3, 0] {
        b.extend(v.to_le_bytes());
    }
    b.extend(TAG_OCCU.to_le_bytes());
    b.extend((n as i32).to_le_bytes());
    for q in 0..w {
        for r in 0..h {
            b.push(s.get(s.q0 + q, s.r0 + r) as u8);
        }
    }
    b.extend(TAG_HGHT.to_le_bytes());
    b.extend(((n * 8) as i32).to_le_bytes());
    for q in 0..w {
        for r in 0..h {
            b.extend(hz.get(s.q0 + q, s.r0 + r).to_le_bytes());
        }
    }
    b.extend(TAG_LABL.to_le_bytes());
    b.extend(((n * 4) as i32).to_le_bytes());
    for q in 0..w {
        for r in 0..h {
            b.extend((lb.get(s.q0 + q, s.r0 + r) as i32).to_le_bytes());
        }
    }
    std::fs::write(path, b).expect("write the document");
}

struct HexDoc {
    code: i64,
    cells: HexSet,
    heights: Heights,
    labels: Labels,
    edges: EdgeSet,
    layers: Layers,
    has_h: bool,
    has_l: bool,
    has_e: bool,
    skipped: i64,
}

/// A little-endian cursor over the file's bytes.
struct Cursor<'a> {
    b: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take<const N: usize>(&mut self) -> [u8; N] {
        let v: [u8; N] = self.b[self.at..self.at + N].try_into().unwrap();
        self.at += N;
        v
    }
    fn i32(&mut self) -> i64 {
        i32::from_le_bytes(self.take()) as i64
    }
    fn u8(&mut self) -> u8 {
        self.take::<1>()[0]
    }
    fn f64(&mut self) -> f64 {
        f64::from_le_bytes(self.take())
    }
}

fn doc_read(path: &str) -> HexDoc {
    let mut d = HexDoc {
        code: HXF_OK,
        cells: HexSet::chunk(0, 0, 1, 1),
        heights: Heights::new(0, 0, 1, 1),
        labels: Labels::new(0, 0, 1, 1),
        edges: EdgeSet::new(0, 0, 1, 1),
        layers: Layers::new(0, 0, 1, 1),
        has_h: false,
        has_l: false,
        has_e: false,
        skipped: 0,
    };
    let bytes = std::fs::read(path).unwrap_or_default();
    let total = bytes.len() as i64;
    let mut f = Cursor { b: &bytes, at: 0 };
    if total < HXF_HEADER {
        d.code = HXF_TRUNCATED;
        return d;
    }
    if f.i32() != HXF_MAGIC as i64 {
        d.code = HXF_BAD_MAGIC;
        return d;
    }
    if f.i32() != HXF_SCHEMA as i64 {
        d.code = HXF_BAD_SCHEMA;
        return d;
    }
    let (q0, r0, w, h) = (f.i32(), f.i32(), f.i32(), f.i32());
    let nsec = f.i32();
    if f.i32() != 0 {
        d.code = HXF_BAD_RESERVED;
        return d;
    }
    if w < 0 || h < 0 || w * h > total - HXF_HEADER {
        d.code = HXF_BAD_EXTENT;
        return d;
    }
    let n = w * h;
    d.cells = HexSet::chunk(q0, r0, w, h);
    d.heights = Heights::new(q0, r0, w, h);
    d.labels = Labels::new(q0, r0, w, h);
    d.edges = EdgeSet::new(q0, r0, w, h);
    d.layers = Layers::new(q0, r0, w, h);
    let mut seen_occu = false;
    for _ in 0..nsec {
        if f.at as i64 + 8 > total {
            d.code = HXF_TRUNCATED;
            return d;
        }
        let tag = f.i32() as i32;
        let blen = f.i32();
        if f.at as i64 + blen > total {
            d.code = HXF_TRUNCATED;
            return d;
        }
        match tag {
            TAG_OCCU => {
                if blen != n {
                    d.code = HXF_BAD_SECTION;
                    return d;
                }
                for q in 0..w {
                    for r in 0..h {
                        let v = f.u8();
                        d.cells.set(q0 + q, r0 + r, v != 0);
                    }
                }
                seen_occu = true;
            }
            TAG_HGHT => {
                if blen != n * 8 {
                    d.code = HXF_BAD_SECTION;
                    return d;
                }
                for q in 0..w {
                    for r in 0..h {
                        let hv = f.f64();
                        d.heights.set(q0 + q, r0 + r, hv);
                    }
                }
                d.has_h = true;
            }
            TAG_LABL => {
                if blen != n * 4 {
                    d.code = HXF_BAD_SECTION;
                    return d;
                }
                for q in 0..w {
                    for r in 0..h {
                        let lv = f.i32();
                        d.labels.set(q0 + q, r0 + r, lv);
                    }
                }
                d.has_l = true;
            }
            _ => {
                f.at += blen as usize;
                d.skipped += 1;
            }
        }
    }
    if !seen_occu {
        d.code = HXF_MISSING_OCCU;
    }
    d
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

fn micro(x: f64) -> i64 {
    (x * 1000000.0) as i64
}

fn stencil(salt: i64) -> Stencil {
    let q0 = -STENCIL_R;
    let w = 2 * STENCIL_R + 1;
    let mut s = HexSet::centred(STENCIL_R);
    hexdisk_into(&mut s, 0, 0, STENCIL_R);
    let mut hz = Heights::new(q0, q0, w, w);
    let mut lb = Labels::new(q0, q0, w, w);
    let mut eg = EdgeSet::new(q0, q0, w, w);
    for r in q0..q0 + w {
        for q in q0..q0 + w {
            if s.get(q, r) {
                let a = q + STENCIL_R;
                let b = r + STENCIL_R;
                hz.set(q, r, ((a * 7 + b * 13 + salt) % 50) as f64 * 0.25);
                lb.set(q, r, (a * 3 + b * 5 + salt) % 9);
                if (a + b) % 3 == 0 {
                    for d in [0, 2, 3] {
                        eg.set_both(q, r, nb_q(q, r, d), nb_r(r, d), 1 + (a + d) % 7, 1 + (b + d + salt) % 4);
                    }
                }
            }
        }
    }
    let mut ly = Layers::new(s.q0, s.r0, s.w, s.h);
    let li = ly.add("item");
    for r in q0..q0 + w {
        for q in q0..q0 + w {
            if s.get(q, r) {
                ly.set_at(li, q, r, (q + 2 * r + 100 + salt) % 17);
            }
        }
    }
    Stencil { cells: s, heights: hz, labels: lb, edges: eg, layers: ly, has_h: true, has_l: true, has_e: true }
}

fn stencil_digest(st: &Stencil) -> Vec<i64> {
    let c = &st.cells;
    let mut hs = 0.0;
    let (mut ls, mut ys) = (0i64, 0i64);
    for r in c.r0..c.r0 + c.h {
        for q in c.q0..c.q0 + c.w {
            hs += st.heights.get(q, r);
            ls += st.labels.get(q, r);
            ys += st.layers.get_at(0, q, r);
        }
    }
    vec![c.q0, c.r0, c.w, c.h, c.count, micro(hs), ls, ys, st.edges.digest(), st.edges.count, st.edges.refused]
}

fn rotate_op(st: &Stencil) -> Vec<i64> {
    let mut cells = 0;
    for _ in 0..ROTATIONS {
        for n in 1..7 {
            cells += stencil_rotate(st, n).cells.count;
        }
    }
    let mut v = stencil_digest(&stencil_rotate(st, 1));
    v.push(cells);
    v
}

fn bench_rotate(n: i64) -> Row {
    let s0 = stencil(0);
    let s1 = stencil(1);
    let (us, sink) = timed(n, |r| rotate_op(if (r & 1) == 0 { &s0 } else { &s1 })[4]);
    Row { name: "stencil_rotate", iters: n, us, px: ROTATIONS * 6 * s0.cells.count,
          hash: fnv(FNV_OFFSET, &rotate_op(black_box(&s0))), sink }
}

fn write_doc(path: &str, salt: i64) {
    let mut s = HexSet::chunk(0, 0, DOC_W, DOC_W);
    hexdisk_into(&mut s, 64, 64, 60);
    let mut hz = Heights::new(0, 0, DOC_W, DOC_W);
    let mut lb = Labels::new(0, 0, DOC_W, DOC_W);
    for r in 0..DOC_W {
        for q in 0..DOC_W {
            hz.set(q, r, ((q * 5 + r * 11 + salt) % 64) as f64 * 0.125);
            lb.set(q, r, (q + r * 3) % 13);
        }
    }
    doc_write(path, &s, &hz, &lb);
}

fn doc_op(path: &str) -> [i64; 9] {
    let mut cells = 0;
    let mut d = doc_read(path);
    for _ in 1..DOCS {
        d = doc_read(path);
        cells += d.cells.count;
    }
    let mut hs = 0.0;
    let mut ls = 0i64;
    for r in 0..DOC_W {
        for q in 0..DOC_W {
            hs += d.heights.get(q, r);
            ls += d.labels.get(q, r);
        }
    }
    [d.code, d.cells.count, micro(hs), ls, d.has_h as i64, d.has_l as i64, d.has_e as i64,
     d.skipped, cells]
}

fn bench_doc(n: i64) -> Row {
    std::fs::create_dir_all("bench/.build").expect("create bench/.build");
    let (p0, p1) = ("bench/.build/hexdoc0_rs.hxf", "bench/.build/hexdoc1_rs.hxf");
    write_doc(p0, 0);
    write_doc(p1, 1);
    let (us, sink) = timed(n, |r| doc_op(if (r & 1) == 0 { p0 } else { p1 })[1]);
    Row { name: "doc_read", iters: n, us, px: DOCS * DOC_W * DOC_W * 3,
          hash: fnv(FNV_OFFSET, &doc_op(black_box(p0))), sink }
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
    let rows = [bench_count(n), bench_trace(n), bench_rotate(n), bench_doc(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
