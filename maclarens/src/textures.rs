//! Texturas 16x16 generadas por codigo (estilo Minecraft) y mapas normales.
//! No se usan imagenes externas: cada pixel se calcula proceduralmente.
use crate::math::*;
use std::f32::consts::TAU;

pub const TS: usize = 16;

fn c(r: u8, g: u8, b: u8) -> V3 {
    v3(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
}
/// sRGB (aprox.) -> lineal, para iluminar en espacio lineal.
fn lin(c: V3) -> V3 {
    v3(
        c.x.max(0.0).powf(2.2),
        c.y.max(0.0).powf(2.2),
        c.z.max(0.0).powf(2.2),
    )
}
fn n(x: usize, y: usize, seed: u32) -> f32 {
    rnd(x as i32, y as i32, seed)
}
/// Variacion de brillo alrededor de un color base.
fn var(base: V3, amount: f32, x: usize, y: usize, seed: u32) -> V3 {
    base * (1.0 + (n(x, y, seed) - 0.5) * 2.0 * amount)
}

#[derive(Clone)]
pub struct Texture {
    px: Vec<V3>,
}

impl Texture {
    pub fn from_fn(f: impl Fn(usize, usize) -> V3) -> Self {
        let mut px = Vec::with_capacity(TS * TS);
        for y in 0..TS {
            for x in 0..TS {
                px.push(lin(f(x, y)));
            }
        }
        Texture { px }
    }
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> V3 {
        let x = ((u * TS as f32) as usize).min(TS - 1);
        let y = ((v * TS as f32) as usize).min(TS - 1);
        self.px[y * TS + x]
    }
}

/// Mapa normal en espacio tangente (x=u, y=v, z=normal), sacado de un mapa de alturas.
#[derive(Clone)]
pub struct NormalMap {
    nrm: Vec<V3>,
}

impl NormalMap {
    pub fn from_height(h: impl Fn(usize, usize) -> f32, strength: f32) -> Self {
        let hh = |x: i32, y: i32| h(x.rem_euclid(16) as usize, y.rem_euclid(16) as usize);
        let mut nrm = Vec::with_capacity(TS * TS);
        for y in 0..TS as i32 {
            for x in 0..TS as i32 {
                let dx = hh(x + 1, y) - hh(x - 1, y);
                let dy = hh(x, y + 1) - hh(x, y - 1);
                nrm.push(v3(-dx * strength, -dy * strength, 1.0).norm());
            }
        }
        NormalMap { nrm }
    }
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> V3 {
        let x = ((u * TS as f32) as usize).min(TS - 1);
        let y = ((v * TS as f32) as usize).min(TS - 1);
        self.nrm[y * TS + x]
    }
}

// ---------------- Generadores de texturas ----------------

pub fn air() -> Texture {
    Texture::from_fn(|_, _| V3::ZERO)
}

pub fn grass_top() -> Texture {
    Texture::from_fn(|x, y| {
        let mut col = var(c(92, 158, 54), 0.16, x, y, 1);
        if n(x, y, 2) > 0.88 {
            col = col * 0.8;
        }
        col
    })
}

pub fn grass_side() -> Texture {
    Texture::from_fn(|x, y| {
        let edge = 3 + (n(x, 0, 3) * 2.0) as usize;
        if y < edge {
            var(c(92, 158, 54), 0.16, x, y, 1)
        } else {
            dirt_px(x, y)
        }
    })
}

fn dirt_px(x: usize, y: usize) -> V3 {
    let mut col = var(c(124, 86, 58), 0.13, x, y, 4);
    if n(x, y, 5) > 0.9 {
        col = col * 0.72;
    }
    col
}
pub fn dirt() -> Texture {
    Texture::from_fn(dirt_px)
}

fn stone_h(x: usize, y: usize) -> f32 {
    0.6 * n(x, y, 7) + 0.4 * n(x / 2, y / 2, 8)
}
pub fn stone() -> (Texture, NormalMap) {
    let tex = Texture::from_fn(|x, y| {
        let h = stone_h(x, y);
        c(118, 118, 124) * (0.78 + 0.5 * h)
    });
    (tex, NormalMap::from_height(stone_h, 2.2))
}

pub fn sand() -> Texture {
    Texture::from_fn(|x, y| var(c(222, 205, 150), 0.06, x, y, 9))
}

fn wave(x: usize, y: usize) -> f32 {
    let (fx, fy) = (x as f32, y as f32);
    0.5 * (TAU * (fx + 2.0 * fy) / 16.0).sin() + 0.5 * (TAU * (2.0 * fx - fy) / 16.0).sin()
}
pub fn water() -> (Texture, NormalMap) {
    let tex = Texture::from_fn(|x, y| c(40, 100, 200) * (1.0 + 0.16 * wave(x, y)));
    (tex, NormalMap::from_height(wave, 0.35))
}

pub fn glass() -> Texture {
    Texture::from_fn(|x, y| {
        let border = x == 0 || y == 0 || x == 15 || y == 15;
        let streak = (x + y == 5) || (x + y == 8 && x > 3);
        if border {
            c(235, 250, 255)
        } else if streak {
            c(245, 252, 255)
        } else {
            c(196, 226, 238)
        }
    })
}

pub fn tinted_glass(base: V3) -> Texture {
    Texture::from_fn(move |x, y| {
        let border = x == 0 || y == 0 || x == 15 || y == 15;
        let hl = x == 4 && y > 2 && y < 12;
        if border {
            base * 1.25
        } else if hl {
            base * 1.5
        } else {
            base
        }
    })
}

/// Piso de madera barnizada oscura (tablones de 4 px).
pub fn floor_wood() -> Texture {
    Texture::from_fn(|x, y| {
        let row = y / 4;
        let off = (row * 5) % 16;
        let xx = (x + off) % 16;
        let seam = y % 4 == 0 || xx == 0;
        let tone = 0.85 + 0.3 * n(row, xx / 8, 15);
        let base = c(84, 44, 27) * tone;
        if seam {
            c(48, 24, 14)
        } else {
            var(base, 0.07, x, y, 16)
        }
    })
}

/// Madera oscura (muebles, barra, repisas).
pub fn dark_wood() -> Texture {
    Texture::from_fn(|x, y| {
        let grain = 0.8 + 0.4 * n(x, 0, 17);
        let mut col = c(88, 44, 26) * grain;
        if x == 0 || y == 0 || x == 15 || y == 15 {
            col = col * 0.7;
        }
        var(col, 0.05, x, y, 18)
    })
}

/// Tapa de la barra: caoba pulida con vetas horizontales.
pub fn bar_top() -> Texture {
    Texture::from_fn(|x, y| {
        let grain = 0.85 + 0.3 * n(0, y, 19);
        var(c(112, 58, 32) * grain, 0.04, x, y, 20)
    })
}

fn brick_cell(x: usize, y: usize) -> (bool, usize) {
    let row = y / 4;
    let xx = (x + if row % 2 == 0 { 0 } else { 4 }) % 16;
    (y % 4 == 0 || xx % 8 == 0, row * 2 + xx / 8)
}
fn brick_h(x: usize, y: usize) -> f32 {
    let (mortar, _) = brick_cell(x, y);
    if mortar {
        0.0
    } else {
        1.0 + 0.15 * n(x, y, 21)
    }
}
/// Ladrillo blanco (como el pasillo del pub). Lleva mapa normal.
pub fn brick() -> (Texture, NormalMap) {
    let tex = Texture::from_fn(|x, y| {
        let (mortar, id) = brick_cell(x, y);
        if mortar {
            c(150, 148, 142)
        } else {
            c(228, 226, 220) * (0.93 + 0.1 * n(id, 0, 22)) * (0.97 + 0.06 * n(x, y, 23))
        }
    });
    (tex, NormalMap::from_height(brick_h, 2.0))
}

pub fn green_wall() -> Texture {
    Texture::from_fn(|x, y| var(c(30, 84, 68), 0.05, x, y, 24))
}

pub fn leather() -> Texture {
    Texture::from_fn(|x, y| {
        if x % 8 == 0 || y % 8 == 0 {
            c(105, 12, 16)
        } else {
            var(c(192, 22, 26), 0.06, x, y, 25)
        }
    })
}

pub fn door() -> Texture {
    Texture::from_fn(|x, y| var(c(206, 46, 30), 0.05, x, y, 26))
}

/// Pantalla de lampara: tela crema con costuras verticales.
pub fn lamp() -> Texture {
    Texture::from_fn(|x, y| {
        let rib = if x % 4 == 0 { 0.9 } else { 1.0 };
        let fade = 1.0 - 0.12 * (y as f32 / 15.0);
        c(255, 226, 168) * (rib * fade * (0.96 + 0.08 * n(x, y, 27)))
    })
}

/// Llama de vela.
pub fn flame() -> Texture {
    Texture::from_fn(|x, y| {
        let d = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt() / 8.0;
        c(255, 250, 200).lerp(c(255, 140, 30), d.min(1.0))
    })
}

pub fn gold() -> Texture {
    Texture::from_fn(|x, y| var(c(214, 172, 74), 0.08, x, y, 30))
}

pub fn wainscot() -> Texture {
    Texture::from_fn(|x, y| {
        let groove = x % 8 == 0 || y == 0 || y == 15;
        let col = c(50, 27, 18) * (0.85 + 0.3 * n(x / 8, 0, 31));
        if groove { col * 0.6 } else { var(col, 0.06, x, y, 32) }
    })
}

/// Cuadro negro brillante (marcos de la pared, menus).
pub fn picture() -> Texture {
    Texture::from_fn(|x, y| {
        let hl = x + y == 9 || x + y == 10;
        if x == 0 || y == 0 || x == 15 || y == 15 {
            c(38, 20, 14)
        } else if hl {
            c(40, 40, 46)
        } else {
            var(c(12, 12, 15), 0.2, x, y, 33)
        }
    })
}

/// Cojin de cuero verde oscuro para los bancos de la barra (tono apagado, no saturado).
pub fn green_seat() -> Texture {
    Texture::from_fn(|x, y| {
        if x % 8 == 0 || y % 8 == 0 {
            c(24, 38, 28)
        } else {
            var(c(42, 68, 50), 0.08, x, y, 34)
        }
    })
}

/// Tira de neon calida (luz continua, no una lampara con pantalla).
pub fn neon() -> Texture {
    Texture::from_fn(|_, y| {
        if y > 4 && y < 11 {
            c(255, 220, 140)
        } else {
            c(255, 176, 70)
        }
    })
}

/// Acera de piedra lisa bajo el diorama (junta cada 8px, como losas).
pub fn pavement() -> Texture {
    Texture::from_fn(|x, y| {
        let seam = x % 8 == 0 || y % 8 == 0;
        if seam {
            c(118, 118, 114)
        } else {
            var(c(176, 176, 170), 0.05, x, y, 40)
        }
    })
}

pub fn plaster() -> Texture {
    Texture::from_fn(|x, y| var(c(232, 230, 224), 0.03, x, y, 28))
}

pub fn mirror() -> Texture {
    Texture::from_fn(|x, y| var(c(150, 172, 188), 0.03, x, y, 29) * (1.0 - 0.06 * (x + y) as f32 / 30.0))
}

/// Piel para las figuras de personas (tono calido con variacion sutil).
pub fn skin_tex() -> Texture {
    Texture::from_fn(|x, y| var(c(222, 178, 142), 0.06, x, y, 60))
}
/// Cabello (una gorra de color solido encima de la cabeza).
pub fn hair_tex() -> Texture {
    Texture::from_fn(|x, y| var(c(46, 30, 20), 0.12, x, y, 61))
}
/// Tela generica para camisas/ropa: recibe el color base (fraccion 0..1) y le da textura de tejido.
pub fn cloth(base: V3) -> Texture {
    Texture::from_fn(move |x, y| var(base, 0.06, x, y, 62))
}
