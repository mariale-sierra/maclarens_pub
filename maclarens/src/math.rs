//! Matematica basica: vectores, hashing, ruido Perlin y fBm.
use std::ops::{Add, AddAssign, Div, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const fn v3(x: f32, y: f32, z: f32) -> V3 {
    V3 { x, y, z }
}

impl V3 {
    pub const ZERO: V3 = v3(0.0, 0.0, 0.0);
    pub const ONE: V3 = v3(1.0, 1.0, 1.0);
    #[inline]
    pub fn dot(self, o: V3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    #[inline]
    pub fn cross(self, o: V3) -> V3 {
        v3(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }
    #[inline]
    pub fn len(self) -> f32 {
        self.dot(self).sqrt()
    }
    #[inline]
    pub fn norm(self) -> V3 {
        let l = self.len();
        if l > 0.0 {
            self / l
        } else {
            self
        }
    }
    #[inline]
    pub fn get(self, i: usize) -> f32 {
        match i {
            0 => self.x,
            1 => self.y,
            _ => self.z,
        }
    }
    #[inline]
    pub fn set(&mut self, i: usize, v: f32) {
        match i {
            0 => self.x = v,
            1 => self.y = v,
            _ => self.z = v,
        }
    }
    #[inline]
    pub fn lerp(self, o: V3, t: f32) -> V3 {
        self * (1.0 - t) + o * t
    }
    /// Reflexion de un vector incidente respecto a la normal n.
    #[inline]
    pub fn reflect(self, n: V3) -> V3 {
        self - n * (2.0 * self.dot(n))
    }
    /// Producto componente a componente (mezcla de colores).
    #[inline]
    pub fn mul_v(self, o: V3) -> V3 {
        v3(self.x * o.x, self.y * o.y, self.z * o.z)
    }
}

impl Add for V3 {
    type Output = V3;
    #[inline]
    fn add(self, o: V3) -> V3 {
        v3(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}
impl Sub for V3 {
    type Output = V3;
    #[inline]
    fn sub(self, o: V3) -> V3 {
        v3(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}
impl Mul<f32> for V3 {
    type Output = V3;
    #[inline]
    fn mul(self, s: f32) -> V3 {
        v3(self.x * s, self.y * s, self.z * s)
    }
}
impl Div<f32> for V3 {
    type Output = V3;
    #[inline]
    fn div(self, s: f32) -> V3 {
        v3(self.x / s, self.y / s, self.z / s)
    }
}
impl Neg for V3 {
    type Output = V3;
    #[inline]
    fn neg(self) -> V3 {
        v3(-self.x, -self.y, -self.z)
    }
}
impl AddAssign for V3 {
    #[inline]
    fn add_assign(&mut self, o: V3) {
        *self = *self + o;
    }
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------- Hash y ruido ----------

pub fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^= x >> 16;
    x
}

pub fn hash3(x: i32, y: i32, seed: u32) -> u32 {
    hash_u32(
        (x as u32).wrapping_mul(0x9E3779B1)
            ^ (y as u32).wrapping_mul(0x85EBCA77)
            ^ seed.wrapping_mul(0xC2B2AE3D),
    )
}

/// Numero pseudoaleatorio determinista en [0,1) para una celda (x,y).
pub fn rnd(x: i32, y: i32, seed: u32) -> f32 {
    (hash3(x, y, seed) >> 8) as f32 / 16_777_216.0
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn grad(h: u32, x: f32, y: f32) -> f32 {
    match h & 7 {
        0 => x + y,
        1 => x - y,
        2 => -x + y,
        3 => -x - y,
        4 => x,
        5 => -x,
        6 => y,
        _ => -y,
    }
}

/// Ruido Perlin 2D (aprox. en [-1,1]).
pub fn perlin(x: f32, y: f32, seed: u32) -> f32 {
    let xf0 = x.floor();
    let yf0 = y.floor();
    let (xf, yf) = (x - xf0, y - yf0);
    let (xi, yi) = (xf0 as i32, yf0 as i32);
    let n00 = grad(hash3(xi, yi, seed), xf, yf);
    let n10 = grad(hash3(xi + 1, yi, seed), xf - 1.0, yf);
    let n01 = grad(hash3(xi, yi + 1, seed), xf, yf - 1.0);
    let n11 = grad(hash3(xi + 1, yi + 1, seed), xf - 1.0, yf - 1.0);
    let (u, v) = (fade(xf), fade(yf));
    let a = n00 + (n10 - n00) * u;
    let b = n01 + (n11 - n01) * u;
    a + (b - a) * v
}

/// Suma de octavas de Perlin (fractal Brownian motion).
pub fn fbm(x: f32, y: f32, seed: u32, octaves: u32) -> f32 {
    let (mut amp, mut freq, mut sum, mut norm) = (1.0, 1.0, 0.0, 0.0);
    for o in 0..octaves {
        sum += amp * perlin(x * freq, y * freq, seed.wrapping_add(o * 101));
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}
