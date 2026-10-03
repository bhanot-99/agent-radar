//! Group A -- Code & Docs. Full-bleed micro-dot shaders; see `dots.rs`.

use super::dots::*;
use super::Ctx;

/// Sculpted DedSec skull height field in local coords (~[-0.65,0.65] x
/// [-0.85,0.62]); `par` slides the face features for a fake head-turn.
pub(super) fn skull_height(u: f32, v: f32, par: f32) -> f32 {
    let cran = sd_ellipse(u, v + 0.22, 0.62, 0.58);
    let cheek = sd_ellipse(u, v - 0.1, 0.53, 0.3);
    let jaw = sd_round_box(u, v - 0.36, 0.33, 0.22, 0.12);
    let d = smin(smin(cran, cheek, 0.14), jaw, 0.1);
    if d > 0.0 {
        return 0.0;
    }
    let mut h = smoothstep(0.0, 0.25, -d);
    h *= 0.7 + 0.3 * (1.0 - (u * u + (v + 0.25) * (v + 0.25)).min(1.0));
    let uu = u - par;
    for sx in [-0.23f32, 0.23] {
        let e = sd_ellipse(uu - sx, v - 0.03, 0.155, 0.125);
        if e < 0.04 {
            h -= smoothstep(0.04, -0.09, e) * 1.1;
        }
    }
    let nd = sd_ellipse(uu, v - 0.26, 0.05, 0.09);
    if nd < 0.03 {
        h -= smoothstep(0.03, -0.05, nd) * 1.0;
    }
    if v > 0.42 && v < 0.58 && uu.abs() < 0.24 {
        let gap = (fract(uu * 13.0 + 0.5) - 0.5).abs();
        if gap < 0.09 {
            h -= 0.55;
        }
        if (v - 0.5).abs() < 0.014 {
            h -= 0.8;
        }
    }
    // Temple hollows and a DedSec crack across the cranium.
    let crack = (v + 0.45 + 0.12 * (uu * 9.0).sin() * 0.4 - uu * 0.3).abs();
    if crack < 0.012 && uu > -0.1 && uu < 0.45 {
        h -= 0.5;
    }
    h.max(0.0)
}

/// Circuit-board traces with pulses racing along them, built on a dot-space
/// grid so every trace is exactly one dot wide.
pub(super) fn circuit(x: usize, y: usize, t: f32, seed: u32, cell: usize) -> (f32, u8) {
    let cx = (x / cell) as u32;
    let cy = (y / cell) as u32;
    let fx = (x % cell) as f32 / cell as f32;
    let fy = (y % cell) as f32 / cell as f32;
    let mid = cell / 2;
    let hc = hash(seed ^ cx.wrapping_mul(73_856_093) ^ cy.wrapping_mul(19_349_663));
    let mut l = 0.0f32;
    let mut tint = TINT_ACCENT;
    if hc & 3 == 1 && y % cell == mid {
        l = 0.2;
        let p = fract(t * (0.6 + rnd(hc) * 0.9) + rnd(hc ^ 9));
        if (fx - p).abs() < 0.18 {
            l = 0.95;
            tint = TINT_ICE;
        }
    }
    if hc & 12 == 4 && x % cell == mid {
        l = l.max(0.2);
        let p = fract(t * (0.5 + rnd(hc ^ 3)) + rnd(hc ^ 7));
        if (fy - p).abs() < 0.18 {
            l = 0.95;
            tint = TINT_ICE;
        }
    }
    if hc & 48 == 48 && (x % cell).abs_diff(mid) <= 1 && (y % cell).abs_diff(mid) <= 1 {
        l = l.max(0.55);
    }
    (l, tint)
}

/// `RustEdit` -- ANATOMICAL-FORGE: a sculpted, stipple-lit DedSec skull on a
/// live circuit board, a borrow-checker laser slicing across its face and
/// vaporizing it into rising dot-ash; twin DNA helices spiral at the flanks.
pub fn rust_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let laser = |tt: f32| 0.72 * (tt * 0.9).sin();
    let ly = laser(t);
    let light = norm3(((t * 0.7).cos(), (t * 0.7).sin() * 0.6 - 0.35, 0.85));
    let par = 0.07 * (t * 0.5).sin();
    let sc = 1.2;
    let px = f.px();
    let seed = c.seed;
    let cell = ((f.h as f32) / 9.0).max(5.0) as usize;
    f.shade(|u, v, x, y| {
        let mut out = circuit(x, y, t, seed, cell);
        out.0 *= smoothstep(0.55, 1.2, (u / a).abs() * 2.0 + 0.3);
        let (h, lam) = relief(&|p, q| skull_height(p, q, par), (u - par * 0.4) / sc, (v - 0.05) / sc, 0.02, 2.2, light);
        if h > 0.0 {
            let l = (0.22 + 0.78 * lam) * (0.62 + 0.38 * h.min(1.0));
            let dl = (v - ly).abs();
            out = if dl < 0.14 {
                (l.max(0.95 - dl * 3.0), TINT_GLITCH)
            } else if l > 0.88 {
                (l, TINT_HOT)
            } else {
                (l, TINT_ACCENT)
            };
        }
        let dl = (v - ly).abs();
        if dl < px * 0.75 {
            out = (1.0, TINT_HOT);
        } else if dl < px * 1.6 && out.0 < 0.3 {
            out = (0.3, TINT_GLITCH);
        }
        out
    });
    // DNA double helices at both flanks.
    for side in [-1.0f32, 1.0] {
        let cx = side * (a * 0.62 + 0.15);
        for i in 0..90 {
            let k = i as f32 / 90.0;
            let v = -1.0 + k * 2.0;
            let ph = v * 5.0 + t * 2.2 * side;
            let (s, co) = ph.sin_cos();
            let w = 0.22;
            f.plot(cx + s * w, v, 0.55 + 0.45 * co.max(0.0), TINT_ACCENT);
            f.plot(cx - s * w, v, 0.55 - 0.45 * co.min(0.0), TINT_ICE);
            if i % 5 == 0 {
                f.line((cx - s * w, v), (cx + s * w, v), 0.3, TINT_ACCENT);
            }
        }
    }
    // Burning eyes deep in the sockets.
    let flare = 0.5 + 0.5 * (t * 3.1).sin();
    for sx in [-0.23f32, 0.23] {
        let (eu, ev) = ((sx + par) * sc + par * 0.4, 0.03 * sc + 0.05);
        f.glow(eu, ev, 3.0 + 2.0 * flare, 1.0, TINT_GLITCH);
        f.glow(eu, ev, 1.2, 1.0, TINT_HOT);
    }
    // Vapor ash released wherever the laser crosses bone.
    for i in 0..700usize {
        let life = fract(t * 0.42 + rndi(i, 1));
        let ts = t - life / 0.42;
        let y0 = laser(ts);
        let x0 = (rndi(i, 2) - 0.5) * 1.25 * sc;
        if skull_height(x0 / sc, (y0 - 0.05) / sc, par) <= 0.0 {
            continue;
        }
        let drift = (rndi(i, 3) - 0.5) * 1.8;
        let u = x0 + drift * life + 0.05 * (life * 9.0 + i as f32).sin();
        let v = y0 - life * 1.1;
        f.plot(u, v, (1.0 - life).powf(0.6), if life < 0.25 { TINT_HOT } else { TINT_GLITCH });
    }
}

/// `PythonEdit` -- PARAMETRIC-VIPER: a fully shaded, scaled serpent tube
/// slithering edge to edge over a living topographic contour map, tongue
/// flicking, eye burning.
pub fn python_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let span = 2.0 * a + 2.4;
    let head = -a - 1.2 + fract(t / 11.0) * span + 1.2;
    let body_len = 2.0 * a + 0.6;
    let spine = move |u: f32| -> f32 { 0.32 * (u * 2.3 - t * 3.2).sin() + 0.14 * (u * 5.3 - t * 1.9).sin() };
    let slope = move |u: f32| -> f32 { 0.32 * 2.3 * (u * 2.3 - t * 3.2).cos() + 0.14 * 5.3 * (u * 5.3 - t * 1.9).cos() };
    let light = norm3((-0.4, -0.7, 0.6));
    f.shade(|u, v, _, _| {
        let n = fbm3(u * 1.3, v * 1.3 + t * 0.05, t * 0.08, 3);
        let band = (fract(n * 11.0) - 0.5).abs();
        let mut out = if band < 0.06 { (0.28, TINT_ACCENT) } else { (0.0, TINT_ACCENT) };
        let behind = head - u;
        if behind > -0.05 && behind < body_len {
            let taper = smoothstep(body_len, body_len * 0.55, behind) * smoothstep(-0.05, 0.25, behind);
            let r = 0.17 * taper + 0.01;
            let dy = (v - spine(u)) / (1.0 + slope(u).powi(2)).sqrt();
            let d = dy / r;
            if d.abs() < 1.0 {
                let nz = (1.0 - d * d).sqrt();
                let lam = (d * light.1 + nz * light.2 + 0.2).max(0.0);
                let su = u * 16.0 - t * 0.0;
                let sv = d * 3.0;
                let scale = ((fract(su + sv) - 0.5).abs() < 0.08) || ((fract(su - sv) - 0.5).abs() < 0.08);
                let mut l = lam * (0.55 + 0.45 * nz);
                if scale {
                    l *= 0.35;
                }
                let belly = d > 0.55;
                out = (l.min(1.0), if belly { TINT_HOT } else if l > 0.85 { TINT_HOT } else { TINT_ACCENT });
            }
        }
        // Head: wedge ellipse ahead of the spine.
        let (hu, hv) = (u - head - 0.12, v - spine(head));
        let hd = sd_ellipse(hu, hv, 0.24, 0.15);
        if hd < 0.0 {
            let k = smoothstep(0.0, -0.08, hd);
            out = (0.35 + 0.6 * k * (0.5 - hv * 2.0).clamp(0.2, 1.0), TINT_ACCENT);
            if len(hu - 0.07, hv + 0.06) < 0.035 {
                out = (1.0, TINT_GLITCH);
            }
        }
        out
    });
    // Forked tongue flicking.
    let flick = (t * 6.0).sin().max(0.0);
    if flick > 0.2 {
        let base = (head + 0.36, spine(head));
        let tip = (base.0 + 0.22 * flick, base.1 + 0.02 * (t * 20.0).sin());
        f.line(base, tip, 1.0, TINT_GLITCH);
        f.line(tip, (tip.0 + 0.06, tip.1 - 0.05), 1.0, TINT_GLITCH);
        f.line(tip, (tip.0 + 0.06, tip.1 + 0.05), 1.0, TINT_GLITCH);
    }
    // Shed-skin flakes peeling off the body.
    for i in 0..500usize {
        let life = fract(t * 0.35 + rndi(i, 5));
        let along = rndi(i, 6) * body_len;
        let u0 = head - along;
        let v0 = spine(u0);
        let u = u0 - life * 0.5 + 0.04 * (t * 3.0 + i as f32).sin();
        let v = v0 + (rndi(i, 7) - 0.5) * 0.35 + life * 0.6;
        f.plot(u, v, (1.0 - life) * 0.8, TINT_ACCENT);
    }
}

/// `WebEdit` -- OCULUS-HYDRATE: a giant living eye -- fibrous rotating iris,
/// breathing pupil, periodic blink -- caught in a rotating spider-web whose
/// strands run off every edge of the panel, data packets racing along them.
pub fn web_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let blink_ph = fract(t / 5.3);
    let lid = if blink_ph > 0.94 { ((blink_ph - 0.94) / 0.06 * PI).sin() } else { 0.0 };
    let open = 1.0 - lid;
    let pupil = 0.13 + 0.05 * (t * 1.3).sin();
    let look = (0.12 * (t * 0.6).sin(), 0.05 * (t * 0.9).cos());
    let spin = t * 0.15;
    let strands = 14;
    f.shade(|u, v, _, _| {
        let mut out = (0.0, TINT_ACCENT);
        let r = len(u, v);
        let th = v.atan2(u);
        // Web: radial strands + polygonal capture spiral.
        let sector = TAU / strands as f32;
        let local = (th - spin).rem_euclid(sector) - sector * 0.5;
        let strand_d = (local.sin() * r).abs();
        if strand_d < 0.008 && r > 0.6 {
            out = (0.42, TINT_ACCENT);
        }
        let poly_r = r * (local.cos());
        let ring_k = (poly_r.ln() * 7.0 + t * 0.6).rem_euclid(1.0);
        if (ring_k - 0.5).abs() < 0.035 && r > 0.65 {
            out = (0.5, TINT_ACCENT);
        }
        // Almond-shaped eye.
        let almond = 0.62 * open * (1.0 - (u / 1.25).powi(2)).max(0.0);
        if v.abs() < almond {
            let (iu, iv) = (u - look.0, v - look.1);
            let ir = len(iu, iv);
            let ia = iv.atan2(iu);
            let mut l;
            let mut tint = TINT_HOT;
            if ir < pupil {
                l = 0.0;
            } else if ir < 0.44 {
                let fib = 0.5 + 0.5 * (ia * 46.0 + noise3(ia * 5.0, ir * 9.0, t * 0.4) * 5.0 + t * 0.4).sin();
                let ring = 0.6 + 0.4 * ((ir - pupil) * 40.0).sin();
                l = (0.25 + 0.75 * fib) * ring * smoothstep(0.44, 0.38, ir);
                tint = TINT_ACCENT;
                if ir < pupil + 0.03 {
                    l = l.max(0.9);
                    tint = TINT_GLITCH;
                }
            } else {
                // Sclera shaded as a sphere, veins from noise.
                let edge = v.abs() / almond.max(1e-3);
                l = 0.85 - 0.6 * edge * edge;
                let vein = noise3(u * 9.0, v * 9.0, 2.0);
                if (vein - 0.5).abs() < 0.02 {
                    l = 0.6;
                    tint = TINT_GLITCH;
                }
            }
            if len(iu + 0.12, iv + 0.12) < 0.05 {
                l = 1.0;
                tint = TINT_HOT;
            }
            out = (l, tint);
        } else if (v.abs() - almond).abs() < 0.012 && u.abs() < 1.25 {
            out = (1.0, TINT_ACCENT);
        }
        out
    });
    // Data packets riding the strands.
    for i in 0..strands * 6 {
        let s = i % strands;
        let ang = spin + (s as f32 + 0.5) * TAU / strands as f32 - TAU / strands as f32 * 0.5;
        let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
        let k = fract(t * (0.25 + rndi(i, 3) * 0.3) * dir + rndi(i, 4));
        let r = 0.65 + k * (a + 0.4);
        let (cu, cv) = (ang.cos() * r, ang.sin() * r);
        f.glow(cu, cv, 1.5, 1.0, if dir > 0.0 { TINT_HOT } else { TINT_ICE });
        for tr in 1..10 {
            let rr = r - dir * tr as f32 * 0.03;
            f.plot(ang.cos() * rr, ang.sin() * rr, 0.8 - tr as f32 * 0.08, TINT_ACCENT);
        }
    }
}

/// `StyleEdit` -- CLASSICAL-BUST: the reference marble bust in true 1-bit
/// stipple, center stage, while a paint-front sweeps color across it; two
/// pixel-sorted ghost copies melt down the flanks; a rotating halftone
/// screen glimmers behind.
pub fn style_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let bh = 2.25;
    let bw = bh * (BUST_W as f32 / BUST_H as f32) * 0.98;
    let breathe = 1.0 + 0.015 * (t * 1.1).sin();
    let sweep = fract(t / 4.5) * 3.0 - 1.5;
    let flank = (a * 0.62).max(bw * 0.9);
    let seed = c.seed;
    f.shade(|u, v, x, _| {
        // Rotating halftone screen.
        let (ru, rv) = rot(u, v, t * 0.05);
        let cell = 0.09;
        let (cu, cv) = (fract(ru / cell) - 0.5, fract(rv / cell) - 0.5);
        let dotr = 0.12 + 0.18 * (0.5 + 0.5 * (len(u, v) * 3.0 - t * 1.5).sin());
        let mut out = if len(cu, cv) < dotr { (0.18, TINT_ICE) } else { (0.0, TINT_ACCENT) };
        // Center bust.
        let nx = (u / breathe) / bw + 0.5;
        let ny = (v / breathe + 0.12) / bh + 0.5;
        let s = smoothstep(0.18, 0.72, bust_sample(nx, ny));
        if s > 0.02 {
            let diag = u * 0.6 + v * 0.4;
            let painted = diag < sweep;
            let edge = (diag - sweep).abs() < 0.03;
            let tint = if edge { TINT_HOT } else if painted { TINT_ACCENT } else { TINT_HOT };
            out = (s, tint);
            if edge {
                out.0 = out.0.max(0.9);
            }
        }
        // Flank ghosts with pixel-sort drips.
        for side in [-1.0f32, 1.0] {
            let gx = (u - side * flank) / (bw * 0.82) + 0.5;
            if (0.0..1.0).contains(&gx) {
                let col = (x as u32).wrapping_mul(2_654_435_761) ^ seed;
                let drip = (rnd(col) * 0.6 * (0.5 + 0.5 * (t * 0.7 + rnd(col ^ 1) * 6.0).sin())).max(0.0);
                let gy = (v / (bh * 0.82)) + 0.5;
                let src_y = if rnd(col ^ 2) < 0.4 { gy - drip * gy } else { gy };
                let mut gxs = gx;
                if side > 0.0 {
                    gxs = 1.0 - gx;
                }
                let s = smoothstep(0.2, 0.75, bust_sample(gxs, src_y));
                if s > 0.05 {
                    let l = s * 0.6;
                    if l > out.0 {
                        out = (l, if side > 0.0 { TINT_GLITCH } else { TINT_ICE });
                    }
                }
            }
        }
        out
    });
    // Paint spray riding the sweep front.
    for i in 0..600usize {
        let k = rndi(i, 11) * 2.4 - 1.2;
        let life = fract(t * 1.3 + rndi(i, 12));
        let base_u = (sweep - 0.4 * k) / 0.6;
        let u = base_u + life * 0.45 * (rndi(i, 13) - 0.2);
        let v = k + (rndi(i, 14) - 0.5) * 0.1 + life * life * 0.3;
        f.plot(u, v, 1.0 - life, if i % 3 == 0 { TINT_HOT } else { TINT_ACCENT });
    }
}

/// `MarkupEdit` -- BRUTALIST-FACADE: an isometric megastructure assembling
/// itself -- concrete blocks drop from the sky and lock into a growing
/// stepped ziggurat, each face stippled by its angle to the light; a giant
/// `</>` sigil hovers overhead.
pub fn markup_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let s = 0.2; // iso cube edge in uv
    let origin = (0.0f32, 0.62f32);
    let iso = |gx: f32, gy: f32, gz: f32| -> (f32, f32) {
        (origin.0 + (gx - gy) * s * 0.866, origin.1 + (gx + gy) * s * 0.5 - gz * s)
    };
    // Build order: a stepped ziggurat, back-to-front so painter's order works.
    let mut cubes: Vec<(i32, i32, i32)> = Vec::new();
    for z in 0..4 {
        let r = 3 - z;
        for gx in -r..=r {
            for gy in -r..=r {
                if (gx + gy + z) % 3 != 0 || z == 0 || r <= 1 {
                    cubes.push((gx, gy, z));
                }
            }
        }
    }
    cubes.sort_by_key(|&(x, y, z)| (z, x + y));
    let cycle = 14.0;
    let ph = fract(t / cycle);
    let built = (ph * 1.25 * cubes.len() as f32) as usize;
    // Sky: searchlight beams and the `</>` sigil.
    let sig = "</>";
    let gpx = 0.07;
    let tw = text_width(sig, gpx);
    let sig_o = (a - tw - 0.25, -0.82 + 0.04 * (t * 1.4).sin());
    let sig_o2 = (-a + 0.25, -0.82 + 0.04 * (t * 1.4 + 1.0).cos());
    f.shade(|u, v, _, _| {
        let mut out = (0.0, TINT_ACCENT);
        for (k, bx) in [(-1.0f32, -a * 0.7), (1.0, a * 0.7)] {
            let ang = -PI * 0.5 + k * 0.5 * (t * 0.4 + k).sin();
            let (du, dv) = (u - bx, v - 1.0);
            let pa = dv.atan2(du);
            let d = (pa - ang).abs();
            if d < 0.06 && dv < 0.0 {
                out = (0.22 * (1.0 - d / 0.06) + 0.05, TINT_ICE);
            }
        }
        let g = text_sample(sig, u, v, sig_o, gpx).max(text_sample(sig, u, v, sig_o2, gpx));
        if g > 0.05 {
            let scan = 0.6 + 0.4 * ((v * 40.0 - t * 6.0).sin());
            out = (g * scan, TINT_GLITCH);
        }
        // Ground fog.
        if v > 0.75 {
            let n = fbm3(u * 2.0 + t * 0.2, v * 4.0, t * 0.1, 3);
            out.0 = out.0.max((n - 0.35) * (v - 0.75) * 3.0);
        }
        out
    });
    let light = [0.92f32, 0.55, 0.22]; // top, left, right faces
    for (idx, &(gx, gy, gz)) in cubes.iter().enumerate() {
        if idx > built {
            break;
        }
        let drop = if idx + 6 > built { ((idx + 6 - built) as f32 / 6.0).powi(2) * 6.0 } else { 0.0 };
        let (x, y, z) = (gx as f32, gy as f32, gz as f32 + drop);
        let top = [iso(x, y, z + 1.0), iso(x + 1.0, y, z + 1.0), iso(x + 1.0, y + 1.0, z + 1.0), iso(x, y + 1.0, z + 1.0)];
        let left = [iso(x, y + 1.0, z + 1.0), iso(x + 1.0, y + 1.0, z + 1.0), iso(x + 1.0, y + 1.0, z), iso(x, y + 1.0, z)];
        let right = [iso(x + 1.0, y, z + 1.0), iso(x + 1.0, y + 1.0, z + 1.0), iso(x + 1.0, y + 1.0, z), iso(x + 1.0, y, z)];
        let window = hash((idx as u32) ^ ((t * 2.0) as u32 / 3)) % 5 == 0;
        for (fi, quad) in [top, left, right].iter().enumerate() {
            fill_quad(f, quad, light[fi], if fi == 2 { TINT_ICE } else { TINT_ACCENT }, fi > 0 && window);
        }
    }
}

/// Fills a convex quad with luminance `l`; edges drawn brighter. With
/// `lit_window`, a glowing slot appears in the face center.
pub(super) fn fill_quad(f: &mut DotField, q: &[(f32, f32); 4], l: f32, tint: u8, lit_window: bool) {
    let pts: Vec<(f32, f32)> = q.iter().map(|p| f.to_dot(p.0, p.1)).collect();
    let minx = pts.iter().map(|p| p.0).fold(f32::MAX, f32::min).floor().max(0.0) as i32;
    let maxx = pts.iter().map(|p| p.0).fold(f32::MIN, f32::max).ceil().min(f.w as f32) as i32;
    let miny = pts.iter().map(|p| p.1).fold(f32::MAX, f32::min).floor().max(0.0) as i32;
    let maxy = pts.iter().map(|p| p.1).fold(f32::MIN, f32::max).ceil().min(f.h as f32) as i32;
    let (cx, cy) = (pts.iter().map(|p| p.0).sum::<f32>() / 4.0, pts.iter().map(|p| p.1).sum::<f32>() / 4.0);
    for y in miny..maxy {
        for x in minx..maxx {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut inside = true;
            let mut sign = 0.0f32;
            let mut min_edge = f32::MAX;
            for i in 0..4 {
                let (ax, ay) = pts[i];
                let (bx, by) = pts[(i + 1) % 4];
                let cr = (bx - ax) * (py - ay) - (by - ay) * (px - ax);
                if cr.abs() > 1e-6 {
                    if sign == 0.0 {
                        sign = cr.signum();
                    } else if cr.signum() != sign {
                        inside = false;
                        break;
                    }
                }
                let el = ((bx - ax).powi(2) + (by - ay).powi(2)).sqrt().max(1e-3);
                min_edge = min_edge.min(cr.abs() / el);
            }
            if !inside {
                continue;
            }
            let mut lv = l;
            let mut tn = tint;
            if min_edge < 1.0 {
                lv = 1.0;
                tn = TINT_HOT;
            } else if lit_window && (px - cx).abs() < 1.5 && (py - cy).abs() < 1.5 {
                lv = 1.0;
                tn = TINT_GLITCH;
            }
            // Painter's order: later quads overwrite earlier ones.
            f.erase(x, y);
            f.dot(x, y, lv, tn);
        }
    }
}

fn gear_height(u: f32, v: f32, r: f32, teeth: f32, ang: f32) -> f32 {
    let d = len(u, v);
    let th = v.atan2(u) - ang;
    let tooth = smoothstep(-0.3, 0.3, (th * teeth).cos());
    let outer = r + 0.07 * tooth;
    if d > outer {
        return 0.0;
    }
    let mut h = smoothstep(outer, outer - 0.04, d) * 0.6 + 0.4;
    if (d - r * 0.72).abs() < 0.03 {
        h -= 0.3;
    }
    let spokes = (th * 5.0).cos();
    if d < r * 0.66 && d > r * 0.28 && spokes < 0.3 {
        h = 0.0;
    }
    if d < r * 0.12 {
        h = 0.0;
    }
    if d < r * 0.22 && d >= r * 0.12 {
        h = 1.0;
    }
    h
}

/// `ConfigEdit` -- VERNIER-CALIPER: a train of meshing, sculpted gears that
/// actually counter-rotate at the right ratios, a vernier ruler sliding
/// along both edges, sparks spitting from every mesh point.
pub fn config_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    // Gear train laid out to span the panel: (cx, cy, radius, teeth).
    let mut gears: Vec<(f32, f32, f32, f32, f32)> = Vec::new();
    let radii = [0.62f32, 0.4, 0.55, 0.32, 0.5, 0.42];
    let mut x = -a + 0.5;
    let mut ang_prev = t * 0.6;
    let mut prev_r = 0.0;
    let mut prev_teeth = 0.0;
    for (k, &r) in radii.iter().enumerate() {
        let teeth = (r * 30.0).round();
        if k > 0 {
            x += prev_r + r + 0.06;
        }
        if x - r > a + 0.2 {
            break;
        }
        let cy = if k % 2 == 0 { 0.08 } else { -0.18 };
        let ang = if k == 0 { ang_prev } else { -ang_prev * prev_teeth / teeth + PI / teeth };
        gears.push((x, cy, r, teeth, ang));
        ang_prev = ang;
        prev_r = r;
        prev_teeth = teeth;
    }
    let light = norm3(((t * 0.4).cos() * 0.6, -0.6, 0.7));
    let px = f.px();
    f.shade(|u, v, _, _| {
        let mut out = (0.0, TINT_ACCENT);
        // Vernier rulers top and bottom.
        for (base, dir, off) in [(-0.96f32, 1.0f32, t * 0.2), (0.96, -1.0, -t * 0.27)] {
            let dv = (v - base) * dir;
            if (0.0..0.08).contains(&dv) {
                let m = fract((u + off) * 10.0);
                let major = fract((u + off) * 2.0) < 0.02;
                if m < px * 10.0 * 1.2 && dv < if major { 0.08 } else { 0.04 } {
                    out = (if major { 0.95 } else { 0.55 }, if major { TINT_HOT } else { TINT_ACCENT });
                }
            }
        }
        for &(cx, cy, r, teeth, ang) in &gears {
            if len(u - cx, v - cy) > r + 0.1 {
                continue;
            }
            let (h, lam) = relief(&|p, q| gear_height(p, q, r, teeth, ang), u - cx, v - cy, 0.012, 1.5, light);
            if h > 0.0 {
                let l = (0.15 + 0.85 * lam) * (0.4 + 0.6 * h);
                out = (l, if l > 0.85 { TINT_HOT } else { TINT_ACCENT });
            }
        }
        out
    });
    // Sparks at each mesh point.
    for w in gears.windows(2) {
        let (ax, ay, ar, ..) = w[0];
        let (bx, by, ..) = w[1];
        let d = len(bx - ax, by - ay);
        let (mx, my) = (ax + (bx - ax) / d * ar, ay + (by - ay) / d * ar);
        for i in 0..90usize {
            let life = fract(t * 1.4 + rndi(i, (mx * 100.0) as u32));
            let ang = rndi(i, 21) * TAU;
            let sp = 0.3 + rndi(i, 22) * 0.6;
            let u = mx + ang.cos() * sp * life;
            let v = my + ang.sin() * sp * life + life * life * 0.6;
            f.plot(u, v, 1.0 - life, if life < 0.3 { TINT_HOT } else { TINT_GLITCH });
        }
    }
}

/// `DocsEdit` -- VEILED-SCRIBE: an invisible hand writes endless cursive
/// across ruled parchment; the quill leaves a live ink trail, old lines
/// lift off the page as drifting dust.
pub fn docs_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let lines = 5usize;
    let line_gap = 1.6 / lines as f32;
    let line_len = 2.0 * a - 0.4;
    let speed = 0.55;
    let total = t * speed + line_len * 2.4;
    let page_len = line_len * lines as f32;
    let page_pos = total % page_len;
    let cur_line = (page_pos / line_len) as usize;
    let px = f.px();
    let line_y = |k: usize| -0.75 + k as f32 * line_gap + 0.12;
    let seed = c.seed;
    f.shade(|u, v, x, _| {
        for k in 0..lines {
            if (v - line_y(k) - 0.11).abs() < px * 0.5 && x % 3 != 0 {
                return (0.22, TINT_ACCENT);
            }
        }
        // Margin rule and parchment grain.
        if (u + a - 0.12).abs() < px * 0.6 {
            return (0.45, TINT_GLITCH);
        }
        let g = fbm3(u * 3.0, v * 3.0, seed as f32 * 0.001, 3);
        if g > 0.62 {
            return ((g - 0.62) * 1.2, TINT_ACCENT);
        }
        (0.0, TINT_ACCENT)
    });
    let path = |s: f32, k: usize| -> (f32, f32) {
        let u = -a + 0.2 + s + 0.035 * (s * 31.0).sin();
        let word = (s * 3.1).sin().abs();
        let v = line_y(k)
            + 0.07 * (s * 23.0).sin() * (s * 4.7).cos()
            + 0.03 * (s * 61.0 + k as f32).sin() * word;
        (u, v)
    };
    let mut pen = (0.0, 0.0);
    for k in 0..=cur_line.min(lines - 1) {
        let end = if k < cur_line { line_len } else { page_pos - cur_line as f32 * line_len };
        let n = (end * f.w as f32 / (2.0 * a) * 2.0) as usize;
        for i in 0..n {
            let s = i as f32 / n.max(1) as f32 * end;
            // Word gaps.
            if fract(s * 1.7 + k as f32 * 0.3) > 0.86 {
                continue;
            }
            let p = path(s, k);
            let age = end - s;
            let fresh = k == cur_line && age < 0.25;
            let fade = if k + 2 < cur_line { 0.35 } else { 0.85 };
            let tint = if fresh { TINT_HOT } else { TINT_ACCENT };
            f.plot(p.0, p.1, if fresh { 1.0 } else { fade }, tint);
            f.plot(p.0, p.1 + px, if fresh { 0.9 } else { fade * 0.8 }, tint);
        }
        pen = path(end, k);
    }
    // Quill: shaft and barbs, angled up-right from the nib.
    let tip = pen;
    let back = (tip.0 + 0.7, tip.1 - 0.95);
    f.line(tip, back, 1.0, TINT_HOT);
    for i in 0..26 {
        let k = 0.25 + i as f32 / 26.0 * 0.75;
        let p = (mix(tip.0, back.0, k), mix(tip.1, back.1, k));
        let w = 0.2 * (k * PI).sin() * (1.0 + 0.1 * (t * 5.0 + i as f32).sin());
        f.line(p, (p.0 + w * 0.8, p.1 + w * 0.6), 0.6, TINT_ACCENT);
        f.line(p, (p.0 - w * 0.8, p.1 - w * 0.4), 0.6, TINT_ACCENT);
    }
    // Ink spatter and dust lifting from older lines.
    for i in 0..650usize {
        let k = (rndi(i, 31) * lines as f32) as usize;
        if k > cur_line {
            continue;
        }
        let life = fract(t * 0.25 + rndi(i, 32));
        let s = rndi(i, 33) * line_len;
        let p = path(s, k);
        let u = p.0 + (life * 2.0 + i as f32).sin() * 0.08 * life;
        let v = p.1 - life * 0.9;
        f.plot(u, v, (1.0 - life) * 0.75, if i % 4 == 0 { TINT_GLITCH } else { TINT_ACCENT });
    }
}

/// `ShellScriptEdit` -- OCCULT-SIGIL: digital rain pours through a huge
/// counter-rotating occult sigil -- rune ring, hexagram, pentagram -- with a
/// breathing `$_` prompt burning at its heart.
pub fn shell_script_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let seed = c.seed;
    let h = f.h as f32;
    f.shade(|_, _, x, y| {
        let col = hash(x as u32 ^ seed);
        if col % 2 != 0 {
            return (0.0, TINT_ACCENT);
        }
        let speed = 10.0 + rnd(col) * 30.0;
        let len_t = 8.0 + rnd(col ^ 1) * 26.0;
        let head = fract(t * speed / (h + len_t) + rnd(col ^ 2)) * (h + len_t);
        let d = head - y as f32;
        if (0.0..len_t).contains(&d) {
            let flick = if hash(col ^ y as u32 ^ (t * 8.0) as u32) % 7 == 0 { 0.0 } else { 1.0 };
            let l = (1.0 - d / len_t).powf(1.5) * 0.75 * flick;
            (l, if d < 1.5 { TINT_HOT } else { TINT_ACCENT })
        } else {
            (0.0, TINT_ACCENT)
        }
    });
    let pulse = 0.85 + 0.15 * (t * 2.2).sin();
    let r0 = 0.92 * pulse;
    // Clear a void behind the sigil.
    let (cx, cy) = f.to_dot(0.0, 0.0);
    let rr = r0 * f.h as f32 * 0.5;
    for y in 0..f.h as i32 {
        for x in 0..f.w as i32 {
            if ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt() < rr {
                f.erase(x, y);
            }
        }
    }
    let px = f.px();
    f.ring((0.0, 0.0), r0, 1.0, TINT_HOT);
    f.ring((0.0, 0.0), r0 + px, 1.0, TINT_HOT);
    f.ring((0.0, 0.0), r0 + 0.06, 0.6, TINT_ACCENT);
    f.ring((0.0, 0.0), r0 * 0.86, 0.8, TINT_ACCENT);
    f.ring((0.0, 0.0), r0 * 0.5, 0.7, TINT_ACCENT);
    // Rune ring: glyph fragments orbiting between the two outer circles.
    let runes = 24;
    for k in 0..runes {
        let ang = k as f32 / runes as f32 * TAU + t * 0.35;
        let hsh = hash(k as u32 ^ seed);
        let rm = r0 * 0.93;
        let (ca, sa) = (ang.cos(), ang.sin());
        let p = |dr: f32, dt: f32| (ca * (rm + dr) - sa * dt, sa * (rm + dr) + ca * dt);
        let sz = 0.035;
        f.line(p(-sz, 0.0), p(sz, 0.0), 0.9, TINT_ICE);
        if hsh & 1 == 1 {
            f.line(p(sz, -sz * 0.7), p(sz, sz * 0.7), 0.9, TINT_ICE);
        }
        if hsh & 2 == 2 {
            f.line(p(-sz, -sz * 0.7), p(0.0, sz * 0.7), 0.9, TINT_ICE);
        }
        if hsh & 4 == 4 {
            f.plot(p(0.0, -sz).0, p(0.0, -sz).1, 1.0, TINT_HOT);
        }
    }
    // Hexagram (counter-rotating) and pentagram.
    let star = |f: &mut DotField, n: usize, step: usize, r: f32, rot0: f32, l: f32, tint: u8| {
        for k in 0..n {
            let a0 = rot0 + k as f32 / n as f32 * TAU;
            let a1 = rot0 + ((k + step) % n) as f32 / n as f32 * TAU;
            f.line((a0.cos() * r, a0.sin() * r), (a1.cos() * r, a1.sin() * r), l, tint);
            f.line((a0.cos() * r, a0.sin() * r + f.px()), (a1.cos() * r, a1.sin() * r + f.px()), l, tint);
        }
    };
    star(f, 6, 2, r0 * 0.86, -t * 0.25, 0.85, TINT_ACCENT);
    star(f, 5, 2, r0 * 0.5, t * 0.5 - PI * 0.5, 1.0, TINT_GLITCH);
    // `$_` prompt with blinking cursor.
    let txt = if fract(t * 1.6) < 0.5 { "$_" } else { "$ " };
    let gpx = 0.06 * pulse;
    let tw = text_width("$_", gpx);
    let o = (-tw * 0.5, -3.5 * gpx);
    let (x0, y0) = f.to_dot(o.0, o.1);
    let (x1, y1) = f.to_dot(o.0 + tw, o.1 + 7.0 * gpx);
    for y in y0 as i32..y1 as i32 + 1 {
        for x in x0 as i32..x1 as i32 + 1 {
            let (u, v) = f.uv(x.max(0) as usize, y.max(0) as usize);
            let g = text_sample(txt, u, v, o, gpx);
            if g > 0.1 {
                f.dot(x, y, g, TINT_HOT);
            }
        }
    }
}
