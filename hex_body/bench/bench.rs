// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// hex_body-reference — the pure-Rust twin of the `hex_body` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison.  Plain idiomatic Rust.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// The ports, from src/hex_body.loft:
//   `rig_world_frame3`  the same Rodrigues rotation and 3x3 composition, term for term, but
//                       each bone's frame is one `[f64; 12]` in a stack array — no heap.
//   `bone_shape_has`    `rig_world_seg` + `seg_dist`, with the chain walk HOISTED: the
//                       library re-walks bones 0..=i on every query, the twin walks the rig
//                       once per op and answers each query from the stored segments.  Every
//                       segment is bit-identical either way, so the gap is the re-walk.
//   `rig_read`          the same strict reader (field counts, keywords, the re-spelling
//                       identity), but each line is split ONCE into its words, where the
//                       library's word helpers re-split it for every field they read.
// `black_box` guards each op's INPUT (the repetition number) and the sink — never anything
// inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const TAU: f64 = 6.283185307179586;

const BONES3: usize = 24;
const CALLS: usize = 12000;
const BONES2: usize = 12;
const QUERIES: i64 = 20000;
const TEXTS: usize = 8;
const PARSES: usize = 40;

/// The most bones a frame walk carries on the stack.
const MAXB: usize = 32;

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

fn micro(x: f64) -> i64 {
    (x * 1000000.0) as i64
}

// ── hex_body.loft: the rig ──────────────────────────────────────────

#[derive(Clone, Copy, Default)]
struct Bone {
    parent: i64,
    ox: f64,
    oy: f64,
    oz: f64,
    len: f64,
    ax: f64,
    ay: f64,
    az: f64,
    lo: f64,
    hi: f64,
}

impl Bone {
    fn planar(&self) -> bool {
        self.oz == 0.0 && self.ax == 0.0 && self.ay == 0.0 && self.az == 1.0
    }
}

fn rig_write(rig: &[Bone], name: &str) -> String {
    let mut s = format!("rig {} bones {}", name, rig.len());
    for (bi, b) in rig.iter().enumerate() {
        if b.planar() {
            s += &format!(
                "\nbone {} parent {} at {} {} len {} lim {} {}",
                bi, b.parent, b.ox, b.oy, b.len, b.lo, b.hi
            );
        } else {
            s += &format!(
                "\nbone3 {} parent {} at {} {} {} len {} axis {} {} {} lim {} {}",
                bi, b.parent, b.ox, b.oy, b.oz, b.len, b.ax, b.ay, b.az, b.lo, b.hi
            );
        }
    }
    s
}

/// Is `w` a number spelled the way `rig_write` spells one?  The parse must re-print as `w`.
fn is_int(w: &str) -> bool {
    let v: i64 = w.parse().unwrap_or(0);
    v.to_string() == w
}

fn is_float(w: &str) -> bool {
    let v: f64 = w.parse().unwrap_or(0.0);
    v.to_string() == w
}

fn word_int(w: &str) -> i64 {
    w.parse().unwrap_or(0)
}

fn word_float(w: &str) -> f64 {
    w.parse().unwrap_or(0.0)
}

/// The strict reader: exactly what `rig_write` emits, or an empty rig.
fn rig_read(t: &str) -> Vec<Bone> {
    let lines: Vec<&str> = t.split('\n').collect();
    let nl = lines.len();
    let head: Vec<&str> = lines[0].split(' ').collect();
    let hw = |i: usize| head.get(i).copied().unwrap_or("");
    if hw(0) != "rig" || hw(2) != "bones" || head.len() != 4 || !is_int(hw(3)) {
        return Vec::new();
    }
    let nb = word_int(hw(3));
    if nl as i64 != nb + 1 {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(nl - 1);
    for (ri, ln) in lines.iter().enumerate().skip(1) {
        let ws: Vec<&str> = ln.split(' ').collect();
        let w = |i: usize| ws.get(i).copied().unwrap_or("");
        let floats_ok = |lo: usize, hi: usize| (lo..=hi).all(|i| is_float(w(i)));
        if word_int(w(1)) != ri as i64 - 1 || w(2) != "parent" || !is_int(w(1)) || !is_int(w(3)) {
            return Vec::new();
        }
        let tag = w(0);
        if tag == "bone" {
            if ws.len() != 12 || w(4) != "at" || w(7) != "len" || w(9) != "lim" {
                return Vec::new();
            }
            if !floats_ok(5, 6) || !is_float(w(8)) || !floats_ok(10, 11) {
                return Vec::new();
            }
            out.push(Bone {
                parent: word_int(w(3)),
                ox: word_float(w(5)),
                oy: word_float(w(6)),
                oz: 0.0,
                len: word_float(w(8)),
                ax: 0.0,
                ay: 0.0,
                az: 1.0,
                lo: word_float(w(10)),
                hi: word_float(w(11)),
            });
        } else if tag == "bone3" {
            if ws.len() != 17 || w(4) != "at" || w(8) != "len" || w(10) != "axis" || w(14) != "lim" {
                return Vec::new();
            }
            if !floats_ok(5, 7) || !is_float(w(9)) || !floats_ok(11, 13) || !floats_ok(15, 16) {
                return Vec::new();
            }
            out.push(Bone {
                parent: word_int(w(3)),
                ox: word_float(w(5)),
                oy: word_float(w(6)),
                oz: word_float(w(7)),
                len: word_float(w(9)),
                ax: word_float(w(11)),
                ay: word_float(w(12)),
                az: word_float(w(13)),
                lo: word_float(w(15)),
                hi: word_float(w(16)),
            });
        } else {
            return Vec::new();
        }
    }
    out
}

/// A bone's world frame at bone `i`: `[x, y, z, m00, m01, m02, m10, m11, m12, m20, m21, m22]`.
fn rig_world_frame3(rig: &[Bone], values: &[f64], i: usize) -> [f64; 12] {
    let n = i + 1;
    let mut fr = [[0.0f64; 12]; MAXB];
    for k in 0..n {
        let th = values[k] * TAU;
        let mut c = th.cos();
        let mut s = th.sin();
        let b = &rig[k];
        let (ax, ay, az) = (b.ax, b.ay, b.az);
        let al = (ax * ax + ay * ay + az * az).sqrt();
        let (mut ux, mut uy, mut uz) = (0.0, 0.0, 0.0);
        if al > 0.0 {
            ux = ax / al;
            uy = ay / al;
            uz = az / al;
        } else {
            c = 1.0;
            s = 0.0;
        }
        let omc = 1.0 - c;
        let l = [
            c + omc * ux * ux,
            omc * ux * uy - s * uz,
            omc * ux * uz + s * uy,
            omc * uy * ux + s * uz,
            c + omc * uy * uy,
            omc * uy * uz - s * ux,
            omc * uz * ux - s * uy,
            omc * uz * uy + s * ux,
            c + omc * uz * uz,
        ];
        if b.parent < 0 {
            let f = &mut fr[k];
            f[0] = 0.0;
            f[1] = 0.0;
            f[2] = 0.0;
            f[3..12].copy_from_slice(&l);
        } else {
            let p = fr[b.parent as usize];
            let (gx, gy, gz) = (b.ox, b.oy, b.oz);
            let f = &mut fr[k];
            f[0] = p[0] + p[3] * gx + p[4] * gy + p[5] * gz;
            f[1] = p[1] + p[6] * gx + p[7] * gy + p[8] * gz;
            f[2] = p[2] + p[9] * gx + p[10] * gy + p[11] * gz;
            for row in 0..3 {
                let (p0, p1, p2) = (p[3 + 3 * row], p[4 + 3 * row], p[5 + 3 * row]);
                for col in 0..3 {
                    f[3 + 3 * row + col] = p0 * l[col] + p1 * l[3 + col] + p2 * l[6 + col];
                }
            }
        }
    }
    fr[i]
}

fn pose_of(value: f64, lx: f64, ly: f64) -> (f64, f64) {
    let th = value * TAU;
    let c = th.cos();
    let s = th.sin();
    (lx * c - ly * s, lx * s + ly * c)
}

/// Every bone's world segment `(x0, y0, x1, y1)` — the planar walk, done once.
fn rig_world_segs(rig: &[Bone], values: &[f64]) -> Vec<(f64, f64, f64, f64)> {
    let n = rig.len();
    let mut bx = Vec::with_capacity(n);
    let mut by = Vec::with_capacity(n);
    let mut th = Vec::with_capacity(n);
    for (k, b) in rig.iter().enumerate() {
        let v = values[k];
        if b.parent < 0 {
            bx.push(0.0);
            by.push(0.0);
            th.push(v);
        } else {
            let p = b.parent as usize;
            let pth = th[p];
            let (rx, ry) = pose_of(pth, b.ox, b.oy);
            bx.push(bx[p] + rx);
            by.push(by[p] + ry);
            th.push(pth + v);
        }
    }
    (0..n)
        .map(|i| {
            let (x0, y0) = (bx[i], by[i]);
            let (ex, ey) = pose_of(th[i], rig[i].len, 0.0);
            (x0, y0, x0 + ex, y0 + ey)
        })
        .collect()
}

fn seg_dist(x0: f64, y0: f64, x1: f64, y1: f64, px: f64, py: f64) -> f64 {
    let dx = x1 - x0;
    let dy = y1 - y0;
    let l2 = dx * dx + dy * dy;
    if l2 < 0.000000000001 {
        return ((px - x0) * (px - x0) + (py - y0) * (py - y0)).sqrt();
    }
    let mut t = ((px - x0) * dx + (py - y0) * dy) / l2;
    if t < 0.0 {
        t = 0.0;
    }
    if t > 1.0 {
        t = 1.0;
    }
    let cx = x0 + t * dx;
    let cy = y0 + t * dy;
    ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt()
}

// ── The rigs ────────────────────────────────────────────────────────

fn parent_of(k: i64) -> i64 {
    if k == 0 { -1 } else { (k - 1) / 2 }
}

fn rig3(n: i64, salt: i64) -> Vec<Bone> {
    (0..n)
        .map(|k| {
            let kk = k + salt;
            let (ax, ay, az) = match kk % 6 {
                1 => (0.0, 2.0, 0.0),
                2 => (3.0, 0.0, 0.0),
                3 => (1.0, 1.0, 0.0),
                4 => (0.0, 1.0, 1.0),
                5 => (1.0, 2.0, 2.0),
                _ => (0.0, 0.0, 1.0),
            };
            Bone {
                parent: parent_of(k),
                ox: ((kk * 5) % 7) as f64 * 0.25 + 0.5,
                oy: ((kk * 3) % 5 - 2) as f64 * 0.125,
                oz: ((kk % 4) as f64 - 1.5) * 0.25,
                len: 1.0 + (kk % 3) as f64 * 0.5,
                ax,
                ay,
                az,
                lo: -0.25,
                hi: 0.25 + (kk % 4) as f64 * 0.125,
            }
        })
        .collect()
}

fn rig2(n: i64) -> Vec<Bone> {
    (0..n)
        .map(|k| Bone {
            parent: parent_of(k),
            ox: ((k * 5) % 7) as f64 * 0.25 + 0.5,
            oy: ((k * 3) % 5 - 2) as f64 * 0.125,
            oz: 0.0,
            len: 1.0 + (k % 3) as f64 * 0.5,
            ax: 0.0,
            ay: 0.0,
            az: 1.0,
            lo: -0.25,
            hi: 0.25,
        })
        .collect()
}

fn joint_values(n: usize, r: i64) -> Vec<f64> {
    (0..n).map(|k| (k as f64) * 0.0625 - 0.5 + ((r & 1) as f64) * 0.03125).collect()
}

// ── The rows ────────────────────────────────────────────────────────

fn frame_op(rig: &[Bone], r: i64) -> Vec<i64> {
    let v = joint_values(BONES3, r);
    let mut s = [0.0f64; 12];
    for c in 0..CALLS {
        let f = rig_world_frame3(rig, &v, c % BONES3);
        for j in 0..12 {
            s[j] += f[j];
        }
    }
    s.iter().map(|&x| micro(x)).collect()
}

fn bench_frame3(n: i64) -> Row {
    let rig = rig3(BONES3 as i64, 0);
    let (us, sink) = timed(n, |r| frame_op(&rig, r)[0]);
    Row { name: "rig_world_frame3", iters: n, us, px: CALLS as i64,
          hash: fnv(FNV_OFFSET, &frame_op(&rig, black_box(0))), sink }
}

fn shape_op(rig: &[Bone], r: i64) -> [i64; 2] {
    let v = joint_values(BONES2, r);
    let segs = rig_world_segs(rig, &v);
    let mut s: i64 = 12345;
    let mut hits = 0i64;
    let mut first = -1i64;
    for c in 0..QUERIES {
        s = (s * 1103515245 + 12345) & 0x7FFF_FFFF;
        let px = ((s >> 4) & 4095) as f64 / 256.0 - 8.0;
        let py = ((s >> 16) & 4095) as f64 / 256.0 - 8.0;
        let (x0, y0, x1, y1) = segs[(c % BONES2 as i64) as usize];
        if seg_dist(x0, y0, x1, y1, px, py) <= 0.5 {
            hits += 1;
            if first < 0 {
                first = c;
            }
        }
    }
    [hits, first]
}

fn bench_shape(n: i64) -> Row {
    let rig = rig2(BONES2 as i64);
    let (us, sink) = timed(n, |r| shape_op(&rig, r)[0]);
    Row { name: "bone_shape_has", iters: n, us, px: QUERIES,
          hash: fnv(FNV_OFFSET, &shape_op(&rig, black_box(0))), sink }
}

fn read_op(texts: &[String], r: i64) -> Vec<i64> {
    let (mut bones, mut par, mut pos, mut len, mut axis, mut lim) = (0i64, 0i64, 0i64, 0i64, 0i64, 0i64);
    for j in 0..PARSES {
        let rig = rig_read(&texts[(j + (r & 1) as usize) % TEXTS]);
        for (k, b) in rig.iter().enumerate() {
            bones += 1;
            par += b.parent * (k as i64 + 1);
            pos += micro(b.ox) + 3 * micro(b.oy) + 7 * micro(b.oz);
            len += micro(b.len);
            axis += micro(b.ax) + 3 * micro(b.ay) + 7 * micro(b.az);
            lim += micro(b.lo) + 3 * micro(b.hi);
        }
    }
    vec![bones, par, pos, len, axis, lim]
}

fn bench_read(n: i64) -> Row {
    let texts: Vec<String> = (0..TEXTS as i64)
        .map(|t| {
            if t % 2 == 0 {
                rig_write(&rig3(BONES3 as i64, t), &format!("arm{t}"))
            } else {
                rig_write(&rig2(BONES3 as i64), &format!("leg{t}"))
            }
        })
        .collect();
    let (us, sink) = timed(n, |r| read_op(&texts, r)[0]);
    let mut out = read_op(&texts, black_box(0));
    out.push(texts[0].len() as i64);
    out.push(texts[1].len() as i64);
    Row { name: "rig_read", iters: n, us, px: PARSES as i64, hash: fnv(FNV_OFFSET, &out), sink }
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
    let rows = [bench_frame3(n), bench_shape(n), bench_read(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
