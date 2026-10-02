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
// lerp in f64.  Then `terrain_surface_at` with `terrain_detail_at` / `terrain_ridge_at`
// (the per-type weights in a `[f64; 8]` stack array rather than an allocated vector),
// `terrain_hydrology` (the same selection flood and selection-ordered accumulation — a
// BinaryHeap would change the algorithm) and `terrain_relief_pass` (the type read by direct
// index), over a `Terrain` of seven parallel `Vec`s; `hex_neighbor` from hex_grid.
// `black_box` guards each op's INPUT and the sink — never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const CALLS: i64 = 100000;
const OCTAVES: i64 = 8;

const TILE: f64 = 300.0;
const SURF_NX: i64 = 96;
const SURF_GRID: i64 = 128;
const HYD_NX: i64 = 64;
const REL_NX: i64 = 128;
const RELIEFS: i64 = 20;

const SQRT3: f64 = 1.7320508075688772;

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

struct TerrainType {
    rise: f64,
    steep: f64,
    wet: bool,
}

#[derive(Clone)]
struct TerrainParams {
    seed: i64,
    tile: f64,
    sea: f64,
    lake_fill: f64,
    acc_min: f64,
    nb_base: f64,
    nb_boost: f64,
    steep_delta: f64,
    detail_wl: f64,
    ridge_wl: f64,
    warp: f64,
}

fn terrain_params(seed: i64, tile: f64) -> TerrainParams {
    TerrainParams { seed, tile, sea: 0.0, lake_fill: 4.0, acc_min: 6.0, nb_base: 0.6,
                    nb_boost: 0.2, steep_delta: 60.0, detail_wl: 420.0, ridge_wl: 950.0, warp: 240.0 }
}

struct Terrain {
    nx: i64,
    ny: i64,
    h: Vec<f64>,
    mat: Vec<i64>,
    moist: Vec<f64>,
    flow: Vec<i64>,
    acc: Vec<f64>,
    relief: Vec<f64>,
    wet: Vec<bool>,
}

impl Terrain {
    fn new(nx: i64, ny: i64) -> Terrain {
        let n = (nx * ny) as usize;
        Terrain { nx, ny, h: vec![0.0; n], mat: vec![0; n], moist: vec![0.5; n], flow: vec![-1; n],
                  acc: vec![1.0; n], relief: vec![0.0; n], wet: vec![false; n] }
    }
}

fn terrain_detail_at(p: &TerrainParams, x: f64, y: f64) -> f64 {
    let wx = terrain_fbm(p.seed, x, y, 700.0, 2, 80);
    let wy = terrain_fbm(p.seed, x, y, 700.0, 2, 81);
    terrain_fbm(p.seed, x + p.warp * wx, y + p.warp * wy, p.detail_wl, 8, 3)
}

fn terrain_ridge_at(p: &TerrainParams, x: f64, y: f64) -> f64 {
    let wx = terrain_fbm(p.seed, x, y, 1500.0, 2, 84);
    let wy = terrain_fbm(p.seed, x, y, 1500.0, 2, 85);
    let r = 1.0 - terrain_fbm(p.seed, x + 300.0 * wx, y + 300.0 * wy, p.ridge_wl, 2, 86).abs();
    r * r * r
}

fn terrain_hydrology(t: &mut Terrain, p: &TerrainParams, lake_mat: i64) {
    let (nx, ny) = (t.nx, t.ny);
    let n = (nx * ny) as usize;
    let orig = t.h.clone();
    let mut level = vec![1.0e18; n];
    let mut state = vec![0u8; n]; // 0 = untouched, 1 = open, 2 = closed
    for r in 0..ny {
        for c in 0..nx {
            let i = (r * nx + c) as usize;
            let edge = c == 0 || r == 0 || c == nx - 1 || r == ny - 1;
            if edge || t.h[i] <= p.sea {
                level[i] = t.h[i];
                state[i] = 1;
            }
        }
    }
    loop {
        let mut best: i64 = -1;
        let mut bl = 1.0e18;
        for i in 0..n {
            if state[i] == 1 && level[i] < bl {
                bl = level[i];
                best = i as i64;
            }
        }
        if best < 0 {
            break;
        }
        state[best as usize] = 2;
        let bc = best % nx;
        let br = (best - bc) / nx;
        for d in 0..6 {
            let (nc, nr) = hex_neighbor(bc, br, d);
            if nc >= 0 && nc < nx && nr >= 0 && nr < ny {
                let j = (nr * nx + nc) as usize;
                if state[j] != 2 {
                    let nl = orig[j].max(bl + 0.05);
                    if nl < level[j] {
                        level[j] = nl;
                    }
                    state[j] = 1;
                }
            }
        }
    }
    for i in 0..n {
        t.h[i] = level[i];
        let raised = level[i] > orig[i] + p.lake_fill;
        if raised && level[i] > p.sea {
            t.mat[i] = lake_mat;
        }
    }
    for r in 0..ny {
        for c in 0..nx {
            let i = (r * nx + c) as usize;
            t.flow[i] = -1;
            t.acc[i] = 1.0;
            if t.h[i] > p.sea {
                let mut bh = t.h[i];
                let mut bi = -1;
                for d in 0..6 {
                    let (nc, nr) = hex_neighbor(c, r, d);
                    if nc >= 0 && nc < nx && nr >= 0 && nr < ny && t.h[(nr * nx + nc) as usize] < bh {
                        bh = t.h[(nr * nx + nc) as usize];
                        bi = d;
                    }
                }
                t.flow[i] = bi;
            }
        }
    }
    let mut taken = vec![false; n];
    for _ in 0..n {
        let mut best: i64 = -1;
        let mut bh = 0.0 - 1.0e18;
        for i in 0..n {
            if !taken[i] && t.h[i] > bh {
                bh = t.h[i];
                best = i as i64;
            }
        }
        if best >= 0 {
            let b = best as usize;
            taken[b] = true;
            let bc = best % nx;
            let br = (best - bc) / nx;
            let bi = t.flow[b];
            if bi >= 0 && t.h[b] > p.sea {
                let (nc, nr) = hex_neighbor(bc, br, bi);
                if nc >= 0 && nc < nx && nr >= 0 && nr < ny {
                    let j = (nr * nx + nc) as usize;
                    t.acc[j] = t.acc[j] + t.acc[b];
                }
            }
        }
    }
}

fn terrain_relief_pass(t: &mut Terrain, types: &[TerrainType], p: &TerrainParams) {
    let (nx, ny) = (t.nx, t.ny);
    for r in 0..ny {
        for c in 0..nx {
            let i = (r * nx + c) as usize;
            let wetmat = types.get(t.mat[i] as usize).is_some_and(|ty| ty.wet);
            t.wet[i] = wetmat || t.h[i] <= p.sea || t.acc[i] >= p.acc_min;
        }
    }
    for r in 0..ny {
        for c in 0..nx {
            let i = (r * nx + c) as usize;
            t.relief[i] = 0.0;
            if !t.wet[i] {
                let mut m: f64 = 0.0;
                let mut nsteep = 0;
                for d in 0..6 {
                    let (nc, nr) = hex_neighbor(c, r, d);
                    if nc >= 0 && nc < nx && nr >= 0 && nr < ny {
                        let dh = (t.h[i] - t.h[(nr * nx + nc) as usize]).abs();
                        m = m.max(dh);
                        if dh > p.steep_delta {
                            nsteep += 1;
                        }
                    }
                }
                t.relief[i] = m * (p.nb_base + p.nb_boost * (nsteep as f64));
            }
        }
    }
}

struct TerrainSurf {
    h: f64,
    blend: f64,
    mat: i64,
    mix: f64,
    relief: f64,
    moist: f64,
    watery: f64,
    lakey: f64,
}

fn terrain_surface_at(t: &Terrain, types: &[TerrainType], p: &TerrainParams, x: f64, y: f64) -> TerrainSurf {
    let nt = types.len();
    let mut wm = [0.0f64; 8];
    let s = p.tile / SQRT3;
    let vs = 1.5 * s;
    let rk = p.tile * 1.02;
    let rk2 = rk * rk;
    let ry0 = (y / vs).round() as i64;
    let (mut wsum, mut hsum, mut relsum, mut moisum, mut watsum, mut laksum) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for dr in 0..3 {
        let r = ry0 + dr - 1;
        let off = 0.5 * ((r & 1) as f64);
        let c0 = ((x / p.tile) - off).floor() as i64;
        for dc in 0..4 {
            let c = c0 + dc - 1;
            let cx = ((c as f64) + off) * p.tile;
            let cy = (r as f64) * vs;
            let d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
            if d2 < rk2 {
                let mut w = 1.0 - (d2 / rk2);
                w = w * w;
                let ci = c.max(0).min(t.nx - 1);
                let ri = r.max(0).min(t.ny - 1);
                let i = (ri * t.nx + ci) as usize;
                wsum = wsum + w;
                hsum = hsum + w * t.h[i];
                relsum = relsum + w * t.relief[i];
                moisum = moisum + w * t.moist[i];
                let mi = t.mat[i] as usize;
                if mi < nt {
                    wm[mi] = wm[mi] + w;
                }
                let wetm = types.get(mi).is_some_and(|ty| ty.wet);
                if wetm || t.h[i] <= p.sea {
                    watsum = watsum + w;
                    if t.h[i] > p.sea {
                        laksum = laksum + w;
                    }
                }
            }
        }
    }
    let iw = 1.0 / wsum.max(1.0e-12);
    let hb = hsum * iw;
    let rel = relsum * iw;
    let mut m1 = 0i64;
    let (mut s1, mut s2) = (-1.0, -1.0);
    let (mut steep1, mut steep2, mut rise1, mut rise2) = (0.0, 0.0, 0.0, 0.0);
    for (cm, tyc) in types.iter().enumerate() {
        if !tyc.wet && wm[cm] > 0.0 {
            let jit = 0.72 + 0.56 * (terrain_fbm(p.seed, x, y, 340.0, 3, 40 + cm as i64) * 0.5 + 0.5);
            let steep_root = tyc.steep.max(0.01).sqrt();
            let sc = wm[cm] * iw * steep_root * jit;
            if sc > s1 {
                s2 = s1;
                steep2 = steep1;
                rise2 = rise1;
                m1 = cm as i64;
                s1 = sc;
                steep1 = tyc.steep;
                rise1 = tyc.rise;
            } else if sc > s2 {
                s2 = sc;
                steep2 = tyc.steep;
                rise2 = tyc.rise;
            }
        }
    }
    if s2 < 0.0 {
        s2 = s1;
        steep2 = steep1;
        rise2 = rise1;
    }
    let mut tt = 1.0;
    if s1 > 0.0 {
        tt = ((s1 - s2) / (s1 * 0.18)).max(0.0).min(1.0);
    }
    let mt = 0.5 + 0.5 * tt;
    let steep_eff = steep1 * mt + steep2 * (1.0 - mt);
    let rise_eff = rise1 * mt + rise2 * (1.0 - mt);
    let detail = terrain_detail_at(p, x, y);
    let ridge = terrain_ridge_at(p, x, y);
    let hf = hb + steep_eff * detail + rise_eff * (rel / 100.0) * ridge;
    TerrainSurf { h: hf, blend: hb, mat: m1, mix: mt, relief: rel, moist: moisum * iw,
                  watery: watsum * iw, lakey: laksum * iw }
}

// ── The rows ────────────────────────────────────────────────────────

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

fn micro(x: f64) -> i64 {
    (x * 1000000.0) as i64
}

fn ttype(rise: f64, steep: f64, wet: bool) -> TerrainType {
    TerrainType { rise, steep, wet }
}

fn terrain_types(n: usize) -> Vec<TerrainType> {
    let mut v = vec![ttype(0.0, 0.0, true), ttype(10.0, 4.0, false), ttype(40.0, 25.0, false),
                     ttype(120.0, 80.0, false), ttype(15.0, 8.0, false), ttype(5.0, 2.0, false)];
    if n > 6 {
        v.push(ttype(0.0, 0.0, true));
        v.push(ttype(70.0, 60.0, false));
    }
    v
}

fn gen_height(c: i64, r: i64) -> f64 {
    terrain_fbm(11, (c as f64) * 300.0, (r as f64) * 260.0, 6000.0, 4, 1) * 400.0 + (r as f64) * 6.0 - 60.0
}

fn gen_terrain(nx: i64, ntypes: i64, lift: f64) -> Terrain {
    let mut t = Terrain::new(nx, nx);
    for r in 0..nx {
        for c in 0..nx {
            let i = (r * nx + c) as usize;
            let h = gen_height(c, r) + lift;
            t.h[i] = h;
            t.mat[i] = if h <= 0.0 { 0 } else { 1 + (c / 20 + r / 24) % (ntypes - 1) };
            t.moist[i] = terrain_hash01(3, c, r, 9);
        }
    }
    t
}

fn surface_op(t: &Terrain, types: &[TerrainType], p: &TerrainParams, r: i64) -> [i64; 9] {
    let salt = ((r & 1) as f64) * 0.5;
    let (mut h, mut b, mut mix, mut rel, mut moi, mut wat, mut lak) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    let mut mat = 0i64;
    let mut last = 0.0;
    for sy in 0..SURF_GRID {
        for sx in 0..SURF_GRID {
            let x = 400.0 + (sx as f64) * 215.5 + salt;
            let y = 300.0 + (sy as f64) * 186.5;
            let sf = terrain_surface_at(t, types, p, x, y);
            h += sf.h;
            b += sf.blend;
            mix += sf.mix;
            rel += sf.relief;
            moi += sf.moist;
            wat += sf.watery;
            lak += sf.lakey;
            mat += sf.mat;
            last = sf.h;
        }
    }
    [micro(h), micro(b), micro(mix), micro(rel), micro(moi), micro(wat), micro(lak), mat, micro(last)]
}

fn bench_surface(n: i64) -> Row {
    let types = terrain_types(6);
    let p = terrain_params(7, TILE);
    let mut t = gen_terrain(SURF_NX, 6, 0.0);
    terrain_relief_pass(&mut t, &types, &p);
    let (us, sink) = timed(n, |r| surface_op(&t, &types, &p, r)[7]);
    Row { name: "terrain_surface_at", iters: n, us, px: SURF_GRID * SURF_GRID,
          hash: fnv(FNV_OFFSET, &surface_op(&t, &types, &p, black_box(0))), sink }
}

fn hydrology_op(types: &[TerrainType], p: &TerrainParams, lift: f64) -> [i64; 4] {
    let mut t = gen_terrain(HYD_NX, types.len() as i64, lift);
    terrain_hydrology(&mut t, p, 6);
    let (mut h, mut a, mut m, mut f) = (0.0, 0.0, 0i64, 0i64);
    for i in 0..(HYD_NX * HYD_NX) as usize {
        h += t.h[i];
        a += t.acc[i];
        m += t.mat[i];
        f += t.flow[i] * (i as i64 % 97 + 1);
    }
    [micro(h), micro(a), m, f]
}

fn bench_hydrology(n: i64) -> Row {
    let types = terrain_types(8);
    let p = terrain_params(7, TILE);
    let (us, sink) = timed(n, |r| hydrology_op(&types, &p, ((r & 1) as f64) * 0.25)[2]);
    Row { name: "terrain_hydrology", iters: n, us, px: HYD_NX * HYD_NX,
          hash: fnv(FNV_OFFSET, &hydrology_op(&types, &p, black_box(0.0))), sink }
}

fn relief_op(t: &mut Terrain, types: &[TerrainType], p: &TerrainParams) -> [i64; 2] {
    for _ in 0..RELIEFS {
        terrain_relief_pass(t, types, p);
    }
    let rel: f64 = t.relief.iter().fold(0.0, |acc, &v| acc + v);
    let wet = t.wet.iter().filter(|&&w| w).count() as i64;
    [micro(rel), wet]
}

fn bench_relief(n: i64) -> Row {
    let types = terrain_types(8);
    let p0 = terrain_params(7, TILE);
    let mut p1 = terrain_params(7, TILE);
    p1.nb_base = 0.65;
    let mut t = gen_terrain(REL_NX, 8, 0.0);
    let (us, sink) = timed(n, |r| relief_op(&mut t, &types, if (r & 1) == 0 { &p0 } else { &p1 })[1]);
    Row { name: "terrain_relief_pass", iters: n, us, px: RELIEFS * REL_NX * REL_NX,
          hash: fnv(FNV_OFFSET, &relief_op(&mut t, &types, black_box(&p0))), sink }
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
    let rows = [bench_fbm(n), bench_surface(n), bench_hydrology(n), bench_relief(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
