//! Ray tracer sobre voxeles: DDA, sombras, AO, reflexion, refraccion, mapas normales y emision.
use crate::math::*;
use crate::scene::{b, Material, World, NX, NY, NZ};
use crate::sky::{sun_dir, Skybox};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

const MAX_DEPTH: u32 = 5;
const EPS: f32 = 1e-3;
const LIGHT_RADIUS2: f32 = 15.0 * 15.0;

// ---------------- Camara orbital ----------------

pub struct Camera {
    pub pos: V3,
    fwd: V3,
    right: V3,
    up: V3,
    tan_half: f32,
}

impl Camera {
    /// Camara que orbita alrededor de `target` (azimut/elevacion en grados).
    pub fn orbit(target: V3, az_deg: f32, el_deg: f32, dist: f32, fov_deg: f32) -> Camera {
        let (az, el) = (az_deg.to_radians(), el_deg.to_radians());
        let pos = target + v3(el.cos() * az.sin(), el.sin(), el.cos() * az.cos()) * dist;
        let fwd = (target - pos).norm();
        let right = fwd.cross(v3(0.0, 1.0, 0.0)).norm();
        let up = right.cross(fwd);
        Camera { pos, fwd, right, up, tan_half: (fov_deg.to_radians() * 0.5).tan() }
    }
    /// Camara en primera persona: posicion libre + direccion por yaw/pitch (grados).
    /// yaw=0 mira hacia +z, yaw=90 mira hacia +x; pitch>0 mira hacia arriba.
    pub fn first_person(pos: V3, yaw_deg: f32, pitch_deg: f32, fov_deg: f32) -> Camera {
        let (yaw, pitch) = (yaw_deg.to_radians(), pitch_deg.to_radians());
        let fwd = v3(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()).norm();
        let right = fwd.cross(v3(0.0, 1.0, 0.0)).norm();
        let up = right.cross(fwd);
        Camera { pos, fwd, right, up, tan_half: (fov_deg.to_radians() * 0.5).tan() }
    }
    #[inline]
    fn ray(&self, px: f32, py: f32, w: usize, h: usize) -> V3 {
        let aspect = w as f32 / h as f32;
        let sx = (2.0 * px / w as f32 - 1.0) * aspect * self.tan_half;
        let sy = (1.0 - 2.0 * py / h as f32) * self.tan_half;
        (self.fwd + self.right * sx + self.up * sy).norm()
    }
}

// ---------------- Recorrido de voxeles (Amanatides & Woo) ----------------

struct Visit {
    cell: [i32; 3],
    axis: Option<usize>, // eje de la cara por la que se entro (None = celda inicial dentro de la malla)
    t: f32,
}

struct Dda {
    cell: [i32; 3],
    step: [i32; 3],
    t_max: [f32; 3],
    t_delta: [f32; 3],
    first: Option<Visit>,
    limit: f32,
}

impl Dda {
    fn new(o: V3, d: V3, tmax: f32) -> Option<Dda> {
        let dims = [NX as i32, NY as i32, NZ as i32];
        let (mut t0, mut t1) = (0.0f32, tmax);
        let mut entry: Option<usize> = None;
        for a in 0..3 {
            let (oa, da) = (o.get(a), d.get(a));
            if da.abs() < 1e-9 {
                if oa < 0.0 || oa >= dims[a] as f32 {
                    return None;
                }
                continue;
            }
            let inv = 1.0 / da;
            let (mut ta, mut tb) = ((0.0 - oa) * inv, (dims[a] as f32 - oa) * inv);
            if ta > tb {
                std::mem::swap(&mut ta, &mut tb);
            }
            if ta > t0 {
                t0 = ta;
                entry = Some(a);
            }
            if tb < t1 {
                t1 = tb;
            }
            if t0 > t1 {
                return None;
            }
        }
        let p = o + d * (t0 + 1e-4);
        let mut cell = [0i32; 3];
        let mut step = [0i32; 3];
        let mut t_max = [f32::INFINITY; 3];
        let mut t_delta = [f32::INFINITY; 3];
        for a in 0..3 {
            cell[a] = (p.get(a).floor() as i32).clamp(0, dims[a] - 1);
            let da = d.get(a);
            if da.abs() > 1e-9 {
                step[a] = if da > 0.0 { 1 } else { -1 };
                let boundary = if da > 0.0 { cell[a] + 1 } else { cell[a] } as f32;
                t_max[a] = (boundary - o.get(a)) / da;
                t_delta[a] = 1.0 / da.abs();
            }
        }
        Some(Dda { cell, step, t_max, t_delta, first: Some(Visit { cell, axis: entry, t: t0 }), limit: t1 })
    }

    #[inline]
    fn next(&mut self) -> Option<Visit> {
        if let Some(v) = self.first.take() {
            return Some(v);
        }
        let a = if self.t_max[0] < self.t_max[1] {
            if self.t_max[0] < self.t_max[2] { 0 } else { 2 }
        } else if self.t_max[1] < self.t_max[2] {
            1
        } else {
            2
        };
        let t = self.t_max[a];
        if t > self.limit {
            return None;
        }
        self.cell[a] += self.step[a];
        let dims = [NX as i32, NY as i32, NZ as i32];
        if self.cell[a] < 0 || self.cell[a] >= dims[a] {
            return None;
        }
        self.t_max[a] += self.t_delta[a];
        Some(Visit { cell: self.cell, axis: Some(a), t })
    }
}

struct Hit {
    t: f32,
    cell: [i32; 3],
    axis: usize,
    step: i32,
    block: u8,
}

// ---------------- Optica ----------------

fn fresnel(cosi: f32, n1: f32, n2: f32) -> f32 {
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    let mut c = cosi;
    if n1 > n2 {
        let s2 = (n1 / n2) * (n1 / n2) * (1.0 - cosi * cosi);
        if s2 >= 1.0 {
            return 1.0; // reflexion total interna
        }
        c = (1.0 - s2).sqrt();
    }
    r0 + (1.0 - r0) * (1.0 - c).powi(5)
}

/// Ley de Snell. `d` incidente, `n` normal opuesta a d, eta = n1/n2.
fn refract(d: V3, n: V3, eta: f32) -> Option<V3> {
    let cosi = -d.dot(n);
    let k = 1.0 - eta * eta * (1.0 - cosi * cosi);
    if k < 0.0 {
        None
    } else {
        Some((d * eta + n * (eta * cosi - k.sqrt())).norm())
    }
}

/// Marco tangente (T = direccion de u, B = direccion de v) y coordenadas de textura de la cara.
fn face_frame(axis: usize) -> (V3, V3) {
    match axis {
        0 => (v3(0.0, 0.0, 1.0), v3(0.0, -1.0, 0.0)),
        1 => (v3(1.0, 0.0, 0.0), v3(0.0, 0.0, 1.0)),
        _ => (v3(1.0, 0.0, 0.0), v3(0.0, -1.0, 0.0)),
    }
}
fn face_uv(p: V3, axis: usize) -> (f32, f32) {
    let (fx, fy, fz) = (p.x - p.x.floor(), p.y - p.y.floor(), p.z - p.z.floor());
    match axis {
        0 => (fz, 1.0 - fy),
        1 => (fx, fz),
        _ => (fx, 1.0 - fy),
    }
}

pub struct Renderer<'a> {
    pub world: &'a World,
    pub mats: &'a [Material],
    pub sky: &'a Skybox,
    sun: V3,
}

const SUN_COL: V3 = v3(0.30, 0.33, 0.40);
const AMB_SKY: V3 = v3(0.07, 0.09, 0.13);
const AMB_GROUND: V3 = v3(0.035, 0.028, 0.022);

impl<'a> Renderer<'a> {
    pub fn new(world: &'a World, mats: &'a [Material], sky: &'a Skybox) -> Self {
        Renderer { world, mats, sky, sun: sun_dir() }
    }

    fn first_hit(&self, o: V3, d: V3, medium: u8) -> Option<Hit> {
        let mut dda = Dda::new(o, d, f32::INFINITY)?;
        while let Some(v) = dda.next() {
            if let Some(a) = v.axis {
                let blk = self.world.get(v.cell[0], v.cell[1], v.cell[2]);
                if blk != medium {
                    return Some(Hit { t: v.t, cell: v.cell, axis: a, step: dda.step[a], block: blk });
                }
            }
        }
        None
    }

    /// Transmitancia de luz entre `o` y `o + dir*tmax` (0 = sombra total; el vidrio/agua dejan pasar algo).
    fn shadow(&self, o: V3, dir: V3, tmax: f32) -> f32 {
        if tmax <= 0.0 {
            return 1.0;
        }
        let mut dda = match Dda::new(o, dir, tmax) {
            Some(d) => d,
            None => return 1.0,
        };
        let mut att = 1.0;
        while let Some(v) = dda.next() {
            if v.axis.is_none() {
                continue;
            }
            let blk = self.world.get(v.cell[0], v.cell[1], v.cell[2]);
            if blk == b::AIR {
                continue;
            }
            let m = &self.mats[blk as usize];
            if m.transparency < 0.05 {
                return 0.0;
            }
            att *= m.transparency * 0.9;
            if att < 0.02 {
                return 0.0;
            }
        }
        att
    }

    /// Oclusion ambiental de voxeles con interpolacion bilineal por esquina.
    fn ao(&self, h: &Hit, p: V3) -> f32 {
        let (ta, ba) = match h.axis {
            0 => (2usize, 1usize),
            1 => (0, 2),
            _ => (0, 1),
        };
        let mut c = h.cell;
        c[h.axis] -= h.step;
        let (fu, fv) = (p.get(ta) - p.get(ta).floor(), p.get(ba) - p.get(ba).floor());
        let s = |du: i32, dv: i32| -> bool {
            let mut q = c;
            q[ta] += du;
            q[ba] += dv;
            self.world.is_solid(q[0], q[1], q[2])
        };
        let corner = |su: i32, sv: i32| -> f32 {
            let (s1, s2, cc) = (s(su, 0), s(0, sv), s(su, sv));
            if s1 && s2 {
                0.0
            } else {
                (3 - s1 as i32 - s2 as i32 - cc as i32) as f32 / 3.0
            }
        };
        let (a00, a10, a01, a11) = (corner(-1, -1), corner(1, -1), corner(-1, 1), corner(1, 1));
        let top = a00 * (1.0 - fu) + a10 * fu;
        let bot = a01 * (1.0 - fu) + a11 * fu;
        top * (1.0 - fv) + bot * fv
    }

    pub fn trace(&self, o: V3, d: V3, medium: u8, depth: u32) -> V3 {
        match self.first_hit(o, d, medium) {
            None => self.sky.sample(d),
            Some(h) => self.shade(o, d, &h, medium, depth),
        }
    }

    fn shade(&self, o: V3, d: V3, h: &Hit, medium: u8, depth: u32) -> V3 {
        let p = o + d * h.t;
        let mut nrm = V3::ZERO;
        nrm.set(h.axis, -(h.step as f32));
        let (u, v) = face_uv(p, h.axis);
        let (tan, bit) = face_frame(h.axis);
        let recurse = depth < MAX_DEPTH;
        let n1 = self.mats[medium as usize].ior;

        // ---- El rayo sale de un medio transparente hacia el aire ----
        if h.block == b::AIR {
            let mm = &self.mats[medium as usize];
            let ns = match &mm.normal {
                Some(nm) => {
                    let t = nm.sample(u, v);
                    (tan * t.x + bit * t.y + nrm * t.z).norm()
                }
                None => nrm,
            };
            let cosi = (-d).dot(ns).clamp(0.0, 1.0);
            let kr = fresnel(cosi, n1, 1.0);
            if !recurse {
                return self.sky.sample(d);
            }
            let refl = self.trace(p + nrm * EPS, d.reflect(ns), medium, depth + 1);
            return match refract(d, ns, n1) {
                Some(t) => {
                    let out = self.trace(p - nrm * EPS, t, b::AIR, depth + 1);
                    refl * kr + out * (1.0 - kr)
                }
                None => refl,
            };
        }

        let mat = &self.mats[h.block as usize];
        let tex_i = if h.axis == 1 { if nrm.y > 0.0 { 0 } else { 2 } } else { 1 };
        let base = mat.tex[tex_i].sample(u, v).mul_v(mat.albedo);

        // ---- Material emisivo: luz propia ----
        if mat.emission > 0.0 {
            return base * mat.emission;
        }

        // ---- Mapa normal: perturba la normal en espacio tangente ----
        let ns = match &mat.normal {
            Some(nm) => {
                let t = nm.sample(u, v);
                (tan * t.x + bit * t.y + nrm * t.z).norm()
            }
            None => nrm,
        };

        // ---- Iluminacion directa ----
        let pl = p + nrm * EPS;
        let view = -d;
        let ao = 0.25 + 0.75 * self.ao(h, p);
        let amb = AMB_GROUND.lerp(AMB_SKY, 0.5 + 0.5 * ns.y);
        let mut diff = amb * ao;
        let mut spec = V3::ZERO;

        let ndl = ns.dot(self.sun);
        if ndl > 0.0 && nrm.dot(self.sun) > 0.0 {
            let s = self.shadow(pl, self.sun, f32::INFINITY);
            if s > 0.0 {
                diff += SUN_COL * (ndl * s * (0.55 + 0.45 * ao));
                let hv = (self.sun + view).norm();
                spec += SUN_COL * (ns.dot(hv).max(0.0).powf(mat.shininess) * mat.specular * s);
            }
        }
        for l in &self.world.lights {
            let dv = l.pos - pl;
            let d2 = dv.dot(dv);
            if d2 > LIGHT_RADIUS2 {
                continue;
            }
            let dist = d2.sqrt();
            let ld = dv / dist;
            let ndl = ns.dot(ld);
            if ndl <= 0.0 || nrm.dot(ld) <= 0.0 {
                continue;
            }
            let fall = 1.0 - d2 / LIGHT_RADIUS2;
            let att = fall * fall / (1.0 + 0.35 * d2);
            let s = self.shadow(pl, ld, dist - 0.9);
            if s < 0.002 {
                continue;
            }
            diff += l.color * (ndl * att * s * (0.6 + 0.4 * ao));
            let hv = (ld + view).norm();
            spec += l.color * (ns.dot(hv).max(0.0).powf(mat.shininess) * mat.specular * att * s);
        }
        let local = base.mul_v(diff) + spec;
        let cosi = view.dot(ns).clamp(0.0, 1.0);

        // ---- Materiales transparentes: Fresnel + refraccion (Snell) + reflexion ----
        if mat.transparency > 0.0 {
            let n2 = mat.ior;
            let f = fresnel(cosi, n1, n2);
            let kr = (mat.reflectivity + (1.0 - mat.reflectivity) * f).min(1.0);
            if !recurse {
                return local;
            }
            let refl = if kr > 0.005 {
                self.trace(pl, d.reflect(ns), medium, depth + 1)
            } else {
                V3::ZERO
            };
            let trans = match refract(d, ns, n1 / n2) {
                Some(t) => self.trace(p - nrm * EPS, t, h.block, depth + 1).mul_v(mat.albedo),
                None => refl,
            };
            return refl * kr + (local * (1.0 - mat.transparency) + trans * mat.transparency) * (1.0 - kr);
        }

        // ---- Opacos con reflexion (espejo, piso barnizado, barra) ----
        if mat.reflectivity > 0.0 && recurse {
            let r = mat.reflectivity;
            let f = r + (1.0 - r) * (1.0 - cosi).powi(5) * 0.6;
            let refl = self.trace(pl, d.reflect(ns), medium, depth + 1).mul_v(mat.albedo);
            return local * (1.0 - f) + refl * f;
        }
        local
    }

    /// Renderiza la imagen en paralelo. Los hilos toman filas de una cola atomica (balanceo dinamico).
    pub fn render(&self, cam: &Camera, w: usize, h: usize, aa: usize, threads: usize) -> Vec<u8> {
        let next = AtomicUsize::new(0);
        let out = Mutex::new(vec![0u8; w * h * 3]);
        let inv = 1.0 / (aa * aa) as f32;
        thread::scope(|s| {
            for _ in 0..threads.max(1) {
                s.spawn(|| loop {
                    let y = next.fetch_add(1, Ordering::Relaxed);
                    if y >= h {
                        break;
                    }
                    let mut row = vec![0u8; w * 3];
                    for x in 0..w {
                        let mut acc = V3::ZERO;
                        for sy in 0..aa {
                            for sx in 0..aa {
                                let px = x as f32 + (sx as f32 + 0.5) / aa as f32;
                                let py = y as f32 + (sy as f32 + 0.5) / aa as f32;
                                let d = cam.ray(px, py, w, h);
                                acc += self.trace(cam.pos, d, b::AIR, 0);
                            }
                        }
                        let c = tonemap(acc * inv);
                        row[x * 3..x * 3 + 3].copy_from_slice(&c);
                    }
                    let mut g = out.lock().unwrap();
                    g[y * w * 3..(y + 1) * w * 3].copy_from_slice(&row);
                });
            }
        });
        out.into_inner().unwrap()
    }
}

/// Tone mapping ACES (aprox.) + gamma.
fn tonemap(c: V3) -> [u8; 3] {
    let f = |x: f32| -> u8 {
        let x = x.max(0.0);
        let a = (x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14);
        (a.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8
    };
    [f(c.x), f(c.y), f(c.z)]
}
