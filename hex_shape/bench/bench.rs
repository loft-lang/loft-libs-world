// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_shape-reference — the pure-Rust twin of the `hex_shape` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workload with
// the SAME arithmetic, in the same order, and prints the same row, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports, from src/hexwall.loft: `wall_chain_walk` and its tables (`chain_marks`,
// `chain_verts`, `vert_degree`, `deg_left`), the corner addresses `hex_corner_tri_a/b`,
// `tri_x`/`tri_y`, and — to build the input — `wall_from_run` with its direction tables and
// `wall_write` / `wall_separates`.  From hex_grid `hex_neighbor` and `hex_edge_corners`,
// from hex_form `head_step`, from hex_field the lattice, `nb_q`/`nb_r` and the `EdgeSet`
// material slot behind `edge_mat` / `edge_set_mat`.  The endpoint table is two `Vec<i64>`
// scanned exactly as the loft routine scans it.  `black_box` guards each op's INPUT and
// the sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const FIELD: i64 = 48;
const WALL_DIR: i64 = 2;
const PERIODS: i64 = 60;
const WALKS: i64 = 100;

const D24: i64 = 24;
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

// ── hex_field.loft: lattice, neighbours, the edge material slot ─────

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
}

impl EdgeSet {
    fn new(q0: i64, r0: i64, w: i64, h: i64) -> EdgeSet {
        let (gw, gh) = (w + 2, h + 2);
        EdgeSet { q0, r0, gw, gh, mat: vec![0; (gw * gh * 3) as usize] }
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

    fn edge_mat(&self, qa: i64, ra: i64, qb: i64, rb: i64) -> i64 {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 {
            return 0;
        }
        self.mat[i as usize] as i64
    }

    fn set_mat(&mut self, qa: i64, ra: i64, qb: i64, rb: i64, mat: i64) {
        let i = self.index(qa, ra, qb, rb);
        if i < 0 || !(0..=255).contains(&mat) {
            return;
        }
        self.mat[i as usize] = mat as u8;
    }
}

// ── hex_grid.loft, hex_form.loft ────────────────────────────────────

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

fn hex_edge_corners(dir: i64) -> (i64, i64) {
    match dir {
        0 => (4, 5),
        1 => (3, 4),
        2 => (2, 3),
        3 => (1, 2),
        4 => (0, 1),
        _ => (5, 0),
    }
}

fn head_step(h: i64) -> (i64, i64) {
    const STEPS: [(i64, i64); 12] = [(2, 0), (3, 3), (1, 3), (0, 6), (-1, 3), (-3, 3),
                                     (-2, 0), (-3, -3), (-1, -3), (0, -6), (1, -3), (3, -3)];
    STEPS[h.rem_euclid(12) as usize]
}

// ── hexwall.loft ────────────────────────────────────────────────────

fn between_k(i: i64) -> i64 {
    let (j, s) = if i >= 6 { (i - 6, -1) } else { (i, 1) };
    s * [7, 5, 2, -2, -5, -7][j as usize]
}

fn between_m(i: i64) -> i64 {
    let (j, s) = if i >= 6 { (i - 6, -1) } else { (i, 1) };
    s * [3, 9, 12, 12, 9, 3][j as usize]
}

fn wall_step(d24: i64) -> (i64, i64) {
    let n = ((d24 % D24) + D24) % D24;
    if n % 2 == 0 {
        return head_step(n / 2);
    }
    (between_k((n - 1) / 2), between_m((n - 1) / 2))
}

fn igcd(x: i64, y: i64) -> i64 {
    let mut a = x.abs();
    let mut b = y.abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The primitive triangle-lattice vector of direction `d24` — `tri_a` and `tri_b` together.
fn tri_ab(d24: i64) -> (i64, i64) {
    let (k, m) = wall_step(d24);
    let ta = 3 * k;
    let tb = 3 * (m - k) / 2;
    let g = igcd(ta, tb);
    if g == 0 {
        return (ta, tb);
    }
    (ta / g, tb / g)
}

fn tri_x(a: i64) -> f64 {
    (a as f64) * 0.28867513459481287
}

fn tri_y(a: i64, b: i64) -> f64 {
    (a as f64) / 6.0 + (b as f64) / 3.0
}

fn corner_da(ci: i64) -> i64 {
    [0, -3, -3, 0, 3, 3][ci.rem_euclid(6) as usize]
}

fn corner_db(ci: i64) -> i64 {
    [3, 3, 0, -3, -3, 0][ci.rem_euclid(6) as usize]
}

fn hex_corner_tri_a(q: i64, r: i64, ci: i64) -> i64 {
    3 * lattice_k(q, r) + corner_da(ci)
}

fn hex_corner_tri_b(q: i64, r: i64, ci: i64) -> i64 {
    3 * (lattice_m(r) - lattice_k(q, r)) / 2 + corner_db(ci)
}

fn cell_x_w(q: i64, r: i64) -> f64 {
    (lattice_k(q, r) as f64) * 0.8660254037844386
}

fn cell_y_w(r: i64) -> f64 {
    (lattice_m(r) as f64) * 0.5
}

struct Wall {
    ox: f64,
    oy: f64,
    dx: f64,
    dy: f64,
    half: f64,
    mat: i64,
}

fn wall_from_run(d24: i64, a0: i64, b0: i64, p: i64, mat: i64) -> Wall {
    let (ta, tb) = tri_ab(d24);
    let ea = a0 + 3 * p * ta;
    let eb = b0 + 3 * p * tb;
    let (ax, ay) = (tri_x(a0), tri_y(a0, b0));
    let (bx, by) = (tri_x(ea), tri_y(ea, eb));
    let vx = bx - ax;
    let vy = by - ay;
    let ln = (vx * vx + vy * vy).sqrt();
    Wall { ox: (ax + bx) * 0.5, oy: (ay + by) * 0.5, dx: vx / ln, dy: vy / ln, half: ln * 0.5, mat }
}

fn wall_offset_signed(w: &Wall, px: f64, py: f64) -> f64 {
    (px - w.ox) * (0.0 - w.dy) + (py - w.oy) * w.dx
}

fn wall_separates(w: &Wall, cx: f64, cy: f64, nx: f64, ny: f64) -> bool {
    let oc = wall_offset_signed(w, cx, cy);
    let on = wall_offset_signed(w, nx, ny);
    let pc = oc > 0.0 - WALL_EPS;
    let pn = on > 0.0 - WALL_EPS;
    if pc == pn {
        return false;
    }
    let t = oc / (oc - on);
    let ax = cx + t * (nx - cx);
    let ay = cy + t * (ny - cy);
    let al = (ax - w.ox) * w.dx + (ay - w.oy) * w.dy;
    al >= (0.0 - w.half) - WALL_EPS && al <= w.half + WALL_EPS
}

fn wall_write(w: &Wall, e: &mut EdgeSet, q0: i64, r0: i64, wq: i64, hq: i64) -> i64 {
    let mut n = 0;
    for r in r0..r0 + hq {
        for q in q0..q0 + wq {
            for d in 0..6 {
                let (nq, nr) = hex_neighbor(q, r, d);
                if wall_separates(w, cell_x_w(q, r), cell_y_w(r), cell_x_w(nq, nr), cell_y_w(nr))
                    && e.edge_mat(q, r, nq, nr) == 0
                {
                    e.set_mat(q, r, nq, nr, w.mat);
                    n += 1;
                }
            }
        }
    }
    n
}

#[derive(Default)]
struct WallChain {
    ok: bool,
    n: i64,
    branch: i64,
    closed: bool,
    va: Vec<i64>,
    vb: Vec<i64>,
    x: Vec<f64>,
    y: Vec<f64>,
    q: Vec<i64>,
    r: Vec<i64>,
    d: Vec<i64>,
}

fn vert_degree(va: &[i64], vb: &[i64], ka: i64, kb: i64) -> i64 {
    let mut deg = 0;
    for j in 0..va.len() {
        if va[j] == ka && vb[j] == kb {
            deg += 1;
        }
    }
    deg
}

fn deg_left(va: &[i64], vb: &[i64], used: &[bool], ka: i64, kb: i64) -> i64 {
    let mut deg = 0;
    for j in 0..va.len() {
        if used[j / 2] {
            continue;
        }
        if va[j] == ka && vb[j] == kb {
            deg += 1;
        }
    }
    deg
}

fn wall_chain_walk(e: &EdgeSet, q0: i64, r0: i64, wq: i64, hq: i64, nth: i64) -> WallChain {
    // chain_marks
    let (mut mq, mut mr, mut md) = (Vec::new(), Vec::new(), Vec::new());
    for r in r0..r0 + hq {
        for q in q0..q0 + wq {
            for d in 0..3 {
                let (nq, nr) = hex_neighbor(q, r, d);
                if e.edge_mat(q, r, nq, nr) == 0 {
                    continue;
                }
                mq.push(q);
                mr.push(r);
                md.push(d);
            }
        }
    }
    // chain_verts
    let (mut va, mut vb) = (Vec::new(), Vec::new());
    for i in 0..mq.len() {
        let (e1, e2) = hex_edge_corners(md[i]);
        va.push(hex_corner_tri_a(mq[i], mr[i], e1));
        vb.push(hex_corner_tri_b(mq[i], mr[i], e1));
        va.push(hex_corner_tri_a(mq[i], mr[i], e2));
        vb.push(hex_corner_tri_b(mq[i], mr[i], e2));
    }
    let mut used = vec![false; mq.len()];
    let mut out = WallChain::default();
    let mut found = 0i64;
    for _pass in 0..mq.len() + 1 {
        let (mut sa, mut sb) = (0, 0);
        let mut have = false;
        for j in 0..va.len() {
            if used[j / 2] {
                continue;
            }
            if deg_left(&va, &vb, &used, va[j], vb[j]) != 1 {
                continue;
            }
            sa = va[j];
            sb = vb[j];
            have = true;
            break;
        }
        if !have {
            for j in 0..va.len() {
                if used[j / 2] {
                    continue;
                }
                sa = va[j];
                sb = vb[j];
                have = true;
                break;
            }
        }
        if !have {
            break;
        }
        let (mut ca, mut cb) = (sa, sb);
        let mut cv_a = vec![sa];
        let mut cv_b = vec![sb];
        let (mut cm_q, mut cm_r, mut cm_d) = (Vec::new(), Vec::new(), Vec::new());
        let mut cbr = 0;
        for _step in 0..mq.len() {
            if vert_degree(&va, &vb, ca, cb) > 2 {
                cbr += 1;
            }
            let mut pick: i64 = -1;
            let (mut na, mut nb) = (0, 0);
            for j in 0..va.len() {
                let ei = j / 2;
                if used[ei] {
                    continue;
                }
                if va[j] != ca || vb[j] != cb {
                    continue;
                }
                pick = ei as i64;
                let o = if j % 2 == 0 { j + 1 } else { j - 1 };
                na = va[o];
                nb = vb[o];
                break;
            }
            if pick < 0 {
                break;
            }
            let p = pick as usize;
            used[p] = true;
            cm_q.push(mq[p]);
            cm_r.push(mr[p]);
            cm_d.push(md[p]);
            cv_a.push(na);
            cv_b.push(nb);
            ca = na;
            cb = nb;
        }
        if found == nth {
            out.ok = true;
            out.branch = cbr;
            out.closed = ca == sa && cb == sb && !cm_q.is_empty();
            out.x = cv_a.iter().map(|&a| tri_x(a)).collect();
            out.y = cv_a.iter().zip(&cv_b).map(|(&a, &b)| tri_y(a, b)).collect();
            out.va = cv_a;
            out.vb = cv_b;
            out.q = cm_q;
            out.r = cm_r;
            out.d = cm_d;
        }
        found += 1;
    }
    out.n = found;
    out
}

// ── The row ─────────────────────────────────────────────────────────

fn walled(cq: i64) -> EdgeSet {
    let mut e = EdgeSet::new(0, 0, FIELD, FIELD);
    let w = wall_from_run(WALL_DIR, hex_corner_tri_a(cq, 4, 0), hex_corner_tri_b(cq, 4, 0), PERIODS, 1);
    wall_write(&w, &mut e, 0, 0, FIELD, FIELD);
    e
}

fn walk_op(e: &EdgeSet) -> Vec<i64> {
    let mut c = wall_chain_walk(e, 0, 0, FIELD, FIELD, 0);
    for _ in 1..WALKS {
        c = wall_chain_walk(e, 0, 0, FIELD, FIELD, 0);
    }
    let mut v = vec![c.ok as i64, c.n, c.branch, c.closed as i64, c.va.len() as i64, c.q.len() as i64];
    v.extend(&c.va);
    v.extend(&c.vb);
    v.extend(c.x.iter().map(|&x| (x * 1000000.0) as i64));
    v.extend(c.y.iter().map(|&y| (y * 1000000.0) as i64));
    v.extend(&c.q);
    v.extend(&c.r);
    v.extend(&c.d);
    v
}

fn bench_walk(n: i64) -> Row {
    let e0 = walled(4);
    let e1 = walled(5);
    let (us, sink) = timed(n, |r| walk_op(if (r & 1) == 0 { &e0 } else { &e1 })[4]);
    let one = walk_op(black_box(&e0));
    Row { name: "wall_chain_walk", iters: n, us, px: WALKS * one[5], hash: fnv(FNV_OFFSET, &one), sink }
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
    let rows = [bench_walk(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
