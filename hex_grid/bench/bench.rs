// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_grid-reference — the pure-Rust twin of the `hex_grid` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// Each routine is a port of its loft original (src/hex_grid.loft): integers are i64 like
// loft's, and every branch the library takes is taken here.  `black_box` guards each op's
// INPUT (the repetition number) and the sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const STEPS: i64 = 10000000;
const POINTS: i64 = 2000000;

const SQRT3: f64 = 1.7320508075688772;
const HEX_SIZE: f64 = 1.0;

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

// ── hex_grid.loft ───────────────────────────────────────────────────

fn hex_neighbor(q: i64, r: i64, dir: i64) -> (i64, i64) {
    if (r & 1) == 0 {
        return match dir {
            0 => (q + 1, r),
            1 => (q, r - 1),
            2 => (q - 1, r - 1),
            3 => (q - 1, r),
            4 => (q - 1, r + 1),
            _ => (q, r + 1),
        };
    }
    match dir {
        0 => (q + 1, r),
        1 => (q + 1, r - 1),
        2 => (q, r - 1),
        3 => (q - 1, r),
        4 => (q, r + 1),
        _ => (q + 1, r + 1),
    }
}

fn hex_round(qf: f64, rf: f64) -> (i64, i64) {
    let cx = qf;
    let cz = rf;
    let cy = -cx - cz;
    let mut rxf = cx.round();
    let mut ryf = cy.round();
    let mut rzf = cz.round();
    let dx = (rxf - cx).abs();
    let dy = (ryf - cy).abs();
    let dz = (rzf - cz).abs();
    if dx > dy && dx > dz {
        rxf = -ryf - rzf;
    } else if dy > dz {
        ryf = -rxf - rzf;
    } else {
        rzf = -rxf - ryf;
    }
    let _ = ryf;
    (rxf as i64, rzf as i64)
}

fn px_to_hex(x: f64, y: f64) -> (i64, i64) {
    let qf = (SQRT3 / 3.0 * x - 1.0 / 3.0 * y) / HEX_SIZE;
    let rf = (2.0 / 3.0 * y) / HEX_SIZE;
    let (aq, ar) = hex_round(qf, rf);
    let col = aq + (ar - (ar & 1)) / 2;
    (col, ar)
}

// ── The rows ────────────────────────────────────────────────────────

fn neighbor_op(r: i64) -> [i64; 3] {
    let mut q = r & 1;
    let mut rr = 0i64;
    let mut acc = 0i64;
    for i in 0..STEPS {
        let d = (i + (q & 7) + (rr & 3)) % 6;
        (q, rr) = hex_neighbor(q, rr, d);
        acc += q * 3 + rr;
    }
    [q, rr, acc]
}

fn bench_neighbor(n: i64) -> Row {
    let (us, sink) = timed(n, |r| neighbor_op(r)[2]);
    Row { name: "hex_neighbor", iters: n, us, px: STEPS,
          hash: fnv(FNV_OFFSET, &neighbor_op(black_box(0))), sink }
}

fn px_op(r: i64) -> [i64; 3] {
    let salt = ((r & 1) as f64) * 0.25;
    let mut acc = 0i64;
    let mut c = 0i64;
    let mut rw = 0i64;
    for i in 0..POINTS {
        let x = ((i * 7919) % 20011) as f64 * 0.0173 - 150.0 + salt;
        let y = ((i * 104729) % 19997) as f64 * 0.0151 - 140.0;
        (c, rw) = px_to_hex(x, y);
        acc += c * 1000 + rw;
    }
    [c, rw, acc]
}

fn bench_px(n: i64) -> Row {
    let (us, sink) = timed(n, |r| px_op(r)[2]);
    Row { name: "px_to_hex", iters: n, us, px: POINTS, hash: fnv(FNV_OFFSET, &px_op(black_box(0))), sink }
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
    let rows = [bench_neighbor(n), bench_px(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
