//! Group D -- Network & Data. Full-bleed micro-dot shaders; see `dots.rs`.

use super::dots::*;
use super::{Ctx, ProgressKind};

/// `IncomingDataStream` -- TELEMETRIC-CONE: a hyperspace data tunnel. Banded
/// walls rush toward the viewer, the vanishing point drifts, and hundreds of
/// packet-streaks fly out of the singularity. When real progress is known,
/// the completed fraction of tunnel rings glows hot -- otherwise it never
/// pretends to know.
pub fn incoming_data_stream(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let speed = 1.0;
    let vp = (0.35 * (t * 0.4).sin(), 0.18 * (t * 0.55).cos());
    let pct = match c.stats.progress {
        ProgressKind::Determinate { pct } => Some(pct as f32 / 100.0),
        ProgressKind::Indeterminate => None,
    };
    f.shade(|u, v, _, _| {
        let (du, dv) = (u - vp.0, (v - vp.1) * 1.15);
        let r = len(du, dv).max(1e-3);
        let th = dv.atan2(du);
        let z = 0.32 / r;
        let rings = fract(z * 1.5 - t * 1.6 * speed);
        let segs = fract(th / TAU * 24.0 + z * 0.15);
        let fog = smoothstep(9.0, 0.6, z);
        let mut l;
        let mut tint = TINT_ACCENT;
        if rings < 0.12 {
            l = 0.9 * fog;
            if let Some(p) = pct {
                // Rings closer than the completion depth are "received".
                if 1.0 - (z / 9.0).min(1.0) < p {
                    tint = TINT_HOT;
                }
            }
        } else if segs < 0.06 {
            l = 0.5 * fog;
        } else {
            let n = noise3(th * 4.0, z * 2.0 - t * 4.0, 0.0);
            l = if n > 0.72 { 0.55 * fog } else { 0.06 * fog };
            tint = TINT_ICE;
        }
        if r < 0.06 {
            l = 1.0;
            tint = TINT_HOT;
        }
        (l, tint)
    });
    // Packet streaks.
    for i in 0..520usize {
        let ang = rndi(i, 1) * TAU;
        let sp = 0.6 + rndi(i, 2) * 1.6;
        let life = fract(t * 0.35 * sp + rndi(i, 3));
        let z = (1.0 - life).max(0.02);
        let r0 = 0.08 / z;
        let r1 = 0.08 / (z + 0.06);
        let (ca, sa) = (ang.cos(), ang.sin());
        let p0 = (vp.0 + ca * r0, vp.1 + sa * r0 / 1.15);
        let p1 = (vp.0 + ca * r1, vp.1 + sa * r1 / 1.15);
        if p0.0.abs() > a + 0.5 || p0.1.abs() > 1.5 {
            continue;
        }
        let tint = if i % 9 == 0 { TINT_GLITCH } else if i % 3 == 0 { TINT_ICE } else { TINT_HOT };
        f.line(p1, p0, clamp01(life * 1.4), tint);
    }
}

/// `ArchiveWrite` -- OBSIDIAN-COMPACT: a faceted 3D cube tumbling in space,
/// squeezing shut on a beat with a shockwave, as a vortex of loose dots from
/// every corner of the panel is sucked in and packed away.
pub fn archive_write(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let beat = fract(t / 1.6);
    let squeeze = 1.0 - 0.28 * (smoothstep(0.0, 0.12, beat) - smoothstep(0.12, 0.5, beat));
    let (ry, rx) = (t * 0.6, 0.55 + 0.2 * (t * 0.4).sin());
    let s = 0.48;
    let verts: Vec<(f32, f32, f32)> = (0..8)
        .map(|i| {
            let x = if i & 1 == 0 { -s } else { s };
            let y = (if i & 2 == 0 { -s } else { s }) * squeeze;
            let z = if i & 4 == 0 { -s } else { s };
            let (x, z) = rot(x, z, ry);
            let (y, z) = rot(y, z, rx);
            (x, y, z)
        })
        .collect();
    let light = norm3((-0.4, -0.8, -0.5));
    // Vortex first (behind the cube).
    for i in 0..1100usize {
        let life = fract(t * 0.22 + rndi(i, 4));
        let ang = rndi(i, 5) * TAU + life * 7.0;
        let r = (1.0 - life).powf(1.3) * (a + 0.6) + 0.15;
        let (u, v) = (ang.cos() * r, ang.sin() * r * 0.8);
        let (a2, r2) = (ang - 0.08, r + 0.07);
        f.line((a2.cos() * r2, a2.sin() * r2 * 0.8), (u, v), 0.3 + 0.7 * life, if i % 7 == 0 { TINT_ICE } else { TINT_ACCENT });
    }
    // Shockwave on each squeeze.
    if beat < 0.5 {
        let k = beat / 0.5;
        f.ring((0.0, 0.0), 0.5 + k * (a + 0.5), 1.0 - k, TINT_GLITCH);
        f.ring((0.0, 0.0), 0.45 + k * (a + 0.3), (1.0 - k) * 0.6, TINT_HOT);
    }
    let faces: [([usize; 4], (f32, f32, f32)); 6] = [
        ([0, 1, 3, 2], (0.0, 0.0, -1.0)),
        ([4, 6, 7, 5], (0.0, 0.0, 1.0)),
        ([0, 2, 6, 4], (-1.0, 0.0, 0.0)),
        ([1, 5, 7, 3], (1.0, 0.0, 0.0)),
        ([0, 4, 5, 1], (0.0, -1.0, 0.0)),
        ([2, 3, 7, 6], (0.0, 1.0, 0.0)),
    ];
    let mut drawn: Vec<(f32, [(f32, f32); 4], f32, bool)> = Vec::new();
    for (idx, n0) in faces.iter() {
        let (nx, nz) = rot(n0.0, n0.2, ry);
        let (ny, nz) = rot(n0.1, nz, rx);
        if nz > 0.0 {
            continue; // facing away (camera looks down +z)
        }
        let lam = clamp01(nx * light.0 + ny * light.1 + nz * light.2);
        let quad = idx.map(|k| {
            let (x, y, z) = verts[k];
            let p = project(x, y, z, 3.0, 1.0);
            (p.0, p.1)
        });
        let depth: f32 = idx.iter().map(|&k| verts[k].2).sum::<f32>() / 4.0;
        drawn.push((depth, quad, 0.2 + 0.8 * lam, n0.1 < 0.0));
    }
    drawn.sort_by(|p, q| q.0.partial_cmp(&p.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, quad, l, is_top) in drawn {
        super::group_a_code::fill_quad(f, &quad, l, if is_top { TINT_HOT } else { TINT_ACCENT }, false);
        if is_top {
            // Tape stripe across the lid.
            let m0 = ((quad[0].0 + quad[1].0) * 0.5, (quad[0].1 + quad[1].1) * 0.5);
            let m1 = ((quad[3].0 + quad[2].0) * 0.5, (quad[3].1 + quad[2].1) * 0.5);
            for k in -2..=2 {
                let o = k as f32 * f.px();
                f.line((m0.0 + o, m0.1), (m1.0 + o, m1.1), 1.0, TINT_GLITCH);
            }
        }
    }
}
