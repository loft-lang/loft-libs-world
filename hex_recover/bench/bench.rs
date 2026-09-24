// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_recover-reference — the pure-Rust twin of the `hex_recover` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports: `forms_upto`, `forms_sides`, `field_digest`, `field_norm_text` from
// src/formcensus.loft and `fit_chunk`, `index_build` from src/formfit.loft, and what they
// stand on from hex_form (`head_step`, the form's polygon, law J, `form_admissible`,
// `form_canon`, `form_write`, `poly_holds`, `form_fill`) and hex_field (`HexSet`,
// `lattice_k`/`lattice_m`).  The same loops, the same linear text dedup (`Vec<String>`), the
// same insertion sort over each of the 12 orientations; the index is a
// `HashMap<String, Form>`.  `black_box` guards each op's INPUT (the repetition number, and
// for `forms_upto` its level) and the sink — never anything inside a kernel.
use std::collections::HashMap;
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const UPTO: i64 = 2;
const PROPOSALS: i64 = 11616;
const CAND_SIDES: i64 = 4;
const CAND_LEN: i64 = 3;
const TRI: i64 = 23;
const DIGESTS: i64 = 4;

const HEAD_N: i64 = 12;
const TURN_FULL: i64 = 12;

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

fn fnv_text(h0: i64, t: &str) -> i64 {
    let mut h = h0;
    for &b in t.as_bytes() {
        h = ((h ^ b as i64) * FNV_PRIME) & 0xFFFF_FFFF;
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

// ── hex_field: the cell layer and the lattice ───────────────────────

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

// ── hex_form: the form, law J, the canonical text, the fill ─────────

#[derive(Clone)]
struct Form {
    h0: i64,
    lens: Vec<i64>,
    turns: Vec<i64>,
}

fn head_norm(h: i64) -> i64 {
    ((h % HEAD_N) + HEAD_N) % HEAD_N
}

fn head_step(h: i64) -> (i64, i64) {
    const STEPS: [(i64, i64); 12] = [
        (2, 0), (3, 3), (1, 3), (0, 6), (-1, 3), (-3, 3),
        (-2, 0), (-3, -3), (-1, -3), (0, -6), (1, -3), (3, -3),
    ];
    STEPS[head_norm(h) as usize]
}

fn form_new(h0: i64, lens: Vec<i64>, turns: Vec<i64>) -> Form {
    Form { h0: head_norm(h0), lens, turns }
}

fn form_close(f: &Form) -> (i64, i64) {
    let (mut ck, mut cm, mut h) = (0, 0, f.h0);
    for (l, t) in f.lens.iter().zip(&f.turns) {
        let st = head_step(h);
        ck += l * st.0;
        cm += l * st.1;
        h += t;
    }
    (ck, cm)
}

fn form_closes(f: &Form) -> bool {
    if f.lens.len() < 3 {
        return false;
    }
    form_close(f) == (0, 0) && f.turns.iter().sum::<i64>() == TURN_FULL
}

fn form_poly(f: &Form) -> (Vec<i64>, Vec<i64>) {
    let mut vk = Vec::with_capacity(f.lens.len());
    let mut vm = Vec::with_capacity(f.lens.len());
    let (mut pk, mut pm, mut h) = (0, 0, f.h0);
    for (l, t) in f.lens.iter().zip(&f.turns) {
        vk.push(pk);
        vm.push(pm);
        let st = head_step(h);
        pk += l * st.0;
        pm += l * st.1;
        h += t;
    }
    (vk, vm)
}

fn form_is_simple(f: &Form) -> bool {
    let (vk, vm) = form_poly(f);
    let n = vk.len();
    for a in 0..n {
        for b in a + 1..n {
            if vk[a] == vk[b] && vm[a] == vm[b] {
                return false;
            }
        }
    }
    true
}

fn form_is_convex(f: &Form) -> bool {
    f.turns.iter().all(|&t| t > 0)
}

fn form_admissible(f: &Form) -> bool {
    form_closes(f) && form_is_simple(f) && form_is_convex(f)
}

fn shift(v: &[i64], s: usize) -> Vec<i64> {
    let n = v.len();
    (0..n).map(|i| v[(i + s) % n]).collect()
}

fn shift_h0(f: &Form, s: usize) -> i64 {
    head_norm(f.h0 + f.turns[..s].iter().sum::<i64>())
}

fn form_canon(f: &Form) -> Form {
    let n = f.lens.len();
    if n == 0 {
        return form_new(f.h0, f.lens.clone(), f.turns.clone());
    }
    let mut bt = shift(&f.turns, 0);
    let mut bl = shift(&f.lens, 0);
    let mut bh = shift_h0(f, 0);
    for s in 1..n {
        let ct = shift(&f.turns, s);
        let cl = shift(&f.lens, s);
        let ch = shift_h0(f, s);
        let ord = ct.cmp(&bt).then(cl.cmp(&bl)).then(if ch < bh { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater });
        if ord.is_lt() {
            bt = ct;
            bl = cl;
            bh = ch;
        }
    }
    form_new(bh, bl, bt)
}

fn form_write(f: &Form, name: &str) -> String {
    let mut s = String::new();
    write!(s, "stencil {} h0 {}", name, f.h0).unwrap();
    for (i, (l, t)) in f.lens.iter().zip(&f.turns).enumerate() {
        write!(s, "\nside {} len {} turn {}", i, l, t).unwrap();
    }
    s
}

fn poly_holds(vk: &[i64], vm: &[i64], pk: i64, pm: i64) -> bool {
    let n = vk.len();
    if n < 3 {
        return false;
    }
    for e in 0..n {
        let (ak, am) = (vk[e], vm[e]);
        let j = (e + 1) % n;
        let (bk, bm) = (vk[j], vm[j]);
        let crs = (bk - ak) * (pm - am) - (bm - am) * (pk - ak);
        if crs == 0 && pk >= ak.min(bk) && pk <= ak.max(bk) && pm >= am.min(bm) && pm <= am.max(bm) {
            return true;
        }
    }
    let mut inside = false;
    for c in 0..n {
        let (ck, cm) = (vk[c], vm[c]);
        let j = (c + 1) % n;
        let (dk, dm) = (vk[j], vm[j]);
        if (cm > pm) != (dm > pm) {
            let cr2 = (dk - ck) * (pm - cm) - (dm - cm) * (pk - ck);
            if dm > cm {
                if cr2 > 0 {
                    inside = !inside;
                }
            } else if cr2 < 0 {
                inside = !inside;
            }
        }
    }
    inside
}

fn form_fill(f: &Form, cq: i64, cr: i64, cells: &mut HexSet) -> i64 {
    if !form_closes(f) {
        return -1;
    }
    let (vk, vm) = form_poly(f);
    let bk = lattice_k(cq, cr);
    let bm = lattice_m(cr);
    let mut n = 0;
    for fr in cells.r0..cells.r0 + cells.h {
        for fq in cells.q0..cells.q0 + cells.w {
            if poly_holds(&vk, &vm, lattice_k(fq, fr) - bk, lattice_m(fr) - bm) {
                cells.set(fq, fr, true);
                n += 1;
            }
        }
    }
    n
}

// ── formcensus.loft ─────────────────────────────────────────────────

fn ckey(k: i64, m: i64) -> i64 {
    (k + 400) * 1600 + (m + 400)
}

fn orient_k(k: i64, m: i64, o: i64) -> i64 {
    let (mut ok, mut om) = if o >= 6 { (-k, m) } else { (k, m) };
    for _ in 0..o % 6 {
        let nk = (ok - om) / 2;
        let nm = (3 * ok + om) / 2;
        ok = nk;
        om = nm;
    }
    ok
}

fn orient_m(k: i64, m: i64, o: i64) -> i64 {
    let (mut ok, mut om) = if o >= 6 { (-k, m) } else { (k, m) };
    for _ in 0..o % 6 {
        let nk = (ok - om) / 2;
        let nm = (3 * ok + om) / 2;
        ok = nk;
        om = nm;
    }
    om
}

/// Insertion sort, appending each key and sinking it into place.
fn sort_keys(v: &[i64]) -> Vec<i64> {
    let mut out: Vec<i64> = Vec::with_capacity(v.len());
    for &x in v {
        out.push(x);
        let mut pos = out.len() - 1;
        while pos > 0 && out[pos - 1] > out[pos] {
            out.swap(pos - 1, pos);
            pos -= 1;
        }
    }
    out
}

fn oriented_keys(cells: &HexSet, o: i64) -> Vec<i64> {
    let mut ks = Vec::new();
    let mut ms = Vec::new();
    for gr in cells.r0..cells.r0 + cells.h {
        for gq in cells.q0..cells.q0 + cells.w {
            if cells.get(gq, gr) {
                let (rk, rm) = (lattice_k(gq, gr), lattice_m(gr));
                ks.push(orient_k(rk, rm, o));
                ms.push(orient_m(rk, rm, o));
            }
        }
    }
    if ks.is_empty() {
        return ks;
    }
    let mink = *ks.iter().min().unwrap();
    let minm = *ms.iter().min().unwrap();
    let raw: Vec<i64> = ks.iter().zip(&ms).map(|(&k, &m)| ckey(k - mink, m - minm)).collect();
    sort_keys(&raw)
}

fn field_digest(cells: &HexSet) -> Vec<i64> {
    let mut best = oriented_keys(cells, 0);
    for o in 1..12 {
        let cand = oriented_keys(cells, o);
        if cand < best {
            best = cand;
        }
    }
    best
}

fn digest_text(d: &[i64]) -> String {
    let mut s = String::new();
    for (i, v) in d.iter().enumerate() {
        if i > 0 {
            s.push('.');
        }
        write!(s, "{}", v).unwrap();
    }
    s
}

fn field_norm_text(cells: &HexSet) -> String {
    digest_text(&oriented_keys(cells, 0))
}

fn forms_upto(maxlen: i64) -> Vec<Form> {
    let mut out = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    for la in 1..=maxlen {
        for lb in 1..=maxlen {
            for lc in 1..=maxlen {
                for eh in 0..12 {
                    for ea in 1..12 {
                        for eb in 1..12 {
                            let ec = 12 - ea - eb;
                            if !(1..=11).contains(&ec) {
                                continue;
                            }
                            let ef = form_new(eh, vec![la, lb, lc], vec![ea, eb, ec]);
                            if form_admissible(&ef) {
                                let cf = form_canon(&ef);
                                let ct = form_write(&cf, "x");
                                if !texts.contains(&ct) {
                                    texts.push(ct);
                                    out.push(cf);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

fn forms_sides(sides: i64, maxlen: i64) -> Vec<Form> {
    let mut out = Vec::new();
    let mut texts: Vec<String> = Vec::new();
    let ncomb = 11i64.pow((sides - 1) as u32);
    let nlen = maxlen.pow(sides as u32);
    for ci in 0..ncomb {
        let mut turns = Vec::new();
        let mut rest = ci;
        let mut tot = 0;
        for _ in 0..sides - 1 {
            let dg = rest % 11 + 1;
            rest /= 11;
            turns.push(dg);
            tot += dg;
        }
        let last = 12 - tot;
        if !(1..=11).contains(&last) {
            continue;
        }
        turns.push(last);
        for li in 0..nlen {
            let mut lens = Vec::new();
            let mut lrest = li;
            for _ in 0..sides {
                lens.push(lrest % maxlen + 1);
                lrest /= maxlen;
            }
            for eh in 0..12 {
                let ef = form_new(eh, lens.clone(), turns.clone());
                if form_admissible(&ef) {
                    let cf = form_canon(&ef);
                    let ct = form_write(&cf, "x");
                    if !texts.contains(&ct) {
                        texts.push(ct);
                        out.push(cf);
                    }
                }
            }
        }
    }
    out
}

// ── formfit.loft ────────────────────────────────────────────────────

fn fdiv(a: i64, b: i64) -> i64 {
    let q = a / b;
    if a < 0 && q * b != a { q - 1 } else { q }
}

fn fit_chunk(f: &Form) -> HexSet {
    let (vk, vm) = form_poly(f);
    if vk.is_empty() {
        return HexSet::chunk(0, 0, 1, 1);
    }
    let (mink, maxk) = (*vk.iter().min().unwrap(), *vk.iter().max().unwrap());
    let (minm, maxm) = (*vm.iter().min().unwrap(), *vm.iter().max().unwrap());
    let r0 = fdiv(minm, 3) - 1;
    let r1 = fdiv(maxm, 3) + 1;
    let q0 = fdiv(mink - 1, 2) - 1;
    let q1 = fdiv(maxk, 2) + 1;
    HexSet::chunk(q0, r0, q1 - q0 + 1, r1 - r0 + 1)
}

struct RecoveryIndex {
    map: HashMap<String, Form>,
    count: i64,
    collisions: i64,
    fills: i64,
}

fn index_build(cand: &[Form]) -> RecoveryIndex {
    let mut idx = RecoveryIndex { map: HashMap::new(), count: 0, collisions: 0, fills: 0 };
    for cf in cand {
        let mut cc = fit_chunk(cf);
        form_fill(cf, 0, 0, &mut cc);
        idx.fills += 1;
        let dg = field_norm_text(&cc);
        if idx.map.contains_key(&dg) {
            idx.collisions += 1;
        } else {
            idx.map.insert(dg, cf.clone());
            idx.count += 1;
        }
    }
    idx
}

// ── The rows ────────────────────────────────────────────────────────

fn form_ints(f: &Form) -> Vec<i64> {
    let mut v = vec![f.h0, f.lens.len() as i64];
    v.extend_from_slice(&f.lens);
    v.extend_from_slice(&f.turns);
    v
}

fn bench_upto(n: i64) -> Row {
    let (us, sink) = timed(n, |_| forms_upto(black_box(UPTO)).len() as i64);
    let fs = forms_upto(black_box(UPTO));
    let hash = fs.iter().fold(fnv(FNV_OFFSET, &[fs.len() as i64]), |h, f| fnv(h, &form_ints(f)));
    Row { name: "forms_upto", iters: n, us, px: PROPOSALS, hash, sink }
}

fn index_hash(idx: &RecoveryIndex) -> i64 {
    let sum: i64 = idx.map.iter().map(|(k, f)| fnv(fnv_text(FNV_OFFSET, k), &form_ints(f))).sum();
    fnv(FNV_OFFSET, &[idx.count, idx.collisions, idx.fills, sum & 0xFFFF_FFFF])
}

fn bench_index(n: i64) -> Row {
    let fwd = forms_sides(CAND_SIDES, CAND_LEN);
    let rev: Vec<Form> = fwd.iter().rev().cloned().collect();
    let (us, sink) = timed(n, |r| index_build(if r & 1 == 0 { &fwd } else { &rev }).count);
    Row { name: "index_build", iters: n, us, px: fwd.len() as i64,
          hash: index_hash(&index_build(black_box(&fwd))), sink }
}

fn triangle(shift: i64) -> HexSet {
    let f = form_new(0, vec![TRI, TRI, TRI], vec![4, 4, 4]);
    let c = fit_chunk(&f);
    let mut s = HexSet::chunk(c.q0 + shift, c.r0, c.w, c.h);
    form_fill(&f, shift, 0, &mut s);
    s
}

fn digest_op(s: &HexSet) -> i64 {
    (0..DIGESTS).map(|_| field_digest(s)[0]).sum()
}

fn bench_digest(n: i64) -> Row {
    let s0 = triangle(0);
    let s1 = triangle(1);
    let (us, sink) = timed(n, |r| digest_op(if r & 1 == 0 { &s0 } else { &s1 }));
    Row { name: "field_digest", iters: n, us, px: DIGESTS * s0.count,
          hash: fnv(FNV_OFFSET, &field_digest(black_box(&s0))), sink }
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
    let rows = [bench_upto(n), bench_index(n), bench_digest(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
