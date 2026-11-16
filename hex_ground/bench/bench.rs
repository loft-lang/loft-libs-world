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
// `Heights`, and its `lattice_*`, `corner_*` and `lattice_to_cell`.  `black_box` guards each
// op's INPUT (the repetition number) and the sink — never anything inside a kernel.
use std::f64::consts::PI;
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const HALF: i64 = 48;
const WIN: i64 = 97;
const TERRACES: i64 = 3;
const READS: i64 = 4;

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
    let rows = [bench_terraces(n), bench_read(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
