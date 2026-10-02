// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_world-reference — the pure-Rust twin of the `hex_world` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workload with
// the SAME arithmetic, in the same order, and prints the same row, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The port of src/hex_world.loft's `get_cell` (and the `set_cell` / `ensure_chunk` that
// build the world): a `Vec<Chunk>` searched linearly on two fields, the Cell returned by
// value.  `black_box` guards each op's INPUT and the sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const CHUNKS: i64 = 200;
const READS: i64 = 200000;

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

// ── hex_world.loft ──────────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
struct Cell {
    color: u8,
    height: u8,
    age: u16,
}

struct Chunk {
    cx: i64,
    cz: i64,
    cells: Vec<Cell>,
}

struct World {
    chunks: Vec<Chunk>,
}

fn chunk_idx_32(v: i64) -> i64 {
    let vr = v % 32;
    if vr < 0 { (v - vr - 32) / 32 } else { (v - vr) / 32 }
}

fn hex_idx_32(v: i64) -> i64 {
    let vr = v % 32;
    if vr < 0 { vr + 32 } else { vr }
}

impl World {
    fn ensure_chunk(&mut self, q: i64, r: i64) {
        let (cx, cz) = (chunk_idx_32(q), chunk_idx_32(r));
        if self.chunks.iter().any(|c| c.cx == cx && c.cz == cz) {
            return;
        }
        self.chunks.push(Chunk { cx, cz, cells: vec![Cell::default(); 1024] });
    }

    fn get_cell(&self, q: i64, r: i64) -> Cell {
        let (cx, cz) = (chunk_idx_32(q), chunk_idx_32(r));
        let idx = (hex_idx_32(q) * 32 + hex_idx_32(r)) as usize;
        for c in &self.chunks {
            if c.cx == cx && c.cz == cz {
                return c.cells.get(idx).copied().unwrap_or_default();
            }
        }
        Cell::default()
    }

    fn set_cell(&mut self, q: i64, r: i64, value: Cell) {
        self.ensure_chunk(q, r);
        let (cx, cz) = (chunk_idx_32(q), chunk_idx_32(r));
        let idx = (hex_idx_32(q) * 32 + hex_idx_32(r)) as usize;
        for c in &mut self.chunks {
            if c.cx == cx && c.cz == cz {
                if idx < c.cells.len() {
                    c.cells[idx] = value;
                }
                return;
            }
        }
    }
}

// ── The row ─────────────────────────────────────────────────────────

fn build_world(salt: i64) -> World {
    let mut w = World { chunks: Vec::new() };
    for k in 0..CHUNKS {
        let cx = k % 20 - 10;
        let cz = k / 20 - 5;
        for j in 0..32 {
            let hx = (j * 5 + k) & 31;
            let hz = (j * 11 + k * 3) & 31;
            let v = j + k + salt;
            w.set_cell(cx * 32 + hx, cz * 32 + hz,
                       Cell { color: (1 + v % 9) as u8, height: (v & 127) as u8, age: (v & 1023) as u16 });
        }
    }
    w
}

fn read_op(w: &World) -> [i64; 2] {
    let mut acc = 0i64;
    let mut filled = 0i64;
    for i in 0..READS {
        let q = (i * 7919) % 640 - 320;
        let r = (i * 104729) % 320 - 160;
        let c = w.get_cell(q, r);
        if c.color != 0 {
            filled += 1;
        }
        acc += c.color as i64 + c.height as i64 * 3 + c.age as i64 * 7;
    }
    [filled, acc]
}

fn bench_get_cell(n: i64) -> Row {
    let w0 = build_world(0);
    let w1 = build_world(1);
    let (us, sink) = timed(n, |r| read_op(if (r & 1) == 0 { &w0 } else { &w1 })[1]);
    Row { name: "get_cell", iters: n, us, px: READS, hash: fnv(FNV_OFFSET, &read_op(black_box(&w0))), sink }
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
    let rows = [bench_get_cell(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
