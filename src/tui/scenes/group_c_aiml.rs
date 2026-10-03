//! Group C -- AI / ML. Full-bleed micro-dot shaders; see `dots.rs`.

use super::dots::*;
use super::Ctx;

/// `ModelTrainingCheckpoint` -- ISOMETRIC-TESSERACT: a 4D hypercube rotating
/// through two planes at once, current flowing along all 32 edges, while a
/// galaxy of weights spirals in from every edge of the panel to be flushed
/// into it. A loss curve settles along the floor.
pub fn model_training_checkpoint(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let (xw, yz, xy) = (t * 0.45, t * 0.31, t * 0.2);
    let verts: Vec<[f32; 4]> = (0..16)
        .map(|i| {
            let p = [
                if i & 1 == 0 { -1.0 } else { 1.0 },
                if i & 2 == 0 { -1.0 } else { 1.0 },
                if i & 4 == 0 { -1.0 } else { 1.0 },
                if i & 8 == 0 { -1.0 } else { 1.0 },
            ];
            let (x, w) = rot(p[0], p[3], xw);
            let (y, z) = rot(p[1], p[2], yz);
            let (x, y) = rot(x, y, xy);
            [x, y, z, w]
        })
        .collect();
    let proj: Vec<(f32, f32, f32)> = verts
        .iter()
        .map(|p| {
            let k4 = 2.6 / (2.6 + p[3]);
            let (x, y, z) = (p[0] * k4, p[1] * k4, p[2] * k4);
            project(x, y, z, 4.0, 0.3)
        })
        .collect();
    // Background: faint loss curve and weight-spiral field.
    let px = f.px();
    f.shade(|u, v, _, _| {
        let k = clamp01((u + a) / (2.0 * a));
        let loss = 0.95 - 0.55 * (1.0 - (-k * 4.0).exp()) - 0.03 * (k * 60.0 + t * 3.0).sin() * (1.0 - k);
        if (v - loss).abs() < px * 0.8 {
            return (0.55, TINT_ICE);
        }
        if v > loss && v < 1.0 && ((u * 40.0).floor() as i32 + (v * 40.0).floor() as i32) % 2 == 0 {
            return (0.08, TINT_ICE);
        }
        (0.0, TINT_ACCENT)
    });
    for i in 0..900usize {
        let life = fract(t * 0.18 + rndi(i, 1));
        let ang0 = rndi(i, 2) * TAU;
        let r = (1.0 - life) * (a + 0.4) + 0.05;
        let ang = ang0 + life * 5.0;
        let (u, v) = (ang.cos() * r, ang.sin() * r * 0.75);
        let r2 = r + 0.06;
        let a2 = ang - 0.06;
        f.line((a2.cos() * r2, a2.sin() * r2 * 0.75), (u, v), 0.35 + 0.65 * life, if life > 0.85 { TINT_HOT } else { TINT_ACCENT });
    }
    // Edges: pairs differing in exactly one bit.
    for i in 0..16usize {
        for b in 0..4 {
            let j = i ^ (1 << b);
            if j < i {
                continue;
            }
            let (p0, p1) = (proj[i], proj[j]);
            let depth = (p0.2 + p1.2) * 0.5;
            let base = clamp01(0.35 + (depth - 0.8) * 1.5);
            let flow = fract(t * 0.8 + (i * 7 + b) as f32 * 0.13);
            let tint = if b == 3 { TINT_GLITCH } else { TINT_ACCENT };
            let px = f.px();
            for o in [0.0, px] {
                f.line_fn((p0.0, p0.1 + o), (p1.0, p1.1 + o), |k| {
                    if (k - flow).abs() < 0.08 { (1.0, TINT_HOT) } else { (base, tint) }
                });
            }
        }
    }
    for p in &proj {
        f.glow(p.0, p.1, 1.5 + p.2 * 1.5, 1.0, TINT_HOT);
    }
    // Vault core pulse.
    let pulse = fract(t * 0.9);
    f.ring((0.0, 0.0), 0.05 + pulse * 0.5, 1.0 - pulse, TINT_ICE);
}

/// `ModelConfigEdit` -- SYNAPTIC-CONTOUR: a topographic brain whose contour
/// lines drift like breathing tissue, a firing neural graph threaded through
/// it, flanked by full-height MLP layer columns trading activations.
pub fn model_config_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let seed = c.seed;
    let brain = |u: f32, v: f32| -> f32 {
        let l = sd_ellipse(u + 0.33, v + 0.05, 0.6, 0.62);
        let r = sd_ellipse(u - 0.33, v + 0.05, 0.6, 0.62);
        let stem = sd_round_box(u, v - 0.62, 0.1, 0.2, 0.08);
        smin(smin(l, r, 0.15), stem, 0.1)
    };
    f.shade(|u, v, _, _| {
        let d = brain(u, v);
        if d < 0.0 {
            let n = fbm3(u * 1.8, v * 1.8, t * 0.15, 3) + (-d) * 0.8;
            let band = (fract(n * 9.0) - 0.5).abs();
            let fissure = u.abs() < 0.015 && v < 0.45;
            if fissure {
                return (0.0, TINT_ACCENT);
            }
            if band < 0.09 {
                return (0.75, TINT_ACCENT);
            }
            return (0.08 + 0.1 * smoothstep(0.0, -0.3, d), TINT_ACCENT);
        }
        if d < 0.02 {
            return (1.0, TINT_HOT);
        }
        (0.0, TINT_ACCENT)
    });
    // Neurons inside the brain.
    let nodes: Vec<(f32, f32)> = (0..48usize)
        .filter_map(|i| {
            let u = (rndi(i, seed) - 0.5) * 1.8 + 0.04 * (t * 0.7 + i as f32).sin();
            let v = (rndi(i, seed ^ 7) - 0.5) * 1.2 - 0.05 + 0.04 * (t * 0.9 + i as f32).cos();
            if brain(u, v) < -0.05 { Some((u, v)) } else { None }
        })
        .collect();
    for (i, p) in nodes.iter().enumerate() {
        for (j, q) in nodes.iter().enumerate().skip(i + 1) {
            if len(p.0 - q.0, p.1 - q.1) < 0.42 {
                let pulse = fract(t * 0.9 + (i * 13 + j * 7) as f32 * 0.07);
                f.line_fn(*p, *q, |k| if (k - pulse).abs() < 0.07 { (1.0, TINT_HOT) } else { (0.45, TINT_ICE) });
            }
        }
        let fire = fract(t * 0.7 + rndi(i, 3)) < 0.15;
        f.glow(p.0, p.1, if fire { 3.0 } else { 1.5 }, 1.0, if fire { TINT_GLITCH } else { TINT_HOT });
    }
    // MLP columns on both flanks.
    let layers = [(-a + 0.25, 9), (-a + 0.7, 6), (a - 0.7, 6), (a - 0.25, 9)];
    let pos = |li: usize, k: usize| -> (f32, f32) {
        let (x, n) = layers[li];
        (x, -0.85 + 1.7 * (k as f32 + 0.5) / n as f32)
    };
    for (l0, l1) in [(0usize, 1usize), (3, 2)] {
        for i in 0..layers[l0].1 {
            for j in 0..layers[l1].1 {
                let p = pos(l0, i);
                let q = pos(l1, j);
                let w = noise3(i as f32, j as f32, t * 0.8);
                if w > 0.45 {
                    f.line(p, q, (w - 0.45) * 1.4, TINT_ACCENT);
                }
            }
        }
        // Axons into the brain.
        for j in 0..layers[l1].1 {
            let q = pos(l1, j);
            let tgt = (if l0 == 0 { -0.75 } else { 0.75 }, q.1 * 0.5);
            let pulse = fract(t * 1.2 + j as f32 * 0.21);
            f.line_fn(q, tgt, |k| if (k - pulse).abs() < 0.06 { (1.0, TINT_HOT) } else { (0.25, TINT_ACCENT) });
        }
    }
    for (li, &(_, n)) in layers.iter().enumerate() {
        for k in 0..n {
            let p = pos(li, k);
            let act = noise3(li as f32 * 3.0, k as f32, t * 1.5);
            f.glow(p.0, p.1, 2.0, 0.4 + 0.6 * act, if act > 0.7 { TINT_HOT } else { TINT_ACCENT });
        }
    }
}
