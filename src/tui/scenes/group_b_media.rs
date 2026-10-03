//! Group B -- Media & Assets. Full-bleed micro-dot shaders; see `dots.rs`.

use super::dots::*;
use super::Ctx;

/// `ImageAsset` -- DITHERED-CUMULUS: domain-warped storm clouds roll across
/// the whole panel, sun-lit from one side, while paint bombs burst open in
/// the sky and fling droplets outward.
pub fn image_asset(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let seed = c.seed;
    // Up to 5 live splats, each on its own 2.6s cycle.
    let splats: Vec<(f32, f32, f32, u8, u32)> = (0..5u32)
        .map(|k| {
            let period = 2.6;
            let tt = t + k as f32 * period / 5.0;
            let slot = (tt / period).floor() as u32;
            let ph = fract(tt / period);
            let s = hash(seed ^ k.wrapping_mul(977) ^ slot.wrapping_mul(7919));
            let cu = (rnd(s) * 2.0 - 1.0) * (a - 0.4);
            let cv = (rnd(s ^ 1) * 2.0 - 1.0) * 0.7;
            let tint = [TINT_ACCENT, TINT_GLITCH, TINT_ICE, TINT_HOT][(s % 4) as usize];
            (cu, cv, ph, tint, s)
        })
        .collect();
    f.shade(|u, v, _, _| {
        let q = (u * 0.9 - t * 0.12, v * 0.9);
        let w1 = fbm3(q.0, q.1, t * 0.05, 2);
        let w2 = fbm3(q.0 + 5.2, q.1 + 1.3, t * 0.05, 2);
        let n = fbm3(q.0 + 1.8 * w1, q.1 + 1.8 * w2, t * 0.07, 3);
        let n2 = fbm3(q.0 + 1.8 * w1 + 0.06, q.1 + 1.8 * w2 - 0.06, t * 0.07, 3);
        let dens = smoothstep(0.42, 0.75, n);
        let lit = clamp01(0.55 + (n - n2) * 9.0);
        let mut out = (dens * (0.25 + 0.75 * lit), if lit > 0.8 && dens > 0.6 { TINT_HOT } else { TINT_ACCENT });
        for &(cu, cv, ph, tint, s) in &splats {
            let grow = smoothstep(0.0, 0.12, ph);
            let fade = 1.0 - smoothstep(0.55, 1.0, ph);
            let (du, dv) = (u - cu, v - cv);
            if du.abs() > 0.4 || dv < -0.4 || dv > 1.0 {
                continue;
            }
            let r = len(du, dv);
            let ang = dv.atan2(du);
            let edge = 0.28 * grow * (1.0 + 0.35 * noise3(ang.cos() * 3.0 + rnd(s) * 9.0, ang.sin() * 3.0, 1.0));
            if r < edge {
                out = (0.85 * fade + 0.1, tint);
            }
            // Drips run down from the splat.
            let col = (du * 23.0).floor();
            let dl = rnd(s ^ col as i32 as u32) * 0.5 * smoothstep(0.1, 0.6, ph);
            if du.abs() < edge * 0.8 && dv > 0.0 && dv < edge + dl && (fract(du * 23.0) - 0.5).abs() < 0.25 && dl > 0.15 {
                out = (0.7 * fade, tint);
            }
        }
        out
    });
    for &(cu, cv, ph, tint, s) in &splats {
        if ph > 0.6 {
            continue;
        }
        for i in 0..120usize {
            let ang = rndi(i, s) * TAU;
            let sp = 0.3 + rndi(i, s ^ 3) * 1.1;
            let k = ph / 0.6;
            let u = cu + ang.cos() * sp * k;
            let v = cv + ang.sin() * sp * k + k * k * 0.5;
            f.glow(u, v, 1.0, 1.0 - k, tint);
        }
    }
}

/// `AudioAsset` -- HARMONIC-RIBBON: five twisting 3D sound ribbons weave
/// through each other across the full width, a mirrored spectrum analyzer
/// pulses beneath, and a circular oscilloscope blooms in the center.
pub fn audio_asset(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let ribbons: [(f32, f32, f32, f32, u8); 5] = [
        (0.42, 2.1, 1.7, 0.0, TINT_ACCENT),
        (0.3, 3.3, -2.3, 1.1, TINT_ICE),
        (0.22, 5.1, 3.1, 2.3, TINT_HOT),
        (0.36, 1.4, -1.2, 3.7, TINT_GLITCH),
        (0.18, 7.3, 4.2, 5.1, TINT_ACCENT),
    ];
    let bars = 64usize;
    let spectrum: Vec<f32> = (0..bars)
        .map(|b| {
            let k = b as f32 / bars as f32;
            let e = 0.5 + 0.5 * (t * (3.0 + k * 9.0) + k * 17.0).sin() * (t * 1.3 + k * 5.0).cos();
            clamp01(e * (1.0 - k * 0.5) + 0.15 * noise3(k * 8.0, t * 4.0, 0.0))
        })
        .collect();
    let px = f.px();
    f.shade(|u, v, _, _| {
        let mut out = (0.0, TINT_ACCENT);
        // Spectrum: mirrored bars hanging off the bottom edge and top edge.
        let k = clamp01((u + a) / (2.0 * a));
        let b = ((k * bars as f32) as usize).min(bars - 1);
        let cellk = fract(k * bars as f32);
        if cellk > 0.18 && cellk < 0.82 {
            let hgt = spectrum[b] * 0.45;
            let db = 1.0 - v;
            let dt = v + 1.0;
            if db < hgt || dt < hgt * 0.6 {
                let seg = fract((db.min(dt)) * 22.0) < 0.6;
                if seg {
                    out = (0.35 + 0.4 * spectrum[b], TINT_ACCENT);
                }
            }
        }
        // Ribbons: band of twisting width around a sine path.
        for &(amp, freq, spd, ph, tint) in &ribbons {
            let env = 0.6 + 0.4 * (u * 0.7 + t * 0.3 + ph).sin();
            let y = amp * env * (u * freq + t * spd + ph).sin();
            let twist = (u * freq * 0.5 - t * spd * 0.7 + ph).cos();
            let wdt = 0.012 + 0.07 * twist.abs();
            let d = (v - y).abs();
            if d < wdt {
                let face = twist.abs();
                let edge = d > wdt - px * 1.2;
                let l = if edge { 1.0 } else { 0.25 + 0.6 * face };
                if l > out.0 {
                    out = (l, tint);
                }
            }
        }
        out
    });
    // Circular oscilloscope bloom.
    let n = 720;
    for i in 0..n {
        let ang = i as f32 / n as f32 * TAU;
        let wave = (ang * 6.0 + t * 4.0).sin() * 0.06 + (ang * 13.0 - t * 7.0).sin() * 0.03;
        let r = 0.33 + wave * (0.6 + 0.4 * spectrum[(i * bars / n).min(bars - 1)]);
        f.plot(ang.cos() * r, ang.sin() * r, 1.0, TINT_HOT);
        f.plot(ang.cos() * r * 0.8, ang.sin() * r * 0.8, 0.5, TINT_GLITCH);
    }
}

/// `VideoAsset` -- SPROCKET-FRAME: two film strips cross the panel at
/// opposing angles, sprockets racing; each frame holds a stippled sphere
/// whose position advances frame-to-frame -- a real flipbook -- under a
/// dusty projector cone.
pub fn video_asset(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let strips = [(0.26f32, 0.9f32, -0.25f32), (-0.22, -1.2, 0.35)];
    let light = norm3((-0.5, -0.6, 0.6));
    f.shade(|u, v, _, _| {
        let mut out = (0.0, TINT_ACCENT);
        // Projector cone from the left edge with drifting dust.
        let (pu, pv) = (u + a, v + 0.1);
        if pu > 0.0 {
            let spread = (pv / pu).abs();
            if spread < 0.32 {
                let dust = noise3(u * 14.0 - t * 2.0, v * 14.0, t);
                let beam = (1.0 - spread / 0.32) * 0.18;
                out = (beam + if dust > 0.78 { 0.6 } else { 0.0 }, TINT_ICE);
            }
        }
        for (si, &(ang, speed, off)) in strips.iter().enumerate() {
            let (su, sv) = rot(u, v - off, -ang);
            let half = 0.36;
            if sv.abs() > half {
                continue;
            }
            let scroll = su + t * speed;
            let frame_w = 0.62;
            let fi = (scroll / frame_w).floor();
            let fx = fract(scroll / frame_w) * frame_w - frame_w * 0.5;
            let mut l;
            let mut tint = TINT_ACCENT;
            // Sprocket holes along both edges.
            if sv.abs() > half - 0.1 {
                let hole = (fract(scroll / 0.11) - 0.5).abs() < 0.22 && (sv.abs() - (half - 0.05)).abs() < 0.028;
                l = if hole { 0.0 } else { 0.85 };
                tint = if si == 0 { TINT_ACCENT } else { TINT_GLITCH };
            } else {
                // Frame gate; sphere inside, animated by frame index.
                if fx.abs() > frame_w * 0.5 - 0.025 {
                    l = if fx.abs() > frame_w * 0.5 - 0.008 { 0.9 } else { 0.0 };
                } else {
                    l = 0.06;
                    let k = fi * 0.35;
                    let bx = 0.12 * (k * 2.0).sin();
                    let by = -0.12 + 0.18 * (k * 3.0).sin().abs() * -1.0 + 0.12;
                    let r = 0.13;
                    let (du, dv) = (fx - bx, sv - by);
                    let d = len(du, dv);
                    if d < r {
                        let nz = (1.0 - (d / r).powi(2)).sqrt();
                        let lam = clamp01((du / r) * light.0 + (dv / r) * light.1 + nz * light.2);
                        l = 0.12 + 0.88 * lam;
                        tint = if lam > 0.85 { TINT_HOT } else if si == 0 { TINT_ACCENT } else { TINT_GLITCH };
                    }
                    // Floor shadow.
                    if (sv - 0.22).abs() < 0.015 && (fx - bx).abs() < 0.12 {
                        l = 0.5;
                    }
                }
            }
            out = (l, tint);
        }
        out
    });
}

/// `FontAsset` -- BASKERVILLE-LIGATURE: a monumental glyph, carved and lit
/// like stone, morphs through the alphabet by exploding into dots and
/// re-condensing; type-design guides and marquee alphabets stream past.
pub fn font_asset(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let seq = ['A', 'G', '&', 'R', '@', 'Q', '$', 'K', 'S'];
    let period = 2.8;
    let idx = (t / period) as usize;
    let ph = fract(t / period);
    let cur = seq[(idx + c.seed as usize) % seq.len()];
    let next = seq[(idx + 1 + c.seed as usize) % seq.len()];
    let gpx = 1.75 / 7.0;
    let gw = 5.0 * gpx;
    let o = (-gw * 0.5, -0.875);
    let morph = smoothstep(0.75, 1.0, ph);
    let scatter = (morph * PI).sin();
    let light = norm3(((t * 0.9).cos(), (t * 0.9).sin() * 0.5 - 0.3, 0.8));
    let height = |ch: char, u: f32, v: f32| -> f32 {
        let gx = (u - o.0) / gpx;
        let gy = (v - o.1) / gpx;
        let mut s = 0.0;
        for (dx, dy) in [(0.0, 0.0), (0.35, 0.0), (-0.35, 0.0), (0.0, 0.35), (0.0, -0.35)] {
            s += glyph_sample(ch, gx + dx, gy + dy);
        }
        s / 5.0
    };
    let marquee = "ABCDEFGHIJKLMNOPQRSTUVWXYZ 0123456789 &@$#?! ";
    let mpx = 2.0 * f.px();
    let mw = text_width(marquee, mpx) + 6.0 * mpx;
    let px = f.px();
    f.shade(|u, v, x, y| {
        let mut out = (0.0, TINT_ACCENT);
        // Guides: cap height, x-height, baseline (dashed).
        for (gv, l) in [(o.1, 0.5), (o.1 + 2.0 * gpx, 0.3), (o.1 + 7.0 * gpx, 0.6)] {
            if (v - gv).abs() < px * 0.5 && (x / 2) % 3 != 0 {
                out = (l, TINT_ICE);
            }
        }
        // Marquees top and bottom, scrolling in opposite directions.
        for (mv, dir) in [(-0.98f32, 1.0f32), (0.98 - 7.0 * mpx, -1.0)] {
            let off = (t * 0.25 * dir).rem_euclid(mw);
            let uu = (u + a + off).rem_euclid(mw) - a;
            let g = text_sample(marquee, uu, v, (-a, mv), mpx);
            if g > 0.5 {
                out = (0.55, TINT_ACCENT);
            }
        }
        // Scatter displacement during the morph.
        let j = scatter * 0.35;
        let (ju, jv) = if j > 0.0 {
            let hs = hash((x as u32).wrapping_mul(7919) ^ (y as u32).wrapping_mul(104_729));
            ((rnd(hs) - 0.5) * j, (rnd(hs ^ 1) - 0.5) * j)
        } else {
            (0.0, 0.0)
        };
        let (su, sv) = (u + ju, v + jv);
        if su < o.0 - 0.3 || su > o.0 + gw + 0.3 || sv < o.1 - 0.3 || sv > o.1 + 7.0 * gpx + 0.3 {
            return out;
        }
        let hc = height(cur, su, sv) * (1.0 - morph) + height(next, su, sv) * morph;
        if hc > 0.02 {
            let e = 0.02;
            let hx = height(cur, su + e, sv) * (1.0 - morph) + height(next, su + e, sv) * morph;
            let hy = height(cur, su, sv + e) * (1.0 - morph) + height(next, su, sv + e) * morph;
            let n = norm3(((hc - hx) / e * 0.25, (hc - hy) / e * 0.25, 1.0));
            let lam = (n.0 * light.0 + n.1 * light.1 + n.2 * light.2).max(0.0);
            let l = (0.15 + 0.85 * lam) * smoothstep(0.02, 0.5, hc);
            out = (l, if lam > 0.9 { TINT_HOT } else if scatter > 0.2 { TINT_GLITCH } else { TINT_ACCENT });
        }
        out
    });
    // Bezier control handles orbiting the glyph corners.
    for k in 0..6 {
        let ang = t * 0.8 + k as f32 * TAU / 6.0;
        let (hu, hv) = (ang.cos() * (gw * 0.9), ang.sin() * 0.75);
        let (pu, pv) = (hu * 0.6, hv * 0.6);
        f.line((pu, pv), (hu, hv), 0.5, TINT_ICE);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let (x, y) = f.to_dot(hu, hv);
                if dx != 0 || dy != 0 {
                    f.dot(x as i32 + dx * 2, y as i32 + dy * 2, 1.0, TINT_HOT);
                }
            }
        }
    }
}

/// `NotebookActivity` -- TURING-MORPH: a petri dish of living Turing
/// labyrinth slowly rewiring itself, on graph paper where a lab plot draws
/// itself and scatter samples blink in.
pub fn notebook_activity(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let seed = c.seed;
    let dish_c = (-a * 0.38, 0.0);
    let dish_r = 0.9;
    let plot_x0 = dish_c.0 + dish_r + 0.25;
    let plot_x1 = a - 0.1;
    let draw = fract(t / 6.0);
    let px = f.px();
    f.shade(|u, v, x, y| {
        let mut out = (0.0, TINT_ACCENT);
        // Graph paper.
        let g = (f32::max(1.0, 0.1 / px)) as usize;
        if (x % g == 0 || y % g == 0) && (x / g + y / g) % 2 == 0 {
            out = (0.16, TINT_ACCENT);
        }
        let (du, dv) = (u - dish_c.0, v - dish_c.1);
        let r = len(du, dv);
        if r < dish_r {
            let w = fbm3(du * 1.6, dv * 1.6, t * 0.12, 3) * 4.0;
            let mut s = 0.0;
            for k in 0..4 {
                let ang = k as f32 * PI / 4.0 + 0.3;
                s += ((du * ang.cos() + dv * ang.sin()) * 17.0 + w + t * 0.3 * (k as f32 - 1.5)).sin();
            }
            let l = smoothstep(-0.25, 0.25, s);
            let vign = smoothstep(dish_r, dish_r * 0.7, r);
            out = (l * (0.35 + 0.65 * vign), if l > 0.9 && vign > 0.95 { TINT_HOT } else { TINT_ACCENT });
        }
        if (r - dish_r).abs() < px * 1.2 || (r - dish_r - 0.035).abs() < px * 0.6 {
            out = (1.0, TINT_HOT);
        }
        // Plot axes.
        if u > plot_x0 && u < plot_x1 {
            if (v - 0.7).abs() < px * 0.6 {
                out = (0.7, TINT_ICE);
            }
        }
        if (u - plot_x0).abs() < px * 0.6 && v > -0.8 && v < 0.7 {
            out = (0.7, TINT_ICE);
        }
        out
    });
    // The plot line draws itself left to right.
    let n = 500;
    let span = plot_x1 - plot_x0;
    for i in 0..n {
        let k = i as f32 / n as f32;
        if k > draw {
            break;
        }
        let u = plot_x0 + k * span;
        let v = 0.6 - 1.2 * (1.0 - (-k * 3.0).exp()) * (0.85 + 0.15 * (k * 30.0 + seed as f32).sin());
        f.plot(u, v, 1.0, TINT_HOT);
        if i % 25 == 0 {
            f.line((u, v), (u, 0.7), 0.35, TINT_ACCENT);
        }
    }
    // Scatter samples blinking in.
    for i in 0..220usize {
        let k = rndi(i, seed);
        let u = plot_x0 + k * span;
        let v = 0.6 - 1.2 * (1.0 - (-k * 3.0).exp()) + (rndi(i, 3) - 0.5) * 0.35;
        let blink = fract(t * 0.7 + rndi(i, 4));
        if blink < 0.7 {
            f.glow(u, v, 1.0, 1.0 - blink, TINT_GLITCH);
        }
    }
}
