//! Skybox: un cubemap de 6 caras generado proceduralmente (degradado, sol y nubes).
use crate::math::*;

pub fn sun_dir() -> V3 {
    v3(0.55, 0.62, 0.75).norm()
}

/// Color del cielo nocturno en una direccion (azul oscuro, nunca negro puro, para que se
/// distinga del techo/piso en las tomas exteriores). Valores lineales; la luna puede ser >1.
fn sky_color(d: V3) -> V3 {
    let sun = sun_dir();
    let t = d.y;
    // Nota: el tonemap ACES+gamma aclara bastante los valores lineales chicos, asi que estos
    // numeros estan calibrados (no son el color final) para que el cielo SE VEA azul oscuro
    // de noche, no un pastel claro.
    let zenith = v3(0.0038, 0.0068, 0.018);
    let horizon = v3(0.018, 0.023, 0.042);
    let below = v3(0.0061, 0.0066, 0.0099);
    let mut col = if t >= 0.0 {
        horizon.lerp(zenith, t.sqrt().min(1.0))
    } else {
        horizon.lerp(below, (-t * 3.0).min(1.0))
    };
    // Luna: un resplandor suave, no un sol brillante.
    let s = d.dot(sun).max(0.0);
    col += v3(0.263, 0.284, 0.327) * (s.powi(90) * 0.55) + v3(0.102, 0.116, 0.145) * (s.powi(10) * 0.06);
    if s > 0.9997 {
        col = v3(2.4, 2.5, 2.6);
    }
    // Nubes nocturnas: se proyecta la direccion sobre un plano y se evalua fBm, con tinte frio.
    if t > 0.03 {
        let (px, pz) = (d.x / t * 0.55, d.z / t * 0.55);
        let dens = fbm(px + 40.0, pz + 40.0, 77, 5) * 0.5 + 0.5;
        let cover = smoothstep(0.50, 0.75, dens) * smoothstep(0.03, 0.22, t);
        let shade = 0.75 + 0.35 * smoothstep(0.75, 0.5, dens);
        col = col.lerp(v3(0.051, 0.058, 0.079) * shade, cover * 0.85);
    }
    col
}

/// Estrellas: puntos brillantes dispersos, mas densos lejos de la luna y por encima del horizonte.
fn star_brightness(face: usize, i: usize, j: usize) -> f32 {
    // Se agrupan los texeles de a 7x7 para que cada estrella mida varios pixeles de verdad
    // en la imagen final y no desaparezca al reescalar/mostrar la imagen.
    let (gi, gj) = (i / 4, j / 4);
    let r = rnd(gi as i32 * 131 + gj as i32 * 977, face as i32 * 7919, 999);
    if r > 0.9875 {
        // Forma redondeada chica: el centro del grupo brilla mas que los bordes.
        let (cx, cy) = ((i % 4) as f32 - 1.5, (j % 4) as f32 - 1.5);
        let d = (cx * cx + cy * cy).sqrt();
        (((r - 0.9875) / 0.0125) * smoothstep(1.6, 0.2, d)).min(1.0)
    } else {
        0.0
    }
}

pub struct Skybox {
    size: usize,
    faces: Vec<Vec<V3>>,
}

/// Direccion de un texel de la cara `f` con coordenadas u,v en [-1,1] (convencion OpenGL).
fn face_dir(f: usize, u: f32, v: f32) -> V3 {
    match f {
        0 => v3(1.0, -v, -u),
        1 => v3(-1.0, -v, u),
        2 => v3(u, 1.0, v),
        3 => v3(u, -1.0, -v),
        4 => v3(u, -v, 1.0),
        _ => v3(-u, -v, -1.0),
    }
}

impl Skybox {
    pub fn new(size: usize) -> Skybox {
        let mut faces = Vec::new();
        for f in 0..6 {
            let mut px = Vec::with_capacity(size * size);
            for j in 0..size {
                for i in 0..size {
                    let u = (i as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                    let v = (j as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                    let dir = face_dir(f, u, v).norm();
                    let mut col = sky_color(dir);
                    let star = star_brightness(f, i, j);
                    if star > 0.0 && dir.y > -0.2 && (col.x + col.y + col.z) < 0.15 {
                        col += V3::ONE * (star * 0.26);
                    }
                    px.push(col);
                }
            }
            faces.push(px);
        }
        Skybox { size, faces }
    }

    /// Muestrea el cubemap segun la direccion del rayo.
    #[inline]
    pub fn sample(&self, d: V3) -> V3 {
        let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
        let (f, sc, tc, ma) = if ax >= ay && ax >= az {
            if d.x > 0.0 { (0, -d.z, -d.y, ax) } else { (1, d.z, -d.y, ax) }
        } else if ay >= az {
            if d.y > 0.0 { (2, d.x, d.z, ay) } else { (3, d.x, -d.z, ay) }
        } else if d.z > 0.0 {
            (4, d.x, -d.y, az)
        } else {
            (5, -d.x, -d.y, az)
        };
        let s = self.size as f32;
        let i = (((sc / ma) * 0.5 + 0.5) * s) as usize;
        let j = (((tc / ma) * 0.5 + 0.5) * s) as usize;
        self.faces[f][j.min(self.size - 1) * self.size + i.min(self.size - 1)]
    }
}
