// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_terrain-reference — the pure-Rust twin of the `hex_terrain` package's performance
// pass (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workload
// with the SAME arithmetic, in the same order, and prints the same row, hash included: a
// row whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic
// Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The port of src/hex_terrain.loft's `terrain_fbm` and what it calls (`terrain_vnoise`,
// `terrain_hash01`, `th_hash`): the same masked integer hash in i64 and the same smoothstep
// lerp in f64.  `black_box` guards each op's INPUT and the sink — never anything inside a
// kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const CALLS: i64 = 100000;
const OCTAVES: i64 = 8;

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

// ── hex_terrain.loft ────────────────────────────────────────────────

fn th_hash(seed: i64, ix: i64, iy: i64, ch: i64) -> i64 {
    let a = (ix * 73856093) & 0x7FFF_FFFF;
    let b = (iy * 19349663) & 0x7FFF_FFFF;
    let c = (ch * 83492791) & 0x7FFF_FFFF;
    let s = (seed * 1597334677) & 0x7FFF_FFFF;
    let mut n = a ^ b ^ c ^ s;
    n ^= n >> 13;
    n = (n * 1274126177) & 0x7FFF_FFFF;
    n ^ (n >> 16)
}

fn terrain_hash01(seed: i64, ix: i64, iy: i64, ch: i64) -> f64 {
    (th_hash(seed, ix, iy, ch) & 0x7FFF_FFFF) as f64 / 2147483648.0
}

fn terrain_vnoise(seed: i64, x: f64, y: f64, wl: f64, ch: i64) -> f64 {
    let u = x / wl;
    let v = y / wl;
    let fu = u.floor();
    let fv = v.floor();
    let (iu, iv) = (fu as i64, fv as i64);
    let du = u - fu;
    let dv = v - fv;
    let su = du * du * (3.0 - 2.0 * du);
    let sv = dv * dv * (3.0 - 2.0 * dv);
    let a = terrain_hash01(seed, iu, iv, ch);
    let b = terrain_hash01(seed, iu + 1, iv, ch);
    let c = terrain_hash01(seed, iu, iv + 1, ch);
    let d = terrain_hash01(seed, iu + 1, iv + 1, ch);
    (a * (1.0 - su) + b * su) * (1.0 - sv) + (c * (1.0 - su) + d * su) * sv
}

fn terrain_fbm(seed: i64, x: f64, y: f64, wl0: f64, octaves: i64, ch: i64) -> f64 {
    let mut out = 0.0;
    let mut amp = 1.0;
    let mut wl = wl0;
    let mut tot = 0.0;
    for o in 0..octaves {
        out += amp * (terrain_vnoise(seed, x, y, wl, ch * 16 + o) * 2.0 - 1.0);
        tot += amp;
        amp *= 0.5;
        wl *= 0.5;
    }
    out / tot
}

// ── The row ─────────────────────────────────────────────────────────

fn fbm_op(r: i64) -> [i64; 2] {
    let salt = ((r & 1) as f64) * 0.5;
    let mut sum = 0.0;
    let mut v = 0.0;
    for i in 0..CALLS {
        let x = ((i * 7919) % 20011) as f64 * 3.7 + salt;
        let y = ((i * 104729) % 19997) as f64 * 2.9 - 5000.0;
        v = terrain_fbm(7, x, y, 700.0, OCTAVES, 3);
        sum += v;
    }
    [(sum * 1000000.0) as i64, (v * 1000000.0) as i64]
}

fn bench_fbm(n: i64) -> Row {
    let (us, sink) = timed(n, |r| fbm_op(r)[0]);
    Row { name: "terrain_fbm", iters: n, us, px: CALLS, hash: fnv(FNV_OFFSET, &fbm_op(black_box(0))), sink }
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
    let rows = [bench_fbm(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
