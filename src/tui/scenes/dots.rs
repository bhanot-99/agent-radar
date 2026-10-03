//! Braille micro-dot engine.
//!
//! Every terminal cell is a 2x4 grid of Braille sub-pixels (U+2800..U+28FF),
//! so a 120x26 hero panel is a 240x104 = ~25k-dot framebuffer. Scenes paint
//! a continuous luminance field into it (shaders, particles, lines); the
//! blitter then applies 4x4 Bayer ordered dithering -- the same 1-bit
//! halftone grain as a dithered marble bust -- and packs each 2x4 block into
//! one Braille glyph colored by its dominant tint and mean luminance.
//!
//! Coordinates: scenes work in `uv` space, centered on the panel, with `v`
//! spanning -1 (top) .. +1 (bottom) and `u` spanning -aspect .. +aspect.
//! Dots are square-ish on a typical terminal font, so circles stay round.

use ratatui::style::{Color, Style};

use super::SceneCanvas;
use crate::tui::theme::BG_BASE;

pub const TINT_ACCENT: u8 = 0;
pub const TINT_HOT: u8 = 1;
pub const TINT_GLITCH: u8 = 2;
pub const TINT_ICE: u8 = 3;

pub const PI: f32 = std::f32::consts::PI;
pub const TAU: f32 = std::f32::consts::TAU;

pub struct DotField {
    pub w: usize,
    pub h: usize,
    half_h: f32,
    lum: Vec<f32>,
    tint: Vec<u8>,
}

impl DotField {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            w,
            h,
            half_h: (h as f32 * 0.5).max(1.0),
            lum: vec![0.0; w * h],
            tint: vec![TINT_ACCENT; w * h],
        }
    }

    /// Half-width of the field in uv units (`u` spans -aspect..aspect).
    pub fn aspect(&self) -> f32 {
        self.w as f32 / self.h.max(1) as f32
    }

    /// Size of one dot in uv units -- the minimum visible line width.
    pub fn px(&self) -> f32 {
        1.0 / self.half_h
    }

    #[inline]
    pub fn uv(&self, x: usize, y: usize) -> (f32, f32) {
        (
            (x as f32 + 0.5 - self.w as f32 * 0.5) / self.half_h,
            (y as f32 + 0.5 - self.h as f32 * 0.5) / self.half_h,
        )
    }

    #[inline]
    pub fn to_dot(&self, u: f32, v: f32) -> (f32, f32) {
        (u * self.half_h + self.w as f32 * 0.5, v * self.half_h + self.h as f32 * 0.5)
    }

    /// Runs a per-dot shader over the whole field: `f(u, v, x, y)` returns
    /// `(luminance 0..1, tint)`. Max-blended with whatever is already there.
    pub fn shade<F: FnMut(f32, f32, usize, usize) -> (f32, u8)>(&mut self, mut f: F) {
        for y in 0..self.h {
            for x in 0..self.w {
                let (u, v) = self.uv(x, y);
                let (l, t) = f(u, v, x, y);
                let i = y * self.w + x;
                if l > self.lum[i] {
                    self.lum[i] = l;
                    self.tint[i] = t;
                }
            }
        }
    }

    #[inline]
    pub fn dot(&mut self, x: i32, y: i32, l: f32, t: u8) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let i = y as usize * self.w + x as usize;
        if l > self.lum[i] {
            self.lum[i] = l;
            self.tint[i] = t;
        }
    }

    /// Forces a dot dark (used for occlusion, e.g. ridge-line fills).
    #[inline]
    pub fn erase(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        self.lum[y as usize * self.w + x as usize] = 0.0;
    }

    #[inline]
    pub fn plot(&mut self, u: f32, v: f32, l: f32, t: u8) {
        let (x, y) = self.to_dot(u, v);
        self.dot(x.floor() as i32, y.floor() as i32, l, t);
    }

    /// A soft round blob `r` dots across, brightest at the center.
    pub fn glow(&mut self, u: f32, v: f32, r: f32, l: f32, t: u8) {
        let (cx, cy) = self.to_dot(u, v);
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d <= r {
                    let k = 1.0 - d / (r + 1.0);
                    self.dot(cx as i32 + dx, cy as i32 + dy, l * k, t);
                }
            }
        }
    }

    pub fn line(&mut self, a: (f32, f32), b: (f32, f32), l: f32, t: u8) {
        let (x0, y0) = self.to_dot(a.0, a.1);
        let (x1, y1) = self.to_dot(b.0, b.1);
        let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil().max(1.0) as i32;
        if steps > 4096 {
            return;
        }
        for s in 0..=steps {
            let k = s as f32 / steps as f32;
            self.dot(mix(x0, x1, k).floor() as i32, mix(y0, y1, k).floor() as i32, l, t);
        }
    }

    /// Line with luminance varying along it: `lf(k)` for k in 0..1.
    pub fn line_fn<F: Fn(f32) -> (f32, u8)>(&mut self, a: (f32, f32), b: (f32, f32), lf: F) {
        let (x0, y0) = self.to_dot(a.0, a.1);
        let (x1, y1) = self.to_dot(b.0, b.1);
        let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil().max(1.0) as i32;
        if steps > 4096 {
            return;
        }
        for s in 0..=steps {
            let k = s as f32 / steps as f32;
            let (l, t) = lf(k);
            self.dot(mix(x0, x1, k).floor() as i32, mix(y0, y1, k).floor() as i32, l, t);
        }
    }

    pub fn ring(&mut self, c: (f32, f32), r: f32, l: f32, t: u8) {
        self.arc(c, r, 0.0, TAU, l, t);
    }

    pub fn arc(&mut self, c: (f32, f32), r: f32, a0: f32, a1: f32, l: f32, t: u8) {
        let n = ((a1 - a0).abs() * r * self.half_h * 1.6).ceil().clamp(4.0, 8192.0) as usize;
        for i in 0..=n {
            let a = mix(a0, a1, i as f32 / n as f32);
            self.plot(c.0 + a.cos() * r, c.1 + a.sin() * r, l, t);
        }
    }

    /// Displaces a row of dots horizontally (wrapping) -- the core of the
    /// DedSec slice-glitch.
    pub fn shift_row(&mut self, y: usize, dx: i32) {
        if y >= self.h || self.w == 0 {
            return;
        }
        let row = y * self.w..(y + 1) * self.w;
        let s = dx.rem_euclid(self.w as i32) as usize;
        self.lum[row.clone()].rotate_right(s);
        self.tint[row].rotate_right(s);
    }

    pub fn retint_row(&mut self, y: usize, t: u8) {
        if y >= self.h {
            return;
        }
        for i in y * self.w..(y + 1) * self.w {
            if self.lum[i] > 0.2 {
                self.tint[i] = t;
            }
        }
    }

    pub fn boost_row(&mut self, y: usize, add: f32) {
        if y >= self.h {
            return;
        }
        for i in y * self.w..(y + 1) * self.w {
            if self.lum[i] > 0.05 {
                self.lum[i] = (self.lum[i] + add).min(1.0);
            }
        }
    }

    pub fn lit_count(&self) -> usize {
        self.lum.iter().filter(|l| **l > 0.0).count()
    }
}

const BAYER4: [f32; 16] = [
    0.5 / 16.0, 8.5 / 16.0, 2.5 / 16.0, 10.5 / 16.0,
    12.5 / 16.0, 4.5 / 16.0, 14.5 / 16.0, 6.5 / 16.0,
    3.5 / 16.0, 11.5 / 16.0, 1.5 / 16.0, 9.5 / 16.0,
    15.5 / 16.0, 7.5 / 16.0, 13.5 / 16.0, 5.5 / 16.0,
];

/// Braille bit for sub-pixel (dx, dy) of a 2x4 cell.
const BRAILLE_BIT: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

#[derive(Clone, Copy)]
pub struct Palette {
    pub tints: [(u8, u8, u8); 4],
}

impl Palette {
    pub fn dedsec(accent: Color) -> Self {
        Self {
            tints: [
                rgb_of(accent),
                (0xF4, 0xF7, 0xFF),
                (0xFF, 0x2A, 0x6D),
                (0x00, 0xE5, 0xFF),
            ],
        }
    }
}

pub fn rgb_of(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0xC0, 0xC0, 0xC0),
    }
}

/// Dithers the field and writes one Braille glyph per cell into `canvas`.
/// Returns the number of lit sub-pixel dots actually drawn.
pub fn blit(field: &DotField, canvas: &mut SceneCanvas, pal: &Palette) -> usize {
    let cols = (field.w / 2).min(canvas.width() as usize);
    let rows = (field.h / 4).min(canvas.height() as usize);
    let mut total = 0usize;
    for row in 0..rows {
        for col in 0..cols {
            let mut bits = 0u8;
            let mut tint_w = [0f32; 4];
            let mut lsum = 0f32;
            let mut n = 0u32;
            for (dy, bit_row) in BRAILLE_BIT.iter().enumerate() {
                let y = row * 4 + dy;
                for (dx, bit) in bit_row.iter().enumerate() {
                    let x = col * 2 + dx;
                    let i = y * field.w + x;
                    // Tone curve lifts midtones so sculpted surfaces read solid.
                    let l = field.lum[i];
                    let l = l * (1.5 - 0.5 * l);
                    if l > BAYER4[(y & 3) * 4 + (x & 3)] {
                        bits |= bit;
                        tint_w[field.tint[i] as usize & 3] += l;
                        lsum += l;
                        n += 1;
                    }
                }
            }
            if bits == 0 {
                continue;
            }
            total += n as usize;
            let mut best = 0;
            for k in 1..4 {
                if tint_w[k] > tint_w[best] {
                    best = k;
                }
            }
            let mean = lsum / n as f32;
            let k = 0.32 + 0.68 * mean.min(1.0);
            let (r, g, b) = pal.tints[best];
            let fg = Color::Rgb(
                (r as f32 * k) as u8,
                (g as f32 * k) as u8,
                (b as f32 * k) as u8,
            );
            let ch = char::from_u32(0x2800 + bits as u32).unwrap_or(' ');
            canvas.put_char(col as u16, row as u16, ch, Style::default().fg(fg).bg(BG_BASE));
        }
    }
    total
}

// ---------------------------------------------------------------------------
// DedSec post-processing: slice glitches, chroma bleed, rolling scanline,
// dead-pixel sparkle. Deterministic in (t, seed) so tests are reproducible.
// ---------------------------------------------------------------------------

/// Glitch bursts fire in ~1.7s slots; returns burst intensity 0..1 at `t`.
pub fn glitch_burst(t: f32, seed: u32) -> f32 {
    let slot = (t / 1.7).floor();
    let ph = t / 1.7 - slot;
    let s = hash(seed ^ (slot as i32 as u32).wrapping_mul(0x9E37_79B9));
    if rnd(s) < 0.6 && ph < 0.16 {
        1.0 - ph / 0.16
    } else {
        0.0
    }
}

pub fn dedsec_fx(f: &mut DotField, t: f32, seed: u32) {
    let h = f.h;
    let w = f.w;
    if h == 0 || w == 0 {
        return;
    }
    let burst = glitch_burst(t, seed);
    if burst > 0.0 {
        let slot = (t / 1.7).floor() as i32 as u32;
        let frame_jit = (t * 25.0) as u32;
        let bands = 2 + hash(seed ^ slot) % 4;
        for b in 0..bands {
            let s = hash(seed ^ slot.wrapping_mul(31) ^ b.wrapping_mul(7919) ^ frame_jit);
            let y0 = (rnd(s) * h as f32) as usize;
            let bh = 1 + (rnd(s ^ 0xABCD) * h as f32 * 0.12) as usize;
            let dx = ((rnd(s ^ 0x1234) - 0.5) * w as f32 * 0.22 * burst) as i32;
            let tint = if b % 2 == 0 { TINT_GLITCH } else { TINT_ICE };
            for y in y0..(y0 + bh).min(h) {
                f.shift_row(y, dx);
                f.retint_row(y, tint);
            }
        }
    }
    // Rolling CRT scanline.
    let sy = (fract(t * 0.21) * (h as f32 + 20.0)) as usize;
    if sy < h {
        f.boost_row(sy, 0.28);
    }
    // Dead-pixel sparkle: a sprinkle of hot dots each frame.
    let fr = (t * 25.0) as u32;
    let n = (w * h) / 300;
    for i in 0..n as u32 {
        let s = hash(seed ^ fr.wrapping_mul(0x85EB_CA6B) ^ i.wrapping_mul(0xC2B2_AE35));
        let x = (s % w as u32) as i32;
        let y = ((s >> 12) % h as u32) as i32;
        f.dot(x, y, 0.55 + rnd(s ^ 5) * 0.45, if s & 64 == 0 { TINT_HOT } else { TINT_ICE });
    }
}

// ---------------------------------------------------------------------------
// Math / noise toolkit
// ---------------------------------------------------------------------------

#[inline]
pub fn hash(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// Uniform 0..1 from an integer seed.
#[inline]
pub fn rnd(x: u32) -> f32 {
    (hash(x) >> 8) as f32 / 16_777_216.0
}

#[inline]
pub fn rndi(i: usize, salt: u32) -> f32 {
    rnd((i as u32).wrapping_mul(0x9E37_79B9) ^ salt.wrapping_mul(0x85EB_CA6B))
}

#[inline]
fn h3(x: i32, y: i32, z: i32) -> f32 {
    rnd((x as u32).wrapping_mul(0x8DA6_B343)
        ^ (y as u32).wrapping_mul(0xD816_3841)
        ^ (z as u32).wrapping_mul(0xCB1A_B31F))
}

/// Smooth 3D value noise in 0..1.
pub fn noise3(x: f32, y: f32, z: f32) -> f32 {
    let (xi, yi, zi) = (x.floor(), y.floor(), z.floor());
    let (xf, yf, zf) = (x - xi, y - yi, z - zi);
    let (xi, yi, zi) = (xi as i32, yi as i32, zi as i32);
    let (sx, sy, sz) = (xf * xf * (3.0 - 2.0 * xf), yf * yf * (3.0 - 2.0 * yf), zf * zf * (3.0 - 2.0 * zf));
    let l = |dx, dy, dz| h3(xi + dx, yi + dy, zi + dz);
    let x00 = mix(l(0, 0, 0), l(1, 0, 0), sx);
    let x10 = mix(l(0, 1, 0), l(1, 1, 0), sx);
    let x01 = mix(l(0, 0, 1), l(1, 0, 1), sx);
    let x11 = mix(l(0, 1, 1), l(1, 1, 1), sx);
    mix(mix(x00, x10, sy), mix(x01, x11, sy), sz)
}

pub fn fbm3(x: f32, y: f32, z: f32, oct: u32) -> f32 {
    let (mut a, mut f, mut s, mut n) = (0.5, 1.0, 0.0, 0.0);
    for _ in 0..oct {
        s += a * noise3(x * f, y * f, z * f);
        n += a;
        a *= 0.5;
        f *= 2.03;
    }
    s / n
}

#[inline]
pub fn mix(a: f32, b: f32, k: f32) -> f32 {
    a + (b - a) * k
}

#[inline]
pub fn fract(x: f32) -> f32 {
    x - x.floor()
}

#[inline]
pub fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let k = clamp01((x - e0) / (e1 - e0));
    k * k * (3.0 - 2.0 * k)
}

#[inline]
pub fn rot(u: f32, v: f32, a: f32) -> (f32, f32) {
    let (s, c) = a.sin_cos();
    (u * c - v * s, u * s + v * c)
}

#[inline]
pub fn len(u: f32, v: f32) -> f32 {
    (u * u + v * v).sqrt()
}

pub fn norm3(v: (f32, f32, f32)) -> (f32, f32, f32) {
    let l = (v.0 * v.0 + v.1 * v.1 + v.2 * v.2).sqrt().max(1e-6);
    (v.0 / l, v.1 / l, v.2 / l)
}

/// Approximate signed distance to an axis-aligned ellipse.
#[inline]
pub fn sd_ellipse(u: f32, v: f32, a: f32, b: f32) -> f32 {
    (len(u / a, v / b) - 1.0) * a.min(b)
}

#[inline]
pub fn sd_round_box(u: f32, v: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = u.abs() - hw + r;
    let qy = v.abs() - hh + r;
    len(qx.max(0.0), qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

#[inline]
pub fn sd_segment(u: f32, v: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (pax, pay) = (u - a.0, v - a.1);
    let (bax, bay) = (b.0 - a.0, b.1 - a.1);
    let h = clamp01((pax * bax + pay * bay) / (bax * bax + bay * bay).max(1e-9));
    len(pax - bax * h, pay - bay * h)
}

/// Polynomial smooth-min (soft union of two SDFs).
#[inline]
pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    let h = clamp01(0.5 + 0.5 * (b - a) / k);
    mix(b, a, h) - k * h * (1.0 - h)
}

/// Sculpted-relief lighting for a height field: returns (height, lambert).
/// This is what gives shapes the stippled-marble look -- surfaces facing the
/// light dither dense, surfaces turned away dissolve into sparse grain.
pub fn relief<F: Fn(f32, f32) -> f32>(hf: &F, u: f32, v: f32, e: f32, k: f32, light: (f32, f32, f32)) -> (f32, f32) {
    let h = hf(u, v);
    if h <= 0.0 {
        return (0.0, 0.0);
    }
    let hx = hf(u + e, v);
    let hy = hf(u, v + e);
    let n = norm3(((h - hx) / e * k, (h - hy) / e * k, 1.0));
    let lam = (n.0 * light.0 + n.1 * light.1 + n.2 * light.2).max(0.0);
    (h, lam)
}

/// Perspective-projects a 3D point (camera at z = -dist) to uv + depth scale.
#[inline]
pub fn project(x: f32, y: f32, z: f32, dist: f32, scale: f32) -> (f32, f32, f32) {
    let k = dist / (dist + z).max(0.05);
    (x * k * scale, y * k * scale, k)
}

// ---------------------------------------------------------------------------
// 5x7 bitmap font, sampled as a smooth field so glyphs can be drawn huge and
// dithered/lit like sculpture.
// ---------------------------------------------------------------------------

fn glyph_rows(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '<' => [0b00010, 0b00100, 0b01000, 0b10000, 0b01000, 0b00100, 0b00010],
        '>' => [0b01000, 0b00100, 0b00010, 0b00001, 0b00010, 0b00100, 0b01000],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '$' => [0b00100, 0b01111, 0b10100, 0b01110, 0b00101, 0b11110, 0b00100],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        '{' => [0b00010, 0b00100, 0b00100, 0b01000, 0b00100, 0b00100, 0b00010],
        '}' => [0b01000, 0b00100, 0b00100, 0b00010, 0b00100, 0b00100, 0b01000],
        '#' => [0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010],
        '&' => [0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101],
        '@' => [0b01110, 0b10001, 0b00001, 0b01101, 0b10101, 0b10101, 0b01110],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b00000, 0b00100],
        '*' => [0b00000, 0b00100, 0b10101, 0b01110, 0b10101, 0b00100, 0b00000],
        '=' => [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        ':' => [0b00000, 0b01100, 0b01100, 0b00000, 0b01100, 0b01100, 0b00000],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        _ => [0; 7],
    }
}

#[inline]
fn glyph_px(rows: &[u8; 7], x: i32, y: i32) -> f32 {
    if !(0..5).contains(&x) || !(0..7).contains(&y) {
        return 0.0;
    }
    if rows[y as usize] & (0b10000 >> x) != 0 { 1.0 } else { 0.0 }
}

/// Bilinear sample of a glyph at continuous glyph-space coords
/// (gx in 0..5, gy in 0..7). Returns coverage 0..1.
pub fn glyph_sample(c: char, gx: f32, gy: f32) -> f32 {
    let rows = glyph_rows(c);
    let x = gx - 0.5;
    let y = gy - 0.5;
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (smoothstep(0.0, 1.0, x - x.floor()), smoothstep(0.0, 1.0, y - y.floor()));
    let a = mix(glyph_px(&rows, xi, yi), glyph_px(&rows, xi + 1, yi), fx);
    let b = mix(glyph_px(&rows, xi, yi + 1), glyph_px(&rows, xi + 1, yi + 1), fx);
    mix(a, b, fy)
}

/// Samples a string laid out at `origin` (top-left, uv) with each glyph
/// pixel `px` uv units big and 1 pixel of letter spacing. Coverage 0..1.
pub fn text_sample(s: &str, u: f32, v: f32, origin: (f32, f32), px: f32) -> f32 {
    let gx = (u - origin.0) / px;
    let gy = (v - origin.1) / px;
    if gy < -1.0 || gy > 8.0 || gx < -1.0 {
        return 0.0;
    }
    let idx = (gx / 6.0).floor();
    if idx < 0.0 {
        return 0.0;
    }
    let idx = idx as usize;
    match s.chars().nth(idx) {
        Some(c) => glyph_sample(c, gx - idx as f32 * 6.0, gy),
        None => 0.0,
    }
}

/// Width in uv of a string rendered with `text_sample` at pixel size `px`.
pub fn text_width(s: &str, px: f32) -> f32 {
    (s.chars().count() as f32 * 6.0 - 1.0) * px
}

// ---------------------------------------------------------------------------
// Embedded halftone bust (downsampled from the project's reference art)
// ---------------------------------------------------------------------------

pub const BUST_W: usize = 96;
pub const BUST_H: usize = 168;
static BUST: &[u8; BUST_W * BUST_H] = include_bytes!("bust.gray");

/// Bilinear luminance sample of the bust at normalized coords (0..1, 0..1).
pub fn bust_sample(nx: f32, ny: f32) -> f32 {
    if !(0.0..1.0).contains(&nx) || !(0.0..1.0).contains(&ny) {
        return 0.0;
    }
    let x = nx * (BUST_W - 1) as f32;
    let y = ny * (BUST_H - 1) as f32;
    let (xi, yi) = (x as usize, y as usize);
    let (fx, fy) = (x - xi as f32, y - yi as f32);
    let p = |xx: usize, yy: usize| BUST[yy.min(BUST_H - 1) * BUST_W + xx.min(BUST_W - 1)] as f32 / 255.0;
    mix(mix(p(xi, yi), p(xi + 1, yi), fx), mix(p(xi, yi + 1), p(xi + 1, yi + 1), fx), fy)
}
