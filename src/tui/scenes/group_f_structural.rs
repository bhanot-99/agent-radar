//! Group F -- Structural, plus the idle sentinel. Full-bleed micro-dot
//! shaders; see `dots.rs`.

use super::dots::*;
use super::Ctx;

/// `WorkspaceExpansion` -- SKYSCRAPER-HORIZON: a synthwave megacity. A
/// slatted sun sinks behind a skyline whose windows flicker; a new tower
/// rises floor by floor under a crane in the center, over an infinite
/// perspective grid racing toward the viewer.
pub fn workspace_expansion(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let hz = 0.22;
    let seed = c.seed;
    let rise = fract(t / 10.0);
    let tower_h = 0.15 + 0.95 * smoothstep(0.0, 0.85, rise);
    f.shade(|u, v, x, y| {
        let mut out = (0.0, TINT_ACCENT);
        // Sun with horizontal slats that widen toward the horizon.
        let sd = len(u, v - hz + 0.08);
        if sd < 0.85 && v < hz {
            let slat = fract((v - t * 0.04) * 14.0);
            let cut = slat < smoothstep(-0.4, hz, v) * 0.6;
            if !cut {
                out = (0.35 + 0.6 * (1.0 - (v + 0.6) / (hz + 0.6)).max(0.0), if v < -0.2 { TINT_HOT } else { TINT_GLITCH });
            }
        }
        // Perspective grid floor.
        if v > hz {
            let d = v - hz;
            let z = 0.35 / d;
            let gz = fract(z - t * 1.2);
            let gx = fract(u * z * 0.9);
            let fade = smoothstep(0.0, 0.25, d);
            if gz < 0.06 * z.min(4.0) || gx < 0.035 * z.min(4.0) {
                out = (0.85 * fade + 0.1, TINT_ACCENT);
            }
        }
        // Skyline: column-hashed buildings standing on the horizon.
        let bw = 0.11;
        let bi = (u / bw).floor() as i32;
        let hs = hash(seed ^ (bi as u32).wrapping_mul(0x9E37_79B9));
        let bh = 0.06 + rnd(hs) * 0.42 * (0.35 + 0.65 * ((u / a).abs()));
        let in_center = u.abs() < 0.2;
        if !in_center && v < hz && v > hz - bh && fract(u / bw) > 0.08 {
            let wx = (x / 2) % 3 == 1;
            let wy = (y / 2) % 3 == 1;
            let flick = hash(hs ^ (x as u32 / 6) ^ ((y as u32 / 6) << 8) ^ (t * 1.5) as u32) % 4 == 0;
            let edge = fract(u / bw) < 0.13 || v < hz - bh + 0.012;
            out = if wx && wy && flick { (0.95, TINT_ICE) } else if edge { (0.7, TINT_ACCENT) } else { (0.0, TINT_ACCENT) };
        }
        // The new tower.
        if in_center && v < hz && v > hz - tower_h && u.abs() < 0.17 {
            let top = hz - tower_h;
            let floor = ((v - top) / 0.06) as i32;
            let fresh = v - top < 0.12;
            let frame = (u.abs() - 0.16).abs() < 0.01 || fract((v - top) / 0.06) < 0.15;
            let diag = (fract((u + (v - top)) / 0.12) - 0.5).abs() < 0.06 && fresh;
            out = if frame || diag { (1.0, if fresh { TINT_HOT } else { TINT_ACCENT }) } else if floor % 2 == 0 && (x / 2) % 2 == 0 { (0.6, TINT_ACCENT) } else { (0.15, TINT_ACCENT) };
        }
        out
    });
    // Crane over the tower.
    let top = hz - tower_h;
    let mast = (0.24, top - 0.3);
    f.line((0.24, hz), mast, 1.0, TINT_HOT);
    let jib_ang = 0.25 * (t * 0.5).sin();
    let jib_end = (mast.0 - 0.9 * jib_ang.cos(), mast.1 + 0.9 * jib_ang.sin());
    f.line(mast, jib_end, 1.0, TINT_HOT);
    f.line(mast, (mast.0 + 0.3, mast.1 + 0.05), 0.9, TINT_HOT);
    let hook_x = mix(mast.0, jib_end.0, 0.6 + 0.3 * (t * 0.8).sin());
    let hook_y = mix(mast.1, jib_end.1, 0.6) + 0.25 + 0.05 * (t * 1.7).sin();
    f.line((hook_x, mix(mast.1, jib_end.1, 0.6)), (hook_x, hook_y), 0.7, TINT_ACCENT);
    for dy in 0..4 {
        f.line((hook_x - 0.06, hook_y + dy as f32 * f.px()), (hook_x + 0.06, hook_y + dy as f32 * f.px()), 0.9, TINT_GLITCH);
    }
    // Welding sparks off the fresh floor.
    for i in 0..260usize {
        let life = fract(t * 1.6 + rndi(i, 2));
        let sx = (rndi(i, 3) - 0.5) * 0.34;
        let ang = -PI * 0.5 + (rndi(i, 4) - 0.5) * 2.6;
        let sp = 0.15 + rndi(i, 5) * 0.45;
        f.plot(sx + ang.cos() * sp * life, top + ang.sin() * sp * life + life * life * 0.7, 1.0 - life, if life < 0.3 { TINT_HOT } else { TINT_GLITCH });
    }
}

/// `MassDeletion` -- STIPPLE-DEMOLITION: a wall of file glyphs (one per
/// actually-deleted file, capped) cracks from an impact point, then
/// detonates -- every dot becomes a ballistic particle with real gravity and
/// floor bounce -- behind an expanding shockwave. Loops.
pub fn mass_deletion(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let count = c
        .stats
        .rows
        .iter()
        .find(|(l, _)| *l == "Count")
        .and_then(|(_, v)| v.split_whitespace().next()?.parse::<usize>().ok())
        .unwrap_or(48);
    let cycle = 5.0;
    let ph = fract(t / cycle);
    let slot = (t / cycle) as u32;
    let impact = ((rnd(slot ^ c.seed) - 0.5) * a, (rnd(slot ^ c.seed ^ 9) - 0.5) * 0.6);
    let crack_t = smoothstep(0.05, 0.4, ph);
    let blast = ((ph - 0.42) / 0.58).max(0.0) * cycle * 0.58;
    // Wall layout: icons in a grid sized to the real count (capped).
    let n = count.clamp(8, 160);
    let cols = ((n as f32 * a).sqrt().ceil() as usize).max(4);
    let rows = n.div_ceil(cols);
    let cw = (2.0 * a - 0.3) / cols as f32;
    let ch = 1.5 / rows as f32;
    let floor_v = 0.98;
    let (w, h) = (f.w, f.h);
    let mut particles: Vec<(i32, i32, f32, u8)> = Vec::with_capacity(w * h / 3);
    for y in 0..h {
        for x in 0..w {
            let (u, v) = f.uv(x, y);
            let gx = (u + a - 0.15) / cw;
            let gy = (v + 0.8) / ch;
            if gx < 0.0 || gy < 0.0 {
                continue;
            }
            let (ci, ri) = (gx as usize, gy as usize);
            if ci >= cols || ri >= rows || ri * cols + ci >= n {
                continue;
            }
            let (fx, fy) = (fract(gx), fract(gy));
            // File glyph: dog-eared page with text lines.
            let in_page = fx > 0.15 && fx < 0.85 && fy > 0.08 && fy < 0.92 && !(fx > 0.65 && fy < 0.28 && (fx - 0.65) > (fy - 0.08));
            if !in_page {
                continue;
            }
            let border = fx < 0.2 || fx > 0.8 || fy < 0.13 || fy > 0.87;
            let text = (fract(fy * 6.0) < 0.3) && fx > 0.28 && fx < 0.72;
            let mut l = if border { 0.9 } else if text { 0.6 } else { 0.12 };
            let mut tint = TINT_ACCENT;
            // Cracks radiating from the impact.
            let (du, dv) = (u - impact.0, v - impact.1);
            let ang = dv.atan2(du);
            let rad = len(du, dv);
            let ray = (fract(ang / TAU * 11.0 + noise3(rad * 6.0, 0.0, slot as f32) * 0.3) - 0.5).abs();
            if ray < 0.03 && rad < crack_t * 1.6 {
                l = 1.0;
                tint = TINT_GLITCH;
            }
            if blast <= 0.0 {
                particles.push((x as i32, y as i32, l, tint));
                continue;
            }
            // Ballistic: velocity away from impact, gravity down, floor bounce.
            let hs = hash((x as u32) ^ ((y as u32) << 11) ^ slot);
            let sp = (0.55 / (0.3 + rad)) * (0.4 + rnd(hs) * 0.8);
            let (vx, mut vy) = (du / rad.max(1e-3) * sp, dv / rad.max(1e-3) * sp - 0.6 * rnd(hs ^ 1));
            let g = 1.8;
            let mut pu = u + vx * blast * 0.6;
            let mut pv = v + vy * blast + 0.5 * g * blast * blast;
            if pv > floor_v {
                // One damped bounce, then settle as rubble.
                let tb = {
                    let disc = vy * vy + 2.0 * g * (floor_v - v);
                    (-vy + disc.max(0.0).sqrt()) / g
                };
                let rem = blast - tb;
                vy = -(vy + g * tb) * 0.3;
                pv = (floor_v + vy * rem + 0.5 * g * rem * rem).min(floor_v);
                pu = u + vx * (tb + rem * 0.3) * 0.6;
            }
            let (px, py) = f.to_dot(pu, pv);
            let fade = 1.0 - smoothstep(0.75, 1.0, ph);
            particles.push((px as i32, py as i32, l.max(0.5) * fade, if rnd(hs ^ 2) < 0.15 { TINT_HOT } else { tint }));
        }
    }
    for (x, y, l, tint) in particles {
        f.dot(x, y, l, tint);
    }
    if blast > 0.0 {
        let k = (blast / 1.2).min(1.0);
        f.ring(impact, k * (a + 0.8), 1.0 - k, TINT_HOT);
        f.ring(impact, k * (a + 0.5) * 0.85, (1.0 - k) * 0.7, TINT_GLITCH);
        f.glow(impact.0, impact.1, 10.0 * (1.0 - k), 1.0, TINT_HOT);
    }
    // Rubble line.
    let px = f.px();
    f.line((-a, floor_v + px), (a, floor_v + px), 0.5, TINT_ACCENT);
}

const PAGE_W: f32 = 0.42;
const PAGE_H: f32 = 0.78;
const PAGE_LINES: usize = 12;

#[inline]
fn line_y(i: usize) -> f32 {
    -0.5 + i as f32 * 0.105
}

/// Length and word pattern of text line `i` on the page (stable per file).
#[inline]
fn line_len(i: usize, seed: u32) -> f32 {
    0.28 + rnd(seed ^ (i as u32).wrapping_mul(0x9E37_79B9)) * 0.38
}

/// What the document looks like at page-local (x, y): None outside the
/// page, otherwise which part was hit.
#[derive(Clone, Copy, PartialEq)]
enum PagePart {
    Body,
    Border,
    Flap,
    Text(usize, f32), // line index, 0..1 position along the line
}

fn page_part(x: f32, y: f32, px: f32, seed: u32) -> Option<PagePart> {
    if x.abs() > PAGE_W || y.abs() > PAGE_H {
        return None;
    }
    // Dog-eared top-right corner.
    let fold = 0.2;
    let cx = x - (PAGE_W - fold);
    let cy = y + PAGE_H;
    if cx > 0.0 && cy < fold {
        if cx > cy {
            return None;
        }
        return Some(if cx > cy - px * 1.2 { PagePart::Border } else { PagePart::Flap });
    }
    let edge = (PAGE_W - x.abs()).min(PAGE_H - y.abs());
    if edge < px * 1.3 || ((x - (PAGE_W - fold)).abs() < px * 0.8 && cy < fold) {
        return Some(PagePart::Border);
    }
    for i in 0..PAGE_LINES {
        let ly = line_y(i);
        if (y - ly).abs() < px * 1.6 {
            let x0 = -PAGE_W + 0.12;
            let len = line_len(i, seed);
            let k = (x - x0) / len;
            let gap = fract((x - x0) * 6.5 + rnd(seed ^ i as u32 ^ 0x55) * 3.0) > 0.8;
            if (0.0..1.0).contains(&k) && !gap {
                return Some(PagePart::Text(i, k));
            }
        }
    }
    Some(PagePart::Body)
}

/// `FileMutation` -- LIVING-DOCUMENT: a giant stippled document amid a
/// field of sibling file icons wired to it by inode links. What happens to
/// it follows the real file op: MODIFIED rewrites it line by line under a
/// write head with a +/- diff gutter; CREATED assembles it from dots
/// streaming in from every edge, then types it out; DELETED stamps it with
/// an X and feeds it through a shredder; RENAMED slides it between slots
/// while its real name decodes out of glitch noise.
pub fn file_mutation(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let px = f.px();
    let seed = c.seed;
    let op = c.stats.rows.iter().find(|(l, _)| *l == "Op").map(|(_, v)| v.as_str()).unwrap_or("MODIFIED");
    let period = 4.0;
    let ph = fract(t / period);
    let cyc = (t / period) as u32;

    // Page placement (RENAMED slides it between two slots).
    let slot = a * 0.5;
    let (pu, pv, ps) = if op == "RENAMED" {
        let from = if cyc % 2 == 0 { -slot } else { slot };
        (mix(from, -from, smoothstep(0.15, 0.6, ph)), 0.05, 0.72)
    } else {
        (0.0, 0.0, 1.0)
    };
    let lines_f = PAGE_LINES as f32;

    // Background: grid of sibling file icons, a few flickering with activity.
    let (cw, ch) = (0.3f32, 0.4f32);
    f.shade(|u, v, _, _| {
        if (u - pu).abs() < PAGE_W * ps + 0.2 && op != "RENAMED" {
            return (0.0, TINT_ACCENT);
        }
        let gi = ((u + a) / cw).floor();
        let gj = ((v + 1.0) / ch).floor();
        let (dx, dy) = (u + a - (gi + 0.5) * cw, v + 1.0 - (gj + 0.5) * ch);
        let (iw, ih) = (0.065, 0.1);
        if dx.abs() > iw || dy.abs() > ih {
            return (0.0, TINT_ACCENT);
        }
        let hs = hash(seed ^ (gi as i32 as u32).wrapping_mul(7919) ^ (gj as i32 as u32).wrapping_mul(104_729));
        let busy = hash(hs ^ (t * 2.0) as u32) % 23 == 0;
        let edge = (iw - dx.abs()).min(ih - dy.abs()) < px * 0.9 || (dy + ih > 0.0 && (dy - (-ih + 0.035)).abs() < px * 0.5 && dx.abs() < iw * 0.6 && hs & 1 == 0);
        let lines = (fract(dy / 0.04) < 0.3) && dx.abs() < iw * 0.6 && dy > -ih + 0.03;
        if edge {
            (if busy { 0.9 } else { 0.32 }, if busy { TINT_ICE } else { TINT_ACCENT })
        } else if lines {
            (if busy { 0.6 } else { 0.14 }, TINT_ACCENT)
        } else {
            (0.0, TINT_ACCENT)
        }
    });

    // Inode links from the page to sibling icons, pulses racing along them.
    for k in 0..8 {
        let side = if k % 2 == 0 { -1.0 } else { 1.0 };
        let hs = hash(seed ^ (k as u32).wrapping_mul(0x85EB_CA6B));
        let gi = if side < 0.0 { (rnd(hs) * (a - 0.7) / cw) as i32 } else { ((a + 0.7) / cw) as i32 + (rnd(hs) * (a - 0.7) / cw) as i32 };
        let gj = (rnd(hs ^ 1) * (2.0 / ch)) as i32;
        let target = (-a + (gi as f32 + 0.5) * cw, -1.0 + (gj as f32 + 0.5) * ch);
        let from = (pu + side * PAGE_W * ps, pv + (k as f32 / 8.0 - 0.45) * PAGE_H * ps);
        let pulse = fract(t * 0.6 + rnd(hs ^ 2));
        f.line_fn(from, target, |q| if (q - pulse).abs() < 0.07 { (1.0, TINT_HOT) } else { (0.3, TINT_ICE) });
        f.glow(target.0, target.1, 2.0, 0.9, TINT_ICE);
    }

    // Per-op state for the page's text lines.
    let (write_k, write_x) = match op {
        "CREATED" => {
            let p = ((ph - 0.45) / 0.5).clamp(0.0, 1.0) * lines_f;
            (p.floor() as usize, fract(p))
        }
        _ => {
            let p = (ph * 1.15).min(1.0) * lines_f;
            (p.floor() as usize, fract(p))
        }
    };
    let changed = |i: usize| hash(seed ^ cyc.wrapping_mul(31) ^ i as u32) % 5 < 2;
    let shred_slot = 0.42;
    let shred_drop = ((ph - 0.3) / 0.7).max(0.0) * 2.4;

    // The document itself: walk every dot in the page's bounding box in
    // page-local space so it can be moved/shredded as a whole.
    let (bw, bh) = (PAGE_W * ps + 0.05, PAGE_H * ps + 0.05);
    let (x0, y0) = f.to_dot(pu - bw, pv - bh);
    let (x1, y1) = f.to_dot(pu + bw, pv + bh);
    let light_x = (t * 0.8).sin();
    for yy in (y0.floor() as i32).max(0)..(y1.ceil() as i32).min(f.h as i32) {
        for xx in (x0.floor() as i32).max(0)..(x1.ceil() as i32).min(f.w as i32) {
            let (u, v) = f.uv(xx as usize, yy as usize);
            let (lx, ly) = ((u - pu) / ps, (v - pv) / ps);
            let Some(part) = page_part(lx, ly, px / ps, seed) else { continue };
            // Paper shading: a soft highlight sliding across the sheet.
            let sheen = 0.5 + 0.5 * (-(lx - light_x * 0.3).powi(2) * 6.0).exp();
            let (mut l, mut tint) = match part {
                PagePart::Body => (0.07 + 0.08 * sheen, TINT_ACCENT),
                PagePart::Border => (1.0, TINT_HOT),
                PagePart::Flap => (0.45 + 0.3 * sheen, TINT_ACCENT),
                PagePart::Text(i, k) => match op {
                    "CREATED" => {
                        if i < write_k {
                            (0.9, TINT_ICE)
                        } else if i == write_k && k < write_x {
                            (1.0, TINT_HOT)
                        } else {
                            (0.0, TINT_ACCENT)
                        }
                    }
                    "MODIFIED" | "" => {
                        if i < write_k {
                            if changed(i) { (1.0, TINT_ICE) } else { (0.75, TINT_ACCENT) }
                        } else if i == write_k {
                            if k < write_x { (1.0, TINT_HOT) } else { (0.6, TINT_GLITCH) }
                        } else {
                            (0.75, TINT_ACCENT)
                        }
                    }
                    _ => (0.5 * sheen + 0.4, TINT_ACCENT),
                },
            };
            match op {
                "CREATED" if ph < 0.42 => continue, // still assembling
                "DELETED" if ph < 0.3 => {
                    // Condemned: a red X stamped across the page, flickering.
                    let d1 = (lx * PAGE_H - ly * PAGE_W).abs() / PAGE_H;
                    let d2 = (lx * PAGE_H + ly * PAGE_W).abs() / PAGE_H;
                    let grow = smoothstep(0.0, 0.2, ph) * PAGE_W;
                    if (d1 < 0.035 || d2 < 0.035) && lx.abs() < grow {
                        l = 1.0;
                        tint = TINT_GLITCH;
                    } else if (t * 12.0).sin() > 0.6 {
                        l *= 0.5;
                    }
                }
                "DELETED" => {
                    // Fed through the shredder: above the slot the page is
                    // whole; below it, it is cut into strips that splay out
                    // and fall away.
                    let wv = v + shred_drop;
                    if wv < shred_slot {
                        f.dot(xx, (yy as f32 + shred_drop / px) as i32, l, tint);
                        continue;
                    }
                    let strip_w = 0.055;
                    let strip = (lx / strip_w).floor();
                    if fract(lx / strip_w) < 0.22 {
                        continue;
                    }
                    let below = wv - shred_slot;
                    let hs = hash(seed ^ (strip as i32 as u32).wrapping_mul(2_654_435_761) ^ cyc);
                    let fall = below * below * (0.8 + rnd(hs) * 1.6);
                    let splay = strip * strip_w * below * 0.9 + (rnd(hs ^ 1) - 0.5) * below * 0.6;
                    let wobble = (below * 14.0 + strip).sin() * 0.02 * below;
                    let (nu, nv) = (u + splay + wobble, wv + fall);
                    let fade = 1.0 - smoothstep(0.5, 1.4, below);
                    f.plot(nu, nv, l.max(0.6) * fade, if rnd(hs ^ 3) < 0.3 { TINT_GLITCH } else { tint });
                    continue;
                }
                _ => {}
            }
            f.dot(xx, yy, l, tint);
        }
    }

    match op {
        "CREATED" => {
            // Dots streaming in from every edge to assemble the page.
            for i in 0..1100usize {
                let delay = rndi(i, 1) * 0.15;
                let k = clamp01((ph - delay) / 0.3);
                if k >= 1.0 {
                    continue;
                }
                let target = if i % 3 == 0 {
                    // Perimeter point.
                    let s = rndi(i, 2) * 4.0;
                    let (x, y) = match s as i32 {
                        0 => (-PAGE_W + fract(s) * 2.0 * PAGE_W, -PAGE_H),
                        1 => (PAGE_W, -PAGE_H + fract(s) * 2.0 * PAGE_H),
                        2 => (PAGE_W - fract(s) * 2.0 * PAGE_W, PAGE_H),
                        _ => (-PAGE_W, PAGE_H - fract(s) * 2.0 * PAGE_H),
                    };
                    (pu + x, pv + y)
                } else {
                    (pu + (rndi(i, 3) - 0.5) * 2.0 * PAGE_W, pv + (rndi(i, 4) - 0.5) * 2.0 * PAGE_H)
                };
                let ang = rndi(i, 5) * TAU;
                let start = (ang.cos() * (a + 0.6), ang.sin() * 1.4);
                let e = k * k * (3.0 - 2.0 * k);
                let p = (mix(start.0, target.0, e), mix(start.1, target.1, e));
                let kb = (k - 0.06).max(0.0);
                let eb = kb * kb * (3.0 - 2.0 * kb);
                let q = (mix(start.0, target.0, eb), mix(start.1, target.1, eb));
                f.line(q, p, 0.4 + 0.6 * k, if i % 4 == 0 { TINT_HOT } else { TINT_ICE });
            }
            if ph > 0.42 && ph < 0.6 {
                let k = (ph - 0.42) / 0.18;
                f.ring((pu, pv), 0.3 + k * a, 1.0 - k, TINT_ICE);
            }
        }
        "DELETED" if ph >= 0.3 => {
            // The shredder: a toothed slot spanning the page.
            let w = PAGE_W + 0.12;
            for o in 0..3 {
                let y = shred_slot + o as f32 * px;
                f.line((pu - w, y), (pu + w, y), 1.0, if o == 1 { TINT_GLITCH } else { TINT_HOT });
            }
            let n = (2.0 * w / 0.055) as i32;
            for k in 0..=n {
                let x = pu - w + k as f32 * 0.055;
                let jit = ((t * 30.0 + k as f32).sin() * 0.5 + 0.5) * px * 2.0;
                f.line((x, shred_slot - 3.0 * px - jit), (x, shred_slot - px), 0.8, TINT_ACCENT);
            }
        }
        "RENAMED" => {
            // Slots, a dashed transfer arc, and the real name decoding.
            for s in [-slot, slot] {
                let (hw, hh) = (PAGE_W * ps + 0.04, PAGE_H * ps + 0.04);
                for (p0, p1) in [((s - hw, pv - hh), (s + hw, pv - hh)), ((s + hw, pv - hh), (s + hw, pv + hh)), ((s + hw, pv + hh), (s - hw, pv + hh)), ((s - hw, pv + hh), (s - hw, pv - hh))] {
                    f.line_fn(p0, p1, |q| if (q * 30.0).fract() < 0.5 { (0.35, TINT_ICE) } else { (0.0, TINT_ICE) });
                }
            }
            let flow = fract(t * 0.8);
            let dir = if cyc % 2 == 0 { 1.0 } else { -1.0 };
            let n = 160;
            for i in 0..n {
                let q = i as f32 / n as f32;
                let x = mix(-slot * dir, slot * dir, q);
                let y = pv + PAGE_H * ps + 0.13 + (q * PI).sin() * 0.06;
                let on = (q * 24.0 - flow * 3.0).fract() < 0.5;
                if on {
                    f.plot(x, y, if (q - flow).abs() < 0.05 { 1.0 } else { 0.6 }, TINT_HOT);
                }
            }
            // The file's real name, in the glyphs the bitmap font can draw.
            let base = c.path.rsplit('/').find(|p| !p.is_empty()).unwrap_or(c.path);
            let name: String = base
                .chars()
                .map(|ch| ch.to_ascii_uppercase())
                .map(|ch| if ch.is_ascii_alphanumeric() || "._-<>/$#&@!?*=+:".contains(ch) { ch } else { '_' })
                .take(((2.0 * a - 0.4) / (12.0 * px)) as usize)
                .collect();
            let gpx = 2.0 * px;
            let tw = text_width(&name, gpx);
            let o = (-tw * 0.5, -0.86);
            let reveal = smoothstep(0.1, 0.7, ph);
            let glyphs = ['#', '%', '&', '@', '$', '?', '0', '1', 'X', 'Z'];
            let shown: String = name
                .chars()
                .enumerate()
                .map(|(i, ch)| {
                    if (i as f32 + 0.5) / name.len().max(1) as f32 <= reveal {
                        ch
                    } else {
                        glyphs[(hash(i as u32 ^ (t * 14.0) as u32) as usize) % glyphs.len()]
                    }
                })
                .collect();
            let (gx0, gy0) = f.to_dot(o.0, o.1);
            let (gx1, gy1) = f.to_dot(o.0 + tw, o.1 + 7.0 * gpx);
            for yy in gy0 as i32..gy1 as i32 + 1 {
                for xx in gx0 as i32..gx1 as i32 + 1 {
                    if xx < 0 || yy < 0 {
                        continue;
                    }
                    let (u, v) = f.uv(xx as usize, yy as usize);
                    if text_sample(&shown, u, v, o, gpx) > 0.5 {
                        f.dot(xx, yy, 1.0, if reveal >= 1.0 { TINT_ICE } else { TINT_GLITCH });
                    }
                }
            }
        }
        _ => {
            // MODIFIED: write head sweeping the current line, diff gutter.
            let hy = pv + line_y(write_k.min(PAGE_LINES - 1)) * ps;
            let hx = pu + (-PAGE_W + 0.12 + write_x * line_len(write_k.min(PAGE_LINES - 1), seed)) * ps;
            f.line((-a, hy), (pu - PAGE_W * ps - 0.02, hy), 0.25, TINT_GLITCH);
            f.line((pu + PAGE_W * ps + 0.02, hy), (a, hy), 0.25, TINT_GLITCH);
            if fract(t * 3.0) < 0.6 {
                for o in -3..=3 {
                    f.plot(hx + px, hy + o as f32 * px, 1.0, TINT_HOT);
                    f.plot(hx + 2.0 * px, hy + o as f32 * px, 1.0, TINT_HOT);
                }
            }
            for i in 0..70usize {
                let life = fract(t * 2.2 + rndi(i, 9));
                let ang = rndi(i, 10) * TAU;
                let sp = 0.1 + rndi(i, 11) * 0.3;
                f.plot(hx + ang.cos() * sp * life, hy + ang.sin() * sp * life + life * life * 0.2, 1.0 - life, if i % 2 == 0 { TINT_HOT } else { TINT_ICE });
            }
            for i in 0..write_k.min(PAGE_LINES) {
                if !changed(i) {
                    continue;
                }
                let gy = pv + line_y(i) * ps;
                let gx = pu - PAGE_W * ps - 0.07;
                let added = hash(seed ^ i as u32 ^ 0xADD) % 3 != 0;
                let tint = if added { TINT_ICE } else { TINT_GLITCH };
                for o in -2..=2 {
                    f.plot(gx + o as f32 * px, gy, 1.0, tint);
                    if added {
                        f.plot(gx, gy + o as f32 * px, 1.0, tint);
                    }
                }
            }
        }
    }
}

/// SystemIdle -- CELESTIAL-RADAR: a rotating dotted planet with a landmass
/// mask, a radar sweep circling the whole panel with phosphor afterglow,
/// blips lighting up as the beam passes, satellites in orbit, stars
/// twinkling at the edges.
pub fn system_idle(f: &mut DotField, t: f32) {
    let a = f.aspect();
    let sweep = t * 0.9;
    let center = (0.0f32, 0.0f32);
    f.shade(|u, v, x, y| {
        let (du, dv) = (u - center.0, v - center.1);
        let r = len(du, dv);
        let ang = dv.atan2(du);
        let lag = (sweep - ang).rem_euclid(TAU);
        let mut out: (f32, u8) = (0.0, TINT_ACCENT);
        // Range rings & spokes, spanning past the panel edges.
        let ring = fract(r / 0.35);
        if ring < 0.03 / r.max(0.3) && (x + y) % 2 == 0 {
            out = (0.3, TINT_ACCENT);
        }
        if (fract(ang / TAU * 12.0) < 0.004 / r.max(0.2)) && r > 0.9 {
            out = (0.25, TINT_ACCENT);
        }
        // Afterglow wedge.
        if lag < 1.4 {
            let k = 1.0 - lag / 1.4;
            out.0 = out.0.max(k * k * 0.32);
            if lag < 0.03 {
                out = (1.0, TINT_ICE);
            }
        }
        // Twinkling stars.
        let hs = hash((x as u32) ^ ((y as u32) << 13) ^ 0xBEEF);
        if hs % 173 == 0 && (t * 2.0 + rnd(hs) * 6.0).sin() > 0.3 {
            out = (0.7, TINT_HOT);
        }
        out
    });
    // Fibonacci-sphere planet.
    let pr = 0.7;
    let n = 3600;
    let spin = t * 0.3;
    let golden = PI * (3.0 - 5f32.sqrt());
    for i in 0..n {
        let y = 1.0 - (i as f32 + 0.5) / n as f32 * 2.0;
        let rad = (1.0 - y * y).sqrt();
        let th = golden * i as f32 + spin;
        let (x, z) = (th.cos() * rad, th.sin() * rad);
        if z < -0.05 {
            continue;
        }
        let lon = golden * i as f32;
        let land = noise3(lon.cos() * rad * 2.0, y * 2.0, lon.sin() * rad * 2.0) > 0.5;
        let lit = 0.45 + 0.55 * z;
        let (tx, ty) = rot(x, y, 0.4);
        f.plot(center.0 + tx * pr, center.1 + ty * pr, if land { lit } else { lit * 0.5 }, if land { TINT_ACCENT } else { TINT_ICE });
    }
    // Orbiting satellites with trails.
    for k in 0..3 {
        let (ra, rb, tilt, sp) = [(1.05, 0.35, 0.3, 0.7), (1.4, 0.55, -0.5, -0.45), (0.85, 0.85, 0.0, 0.3)][k];
        for tr in 0..30 {
            let ph = t * sp - tr as f32 * 0.02 * sp.signum();
            let (ou, ov) = rot(ph.cos() * ra, ph.sin() * rb, tilt);
            f.plot(ou, ov, if tr == 0 { 1.0 } else { 0.6 - tr as f32 * 0.02 }, TINT_HOT);
        }
    }
    // Blips that flare when the beam passes.
    for i in 0..18usize {
        let ang = rndi(i, 1) * TAU;
        let r = 0.75 + rndi(i, 2) * (a - 0.5);
        let lag = (sweep - ang).rem_euclid(TAU);
        if lag < 2.5 {
            let k = 1.0 - lag / 2.5;
            let p = (ang.cos() * r, ang.sin() * r);
            f.glow(p.0, p.1, 1.0 + 2.0 * k, k, TINT_ICE);
        }
    }
}
