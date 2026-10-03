//! Group E -- Process & Infrastructure. Full-bleed micro-dot shaders; see
//! `dots.rs`.

use super::dots::*;
use super::Ctx;

/// `GitOperation` -- QUANTUM-FILAMENTS: an endless commit graph streams
/// right-to-left: branches fork off the trunk as crackling lightning
/// filaments, run in parallel lanes, and merge back, commit nodes pulsing.
pub fn git_operation(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let seed = c.seed;
    let scroll = t * 0.35;
    let spacing = 0.75;
    let branch = |i: i32| -> (f32, f32, f32) {
        let s = hash(seed ^ (i as u32).wrapping_mul(2_654_435_761));
        let start = i as f32 * spacing + rnd(s) * 0.3;
        let length = 0.9 + rnd(s ^ 1) * 1.6;
        let lane = [-0.7f32, -0.42, 0.42, 0.7][(s % 4) as usize];
        (start, start + length, lane)
    };
    let lane_at = |x: f32| -> Vec<(f32, i32)> {
        let i0 = (x / spacing).floor() as i32;
        let mut out = vec![(0.0, -1)];
        for i in (i0 - 4)..=i0 {
            let (s, e, lane) = branch(i);
            if x >= s && x <= e {
                let k = smoothstep(s, s + 0.3, x) * (1.0 - smoothstep(e - 0.3, e, x));
                out.push((lane * k, i));
            }
        }
        out
    };
    // Per-column lane positions (with filament jitter), computed once.
    let cols: Vec<Vec<(f32, i32)>> = (0..f.w)
        .map(|x| {
            let (u, _) = f.uv(x, 0);
            let wx = u + scroll;
            lane_at(wx)
                .into_iter()
                .map(|(y, id)| (y + (noise3(wx * 9.0, id as f32 * 3.7, t * 4.0) - 0.5) * 0.06, id))
                .collect()
        })
        .collect();
    let px = f.px();
    f.shade(|u, v, x, _| {
        let mut out = (0.0, TINT_ACCENT);
        let n = noise3(u * 30.0 + scroll * 30.0, v * 30.0, 0.0);
        if n > 0.86 {
            out = (0.3, TINT_ICE);
        }
        for &(y, id) in &cols[x] {
            let d = (v - y).abs();
            let core = if id < 0 { 1.6 } else { 1.0 } * px;
            let l = if d < core {
                1.0
            } else {
                // Crackling halo: exponential falloff broken by noise.
                let crackle = noise3(u * 40.0, v * 40.0, t * 10.0 + id as f32);
                0.55 * (-(d - core) * 18.0).exp() * (0.5 + crackle)
            };
            if l > out.0 {
                out = (l, if d < core { TINT_HOT } else if id < 0 { TINT_ACCENT } else { TINT_ICE });
            }
        }
        out
    });
    // Secondary filament strands braiding around each lane.
    for x in 0..f.w {
        let (u, _) = f.uv(x, 0);
        let wx = u + scroll;
        for &(y, id) in &cols[x] {
            for strand in 0..2 {
                let ph = wx * (9.0 + strand as f32 * 4.0) - t * 5.0 + id as f32;
                let off = ph.sin() * 0.05 * (1.0 + noise3(wx * 3.0, strand as f32, t));
                f.plot(u, y + off, 0.7, if strand == 0 { TINT_ACCENT } else { TINT_GLITCH });
            }
        }
    }
    // Commit nodes at fork/merge points and along the trunk.
    let i0 = ((-a + scroll) / spacing).floor() as i32 - 4;
    let i1 = ((a + scroll) / spacing).ceil() as i32;
    for i in i0..=i1 {
        let (s, e, lane) = branch(i);
        for (wx, wy) in [(s, 0.0), (e, 0.0), ((s + e) * 0.5, lane)] {
            let u = wx - scroll;
            if u.abs() > a + 0.1 {
                continue;
            }
            let pulse = fract(t * 1.3 + i as f32 * 0.37);
            f.glow(u, wy, 4.5, 1.0, TINT_HOT);
            f.ring((u, wy), 0.06, 1.0, TINT_ACCENT);
            f.ring((u, wy), 0.05 + pulse * 0.18, 1.0 - pulse, TINT_GLITCH);
        }
    }
    // Lightning arcs jumping between lanes.
    let slot = (t * 3.0) as u32;
    let s = hash(seed ^ slot);
    let u0 = (rnd(s) * 2.0 - 1.0) * a;
    let mut p = (u0, 0.0);
    let target = [-0.7f32, -0.42, 0.42, 0.7][(s % 4) as usize];
    for k in 1..=10 {
        let q = (u0 + (rnd(s ^ k) - 0.5) * 0.2, target * k as f32 / 10.0);
        f.line(p, q, 1.0, TINT_HOT);
        p = q;
    }
}

fn anchor_height(u: f32, v: f32) -> f32 {
    let shank = sd_round_box(u, v - 0.05, 0.075, 0.42, 0.04);
    let stock = sd_round_box(u, v + 0.25, 0.3, 0.06, 0.04);
    let ring = (len(u, v + 0.47) - 0.09).abs() - 0.04;
    let arc_r = len(u, v - 0.0);
    let arm = if v > 0.1 { (arc_r - 0.42).abs() - 0.065 } else { 1.0 };
    let fluke_l = sd_ellipse(u + 0.4, v - 0.12, 0.1, 0.16);
    let fluke_r = sd_ellipse(u - 0.4, v - 0.12, 0.1, 0.16);
    let crown = sd_ellipse(u, v - 0.44, 0.12, 0.08);
    let d = shank.min(stock).min(ring).min(arm).min(fluke_l).min(fluke_r).min(crown);
    if d > 0.0 { 0.0 } else { 0.35 + 0.65 * smoothstep(0.0, 0.07, -d) }
}

/// `DependencyLockUpdate` -- NAUTICAL-ANCHOR: a sculpted anchor swings on a
/// rattling chain through caustic-lit water; dozens of lesser dependency
/// chains hang behind it in the murk, bubbles streaming up.
pub fn dependency_lock_update(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let swing = 0.18 * (t * 1.1).sin();
    let pivot = (0.0f32, -1.1f32);
    let chain_len = 0.78;
    let light = norm3((-0.5, -0.7, 0.55));
    let rattle = t * 0.6;
    f.shade(|u, v, _, _| {
        // Caustics: abs of interfering waves, brighter toward the surface.
        let w = (noise3(u * 4.0, v * 4.0 - t * 0.3, t * 0.5) - 0.5).abs();
        let mut out = if w < 0.018 { (0.3 * (1.0 - (v + 1.0) * 0.3), TINT_ICE) } else { (0.0, TINT_ICE) };
        // Background chains (parallax, dim).
        for k in 0..7 {
            let cx = -a + 0.3 + k as f32 * (2.0 * a - 0.6) / 6.0;
            if cx.abs() < 0.5 {
                continue;
            }
            let sw = 0.05 * (t * 0.9 + k as f32).sin();
            let lu = u - cx - sw * (v + 1.0);
            let link = 0.12;
            let lv = fract((v + rattle * 0.5 + k as f32 * 0.07) / link) * link - link * 0.5;
            let odd = (((v + rattle * 0.5 + k as f32 * 0.07) / link).floor() as i32) % 2 == 0;
            let d = if odd { (len(lu / 0.6, lv) - 0.04).abs() } else { (lu.abs()).max(lv.abs() - 0.05) - 0.005 };
            if d < 0.008 && v < 0.6 + 0.2 * (k as f32).sin() {
                out = (0.4, TINT_ACCENT);
            }
        }
        // Main chain along the swinging axis.
        let (ru, rv) = rot(u - pivot.0, v - pivot.1, -swing);
        if rv > 0.0 && rv < chain_len {
            let link = 0.16;
            let ph = rv - rattle * 0.3;
            let lv = fract(ph / link) * link - link * 0.5;
            let odd = ((ph / link).floor() as i32) % 2 == 0;
            let d = if odd { (len(ru / 0.55, lv) - 0.055).abs() - 0.008 } else { ru.abs().max(lv.abs() - 0.07) - 0.012 };
            if d < 0.0 {
                out = (0.95, TINT_HOT);
            } else if d < 0.012 {
                out = (0.5, TINT_ACCENT);
            }
        }
        let (h, lam) = relief(&anchor_height, ru / 1.25, (rv - chain_len - 0.62) / 1.25, 0.012, 0.9, light);
        if h > 0.0 {
            let l = (0.3 + 0.7 * lam) * (0.55 + 0.45 * h);
            out = (l, if l > 0.85 { TINT_HOT } else { TINT_ACCENT });
        }
        out
    });
    // Bubbles.
    for i in 0..380usize {
        let life = fract(t * (0.12 + rndi(i, 1) * 0.12) + rndi(i, 2));
        let u = (rndi(i, 3) * 2.0 - 1.0) * a + 0.04 * (t * 3.0 + i as f32).sin();
        let v = 1.05 - life * 2.2;
        f.plot(u, v, 0.7, TINT_ICE);
        if i % 5 == 0 {
            f.plot(u, v - f.px(), 0.5, TINT_HOT);
        }
    }
}

/// `TestFileActivity` -- TACTICAL-RETICLE: a swarm of leggy bugs crawls the
/// whole panel; a sniper reticle with full-bleed crosshairs hunts them one
/// by one, locks on, and blows each target apart into dots.
pub fn test_file_activity(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let n_bugs = 26usize;
    let hunt = 2.2;
    let target = ((t / hunt) as usize) % n_bugs;
    let hph = fract(t / hunt);
    let bug_pos = |i: usize, tt: f32| -> (f32, f32) {
        let u = (noise3(i as f32 * 3.1, tt * 0.12, 0.0) * 2.0 - 1.0) * (a - 0.1) * 1.3;
        let v = (noise3(i as f32 * 3.1, tt * 0.12, 5.0) * 2.0 - 1.0) * 1.2;
        (u.clamp(-a + 0.05, a - 0.05), v.clamp(-0.95, 0.95))
    };
    let tgt = bug_pos(target, t);
    let prev = bug_pos((target + n_bugs - 1) % n_bugs, t);
    let ease = smoothstep(0.0, 0.45, hph);
    let ret = (mix(prev.0, tgt.0, ease), mix(prev.1, tgt.1, ease));
    let locked = hph > 0.5;
    let boom = hph > 0.72;
    let px = f.px();
    f.shade(|u, v, x, y| {
        // Tactical grid.
        let g = 0.25;
        let (gu, gv) = (fract(u / g), fract(v / g));
        if (gu < px / g || gv < px / g) && (x + y) % 3 == 0 {
            return (0.18, TINT_ACCENT);
        }
        // Full-bleed crosshair, dashed, with a gap at the reticle.
        let du = (u - ret.0).abs();
        let dv = (v - ret.1).abs();
        if (du < px * 0.6 && dv > 0.42) || (dv < px * 0.6 && du > 0.42) {
            if ((du + dv) * 30.0).fract() < 0.6 {
                return (0.7, if locked { TINT_GLITCH } else { TINT_HOT });
            }
        }
        (0.0, TINT_ACCENT)
    });
    // Bugs.
    for i in 0..n_bugs {
        if i == target && boom {
            continue;
        }
        let p = bug_pos(i, t);
        let q = bug_pos(i, t + 0.05);
        let ang = (q.1 - p.1).atan2(q.0 - p.0);
        let (ca, sa) = (ang.cos(), ang.sin());
        let tr = |du: f32, dv: f32| (p.0 + du * ca - dv * sa, p.1 + du * sa + dv * ca);
        for k in -5..=5 {
            let du = k as f32 * 0.012;
            let w = 0.05 * (1.0 - (k as f32 / 6.0).powi(2));
            f.line(tr(du, -w), tr(du, w), 0.9, if i == target { TINT_GLITCH } else { TINT_ACCENT });
        }
        for leg in 0..3 {
            let lu = (leg as f32 - 1.0) * 0.04;
            let wig = 0.03 * (t * 14.0 + leg as f32 * 2.0 + i as f32).sin();
            f.line(tr(lu, 0.04), tr(lu + wig, 0.1), 0.75, TINT_ACCENT);
            f.line(tr(lu, -0.04), tr(lu - wig, -0.1), 0.75, TINT_ACCENT);
        }
        f.line(tr(0.06, 0.01), tr(0.12, 0.05), 0.6, TINT_HOT);
        f.line(tr(0.06, -0.01), tr(0.12, -0.05), 0.6, TINT_HOT);
    }
    // Reticle.
    let spin = t * 0.8;
    let rr = if locked { 0.22 } else { 0.32 - 0.05 * (t * 4.0).sin() };
    f.arc(ret, rr, spin, spin + 1.2, 1.0, TINT_HOT);
    f.arc(ret, rr, spin + PI, spin + PI + 1.2, 1.0, TINT_HOT);
    f.arc(ret, rr * 1.35, -spin, -spin + 0.7, 0.8, TINT_GLITCH);
    f.arc(ret, rr * 1.35, -spin + PI, -spin + PI + 0.7, 0.8, TINT_GLITCH);
    f.ring(ret, rr * 0.5, 0.7, TINT_ACCENT);
    for k in 0..36 {
        let ang = k as f32 / 36.0 * TAU;
        let r1 = rr * if k % 9 == 0 { 0.7 } else { 0.85 };
        f.line((ret.0 + ang.cos() * r1, ret.1 + ang.sin() * r1), (ret.0 + ang.cos() * rr, ret.1 + ang.sin() * rr), 0.6, TINT_ACCENT);
    }
    f.glow(ret.0, ret.1, 1.5, 1.0, TINT_GLITCH);
    if boom {
        let k = (hph - 0.72) / 0.28;
        f.ring(tgt, k * 0.8, 1.0 - k, TINT_HOT);
        for i in 0..260usize {
            let ang = rndi(i, target as u32) * TAU;
            let sp = 0.2 + rndi(i, 9) * 0.9;
            f.plot(tgt.0 + ang.cos() * sp * k, tgt.1 + ang.sin() * sp * k + k * k * 0.3, 1.0 - k, if i % 3 == 0 { TINT_HOT } else { TINT_GLITCH });
        }
    }
}

fn vault_height(u: f32, v: f32, wheel: f32, bolts: f32, dial: f32) -> f32 {
    let r = len(u, v);
    let th = v.atan2(u);
    if r > 0.98 {
        return 0.0;
    }
    let mut h = 0.5;
    if r > 0.86 {
        h = 0.9 - (r - 0.92).abs() * 4.0;
    } else if r > 0.82 {
        h = 0.2;
    }
    // Bolts around the rim, sliding radially.
    for k in 0..12 {
        let ang = k as f32 / 12.0 * TAU;
        let (bu, bv) = rot(u, v, -ang);
        let br = 0.7 + bolts * 0.12;
        if (bu - br).abs() < 0.06 && bv.abs() < 0.035 {
            h = 1.0;
        }
    }
    // Combination dial ticks.
    if (0.55..0.66).contains(&r) {
        let tick = fract((th - dial) / TAU * 60.0);
        h = if tick < 0.2 { 1.0 } else { 0.35 };
    }
    // Wheel: hub + three spokes.
    let (wu, wv) = rot(u, v, -wheel);
    if r < 0.5 {
        h = 0.3;
        for k in 0..3 {
            let (su, sv) = rot(wu, wv, -(k as f32) * TAU / 3.0);
            if su > 0.0 && su < 0.48 && sv.abs() < 0.04 {
                h = 0.95;
            }
            if (len(su - 0.48, sv) - 0.06).abs() < 0.02 {
                h = 1.0;
            }
        }
        if r < 0.13 {
            h = 1.0 - r * 3.0;
        }
    }
    h
}

/// `EnvSecretChange` -- BULKHEAD-VAULT: a sculpted bank-vault door fills the
/// panel; its wheel spins, bolts shoot and retract, the dial hunts the
/// combination, while red laser tripwires sweep the room and encrypted bit
/// streams pour in from both sides.
pub fn env_secret_change(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let wheel = t * 0.9 + (t * 0.5).sin() * 2.0;
    let bolts = 0.5 + 0.5 * (t * 0.8).sin();
    let dial = (t * 0.7).sin() * 3.0;
    let light = norm3((-0.45, -0.65, 0.6));
    let alarm = fract(t / 2.4) < 0.08;
    let seed = c.seed;
    f.shade(|u, v, x, y| {
        let mut out = (0.0, TINT_ACCENT);
        // Bit streams flowing toward the door from the flanks.
        if u.abs() > 1.0 {
            let row = y / 3;
            let hs = hash(row as u32 ^ seed);
            if hs % 2 == 0 && y % 3 == 1 {
                let dir = if u < 0.0 { 1.0 } else { -1.0 };
                let ph = (x as f32 * 0.5 - dir * t * (12.0 + rnd(hs) * 18.0)) as i32;
                let bit = hash(ph as u32 ^ hs) % 3 == 0;
                if bit {
                    out = (0.5, if hs % 5 == 0 { TINT_ICE } else { TINT_ACCENT });
                }
            }
        }
        // Laser tripwires.
        for k in 0..3 {
            let ang = 0.5 + k as f32 * 0.9 + 0.3 * (t * 0.6 + k as f32).sin();
            let (_, dv) = rot(u - (k as f32 - 1.0) * a * 0.6, v, -ang);
            if dv.abs() < 0.008 {
                out = (1.0, TINT_GLITCH);
            } else if dv.abs() < 0.025 && out.0 < 0.3 {
                out = (0.3, TINT_GLITCH);
            }
        }
        let (h, lam) = relief(&|p, q| vault_height(p, q, wheel, bolts, dial), u, v, 0.012, 1.4, light);
        if h > 0.0 {
            let l = (0.12 + 0.88 * lam) * (0.45 + 0.55 * h);
            out = (l, if alarm && h > 0.8 { TINT_GLITCH } else if l > 0.88 { TINT_HOT } else { TINT_ACCENT });
        }
        out
    });
}

/// `CiPipelineEdit` -- ROBOTIC-GANTRY: a conveyor of build artifacts runs the
/// full width; articulated robot arms dive, weld, and lift, spraying sparks,
/// while trolleys shuttle along an overhead rail.
pub fn ci_pipeline_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let belt_v = 0.55;
    let belt_speed = 0.35;
    let light = norm3((-0.4, -0.8, 0.5));
    let px = f.px();
    f.shade(|u, v, _, _| {
        let mut out = (0.0, TINT_ACCENT);
        // Overhead rail.
        if (v + 0.9).abs() < px * 1.2 {
            out = (0.8, TINT_ACCENT);
        }
        if (v + 0.86).abs() < px * 0.6 && fract(u * 8.0) < 0.5 {
            out = (0.4, TINT_ACCENT);
        }
        // Belt with rollers.
        let bv = v - belt_v;
        if bv > 0.0 && bv < 0.1 {
            let tread = fract((u + t * belt_speed) * 12.0) < 0.5;
            out = (if bv < 0.02 || bv > 0.08 { 0.9 } else if tread { 0.45 } else { 0.15 }, TINT_ACCENT);
        }
        let rx = fract((u + a) / 0.3) * 0.3 - 0.15;
        let rr = len(rx, v - belt_v - 0.17);
        if rr < 0.06 {
            let spoke = ((v - belt_v - 0.17).atan2(rx) + t * belt_speed / 0.06 * 0.3).sin() > 0.7;
            out = (if rr > 0.045 || spoke { 0.8 } else { 0.2 }, TINT_ICE);
        }
        // Packages riding the belt.
        let pk = (u + t * belt_speed).rem_euclid(0.55) - 0.275;
        let idx = ((u + t * belt_speed) / 0.55).floor() as i32;
        let hgt = 0.12 + rnd(idx as u32) * 0.1;
        let (h, lam) = relief(&|p, q| if p.abs() < 0.13 && q > -hgt && q < 0.0 { 0.6 + 0.4 * smoothstep(0.13, 0.1, p.abs()) } else { 0.0 }, pk, bv, 0.01, 2.0, light);
        if h > 0.0 {
            let stamp = (pk.abs() < 0.04 && (bv + hgt * 0.5).abs() < 0.03) && idx % 2 == 0;
            out = (if stamp { 1.0 } else { 0.2 + 0.7 * lam }, if stamp { TINT_GLITCH } else { TINT_ACCENT });
        }
        out
    });
    // Arms: 2-link IK from the rail to a target bobbing over the belt.
    let arms = ((2.0 * a / 1.1) as usize).max(2);
    for k in 0..arms {
        let base = (-a + (k as f32 + 0.5) * 2.0 * a / arms as f32, -0.86);
        let cyc = fract(t * 0.45 + k as f32 * 0.31);
        let dip = (cyc * TAU).sin().max(0.0);
        let target = (base.0 + 0.25 * (t * 0.7 + k as f32).sin(), belt_v - 0.25 + 0.2 * (1.0 - dip) * -1.0);
        let (l1, l2) = (0.6, 0.6);
        let (dx, dy) = (target.0 - base.0, target.1 - base.1);
        let d = len(dx, dy).min(l1 + l2 - 0.01);
        let a1 = dy.atan2(dx) - ((l1 * l1 + d * d - l2 * l2) / (2.0 * l1 * d)).clamp(-1.0, 1.0).acos();
        let elbow = (base.0 + a1.cos() * l1, base.1 + a1.sin() * l1);
        for o in -2..=2 {
            let off = o as f32 * px;
            f.line((base.0 + off, base.1), (elbow.0 + off, elbow.1), 0.9, TINT_HOT);
            f.line((elbow.0 + off, elbow.1), (target.0 + off, target.1), 0.75, TINT_ACCENT);
        }
        f.glow(elbow.0, elbow.1, 3.0, 1.0, TINT_ICE);
        f.glow(base.0, base.1, 4.0, 1.0, TINT_ACCENT);
        // Gripper claws.
        let open = 0.04 + 0.03 * dip;
        f.line(target, (target.0 - open, target.1 + 0.08), 1.0, TINT_HOT);
        f.line(target, (target.0 + open, target.1 + 0.08), 1.0, TINT_HOT);
        if dip > 0.6 {
            for i in 0..70usize {
                let life = fract(t * 2.0 + rndi(i, k as u32));
                let ang = -PI * 0.5 + (rndi(i, 77) - 0.5) * 2.4;
                let sp = 0.2 + rndi(i, 78) * 0.5;
                let u = target.0 + ang.cos() * sp * life;
                let v = target.1 + 0.08 + ang.sin() * sp * life + life * life * 0.9;
                f.plot(u, v, 1.0 - life, if life < 0.3 { TINT_HOT } else { TINT_GLITCH });
            }
        }
    }
    // Overhead trolleys.
    for k in 0..3 {
        let u = ((t * (0.2 + k as f32 * 0.07) + k as f32 * 0.4).rem_euclid(1.0) * 2.0 - 1.0) * a;
        for dy in 0..4 {
            f.line((u - 0.08, -0.95 + dy as f32 * px), (u + 0.08, -0.95 + dy as f32 * px), 0.9, TINT_ICE);
        }
    }
}

/// `ContainerConfigEdit` -- LEVIATHAN-CARGO: a colossal whale stacked with
/// shipping containers swims across a moonlit, caustic-lit ocean, breaching
/// to blow a spout of dots before diving again, bubbles trailing.
pub fn container_config_edit(f: &mut DotField, c: &Ctx) {
    let t = c.t;
    let a = f.aspect();
    let cycle = 13.0;
    let ph = fract(t / cycle);
    let wx = -a - 1.4 + ph * (2.0 * a + 2.8);
    let wy = 0.25 - 0.32 * (ph * PI * 2.0).sin().max(0.0);
    let surf = |u: f32| -0.18 + 0.04 * (u * 3.0 + t * 1.2).sin() + 0.025 * (u * 7.0 - t * 1.7).sin();
    let light = norm3((-0.4, -0.8, 0.45));
    let tail_flap = (t * 2.2).sin() * 0.12;
    let whale = move |u: f32, v: f32| -> f32 {
        let body = sd_ellipse(u, v, 0.85, 0.24);
        let head = sd_ellipse(u - 0.45, v - 0.02, 0.5, 0.26);
        let tu = u + 0.95;
        let tv = v - tail_flap * (tu.abs() * 4.0).min(1.0);
        let stalk = sd_round_box(tu + 0.05, tv, 0.18, 0.06, 0.04);
        let fluke = sd_ellipse(tu + 0.25, tv, 0.08, 0.22);
        let fin = sd_ellipse(u - 0.15, v - 0.24, 0.16, 0.05);
        let d = smin(smin(smin(body, head, 0.15), smin(stalk, fluke, 0.06), 0.08), fin, 0.05);
        if d > 0.0 {
            return 0.0;
        }
        let mut h = smoothstep(0.0, 0.18, -d);
        if v > 0.08 && u > -0.4 && u < 0.85 && (fract(v * 45.0) < 0.3) {
            h *= 0.6; // throat grooves
        }
        if len(u - 0.62, v - 0.03) < 0.025 {
            h = 0.02; // eye
        }
        h
    };
    f.shade(|u, v, x, y| {
        let s = surf(u);
        let mut out = if v < s {
            // Sky: stars and a dithered moon.
            let md = len(u - a * 0.6, v + 0.65);
            if md < 0.18 {
                let crater = noise3(u * 12.0, v * 12.0, 0.0);
                (0.75 + 0.25 * crater - smoothstep(0.1, 0.18, md) * 0.4, TINT_HOT)
            } else if hash((x as u32) ^ ((y as u32) << 12)) % 211 == 0 && noise3(x as f32, y as f32, t * 2.0) > 0.4 {
                (0.8, TINT_HOT)
            } else {
                (0.0, TINT_ACCENT)
            }
        } else {
            let depth = v - s;
            let cau = (noise3(u * 5.0, v * 5.0 + t * 0.2, t * 0.6) - 0.5).abs();
            let l = if cau < 0.03 { 0.45 } else { 0.0 } * (1.0 - depth * 0.6).max(0.15);
            (l, TINT_ICE)
        };
        if (v - s).abs() < 0.012 {
            out = (0.9, TINT_ICE);
        }
        // Moon glitter on the water.
        if v > s && v < s + 0.25 && (u - a * 0.6).abs() < 0.2 && noise3(u * 30.0, v * 60.0, t * 3.0) > 0.7 {
            out = (0.9, TINT_HOT);
        }
        let (h, lam) = relief(&whale, u - wx, v - wy, 0.012, 1.6, light);
        if h > 0.0 {
            let under = v > s;
            let l = (0.12 + 0.88 * lam) * (0.45 + 0.55 * h) * if under { 0.8 } else { 1.0 };
            out = (l, if l > 0.85 { TINT_HOT } else { TINT_ACCENT });
        }
        // Containers on the whale's back.
        for k in 0..5 {
            let cu = u - wx - (-0.45 + k as f32 * 0.2);
            let level = if k % 2 == 0 { 2 } else { 1 };
            for lv in 0..level {
                let cv = v - wy + 0.3 + lv as f32 * 0.1;
                if cu.abs() < 0.09 && cv.abs() < 0.045 {
                    let corr = fract(cu * 60.0) < 0.35;
                    let edge = cu.abs() > 0.08 || cv.abs() > 0.035;
                    let tint = [TINT_GLITCH, TINT_ICE, TINT_ACCENT, TINT_HOT][(k + lv) % 4];
                    out = (if edge { 1.0 } else if corr { 0.75 } else { 0.4 }, tint);
                }
            }
        }
        out
    });
    // Spout when breaching.
    let breach = (ph * PI * 2.0).sin();
    if breach > 0.5 {
        let base = (wx + 0.55, wy - 0.27);
        for i in 0..420usize {
            let life = fract(t * 1.1 + rndi(i, 3));
            let ang = -PI * 0.5 + (rndi(i, 4) - 0.5) * 0.9;
            let sp = 0.7 + rndi(i, 5) * 0.5;
            let u = base.0 + ang.cos() * sp * life * 0.8;
            let v = base.1 + ang.sin() * sp * life + life * life * 0.9;
            f.plot(u, v, (1.0 - life) * (breach - 0.5) * 2.0, if i % 3 == 0 { TINT_HOT } else { TINT_ICE });
        }
    }
    for i in 0..200usize {
        let life = fract(t * 0.5 + rndi(i, 7));
        let u = wx - 1.1 - rndi(i, 8) * 0.8 + 0.03 * (t * 4.0 + i as f32).sin();
        let v = wy + 0.1 - life * 0.6;
        if v > surf(u) {
            f.plot(u, v, 0.8 * (1.0 - life), TINT_ICE);
        }
    }
}
