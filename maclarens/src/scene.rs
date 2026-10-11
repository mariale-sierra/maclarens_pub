//! Materiales, mundo de voxeles, terreno procedural y el diorama de MacLaren's Pub.
//! Resolucion "fina": cada mueble se arma con varias piezas pequenas (patas, marcos,
//! cables) en vez de bloques solidos grandes, para que se vea mas detallado.
use crate::math::*;
use crate::textures::*;

/// Identificadores de bloques.
pub mod b {
    pub const AIR: u8 = 0;
    #[allow(dead_code)]
    pub const GRASS: u8 = 1;
    pub const DIRT: u8 = 2;
    pub const STONE: u8 = 3;
    #[allow(dead_code)]
    pub const SAND: u8 = 4;
    #[allow(dead_code)]
    pub const WATER: u8 = 5;
    pub const WAINSCOT: u8 = 6;
    pub const PICTURE: u8 = 7;
    pub const FLOOR: u8 = 8;
    pub const DARKWOOD: u8 = 9;
    pub const BARTOP: u8 = 10;
    pub const BRICK: u8 = 11;
    pub const GREENWALL: u8 = 12;
    pub const LEATHER: u8 = 13;
    pub const DOOR: u8 = 14;
    pub const WHITE: u8 = 15;
    pub const GLASS: u8 = 16;
    pub const MIRROR: u8 = 17;
    pub const LAMP: u8 = 18;
    pub const NEON: u8 = 19;
    pub const AMBER: u8 = 20;
    pub const GREENGLASS: u8 = 21;
    pub const BLUEGLASS: u8 = 22;
    pub const PAVEMENT: u8 = 23;
    pub const GOLD: u8 = 24;
    pub const GREENSEAT: u8 = 25;
    pub const FLAME: u8 = 26;
    pub const ROOF: u8 = 27;
    pub const SKIN: u8 = 28;
    pub const HAIR: u8 = 29;
    pub const SHIRT_BLUE: u8 = 30;
    pub const SHIRT_MUSTARD: u8 = 31;
    pub const SHIRT_MAROON: u8 = 32;
    pub const RAILING: u8 = 33;
}

pub struct Material {
    pub name: &'static str,
    /// Texturas por cara: [arriba, lados, abajo]
    pub tex: [Texture; 3],
    pub normal: Option<NormalMap>,
    /// Tinte del albedo (se multiplica con la textura; en materiales transparentes tine la luz que pasa).
    pub albedo: V3,
    pub specular: f32,
    pub shininess: f32,
    pub transparency: f32,
    pub ior: f32,
    pub reflectivity: f32,
    pub emission: f32,
}

impl Material {
    fn new(name: &'static str, t: Texture) -> Self {
        Material {
            name,
            tex: [t.clone(), t.clone(), t],
            normal: None,
            albedo: V3::ONE,
            specular: 0.05,
            shininess: 16.0,
            transparency: 0.0,
            ior: 1.0,
            reflectivity: 0.0,
            emission: 0.0,
        }
    }
    fn faces(name: &'static str, top: Texture, side: Texture, bottom: Texture) -> Self {
        let mut m = Material::new(name, side.clone());
        m.tex = [top, side, bottom];
        m
    }
    fn tint(mut self, a: V3) -> Self {
        self.albedo = a;
        self
    }
    fn spec(mut self, s: f32, sh: f32) -> Self {
        self.specular = s;
        self.shininess = sh;
        self
    }
    fn refl(mut self, r: f32) -> Self {
        self.reflectivity = r;
        self
    }
    fn clear(mut self, transparency: f32, ior: f32) -> Self {
        self.transparency = transparency;
        self.ior = ior;
        self
    }
    fn glow(mut self, e: f32) -> Self {
        self.emission = e;
        self
    }
    fn nmap(mut self, n: NormalMap) -> Self {
        self.normal = Some(n);
        self
    }
}

/// Tabla de materiales; el indice es el id del bloque (ver `b`).
pub fn build_materials() -> Vec<Material> {
    let (stone_t, stone_n) = stone();
    let (water_t, water_n) = water();
    let (brick_t, brick_n) = brick();
    vec![
        Material::new("aire", air()).clear(1.0, 1.0),
        Material::faces("pasto", grass_top(), grass_side(), dirt()),
        Material::new("tierra", dirt()),
        Material::new("piedra", stone_t).nmap(stone_n).spec(0.15, 24.0),
        Material::new("arena", sand()),
        Material::new("agua", water_t)
            .nmap(water_n)
            .tint(v3(0.55, 0.82, 1.0))
            .spec(1.0, 90.0)
            .clear(0.85, 1.33)
            .refl(0.04),
        Material::new("panel de madera", wainscot()).spec(0.1, 10.0),
        Material::new("cuadro enmarcado", picture()).spec(0.35, 30.0),
        Material::new("piso", floor_wood()).spec(0.6, 48.0).refl(0.14),
        Material::new("madera oscura", dark_wood()).spec(0.25, 24.0).refl(0.03),
        Material::new("tapa barra", bar_top()).spec(0.9, 90.0).refl(0.22),
        Material::new("ladrillo", brick_t).nmap(brick_n).spec(0.1, 12.0),
        Material::new("pared verde", green_wall()),
        Material::new("cuero rojo", leather()).spec(0.7, 36.0).refl(0.05),
        Material::new("puerta roja", door()).spec(0.3, 20.0),
        Material::new("yeso blanco", plaster()),
        Material::new("vidrio", glass())
            .tint(v3(0.92, 0.98, 1.0))
            .spec(1.0, 120.0)
            .clear(0.92, 1.5)
            .refl(0.03),
        Material::new("espejo", mirror())
            .tint(v3(0.86, 0.93, 0.97))
            .spec(0.8, 100.0)
            .refl(0.88),
        Material::new("lampara", lamp()).glow(3.3),
        Material::new("neon", neon()).glow(2.6),
        Material::new("botella ambar", tinted_glass(v3(0.78, 0.42, 0.08)))
            .tint(v3(1.0, 0.55, 0.15))
            .spec(1.0, 100.0)
            .clear(0.65, 1.5),
        Material::new("botella verde", tinted_glass(v3(0.10, 0.45, 0.18)))
            .tint(v3(0.25, 0.85, 0.35))
            .spec(1.0, 100.0)
            .clear(0.65, 1.5),
        Material::new("botella azul", tinted_glass(v3(0.10, 0.25, 0.65)))
            .tint(v3(0.3, 0.5, 1.0))
            .spec(1.0, 100.0)
            .clear(0.65, 1.5),
        Material::new("pavimento", pavement()).spec(0.12, 14.0),
        Material::new("oro", gold()).spec(0.9, 70.0).refl(0.35),
        Material::new("asiento verde", green_seat()).spec(0.2, 16.0),
        Material::new("llama de vela", flame()).glow(4.0),
        Material::new("techo", dark_wood()).tint(v3(0.45, 0.43, 0.42)).spec(0.05, 8.0),
        Material::new("piel", skin_tex()).spec(0.18, 22.0),
        Material::new("cabello", hair_tex()).spec(0.08, 10.0),
        Material::new("camisa azul", cloth(v3(0.16, 0.30, 0.55))).spec(0.06, 10.0),
        Material::new("camisa mostaza", cloth(v3(0.72, 0.56, 0.16))).spec(0.06, 10.0),
        Material::new("camisa vino", cloth(v3(0.45, 0.10, 0.14))).spec(0.06, 10.0),
        Material::new("baranda verde", cloth(v3(0.07, 0.38, 0.20))).spec(0.4, 40.0).refl(0.08),
    ]
}

pub struct Light {
    pub pos: V3,
    pub color: V3,
}

// Huella del pub + un borde de acera alrededor (sin bosque ni lago: el diorama
// es la "caja" del bar sobre una base flotante, como una maqueta de exhibicion).
// Resolucion doblada respecto a la version anterior: mas voxeles por mueble.
const PW: i32 = 36;
const PD: i32 = 32;
const MARGIN: i32 = 6;
// El frente (lado de la entrada, x=0 de la pared) tiene un margen mucho mas grande que los
// otros tres lados: ahi vive la plaza hundida + las gradas + la acera de la calle.
const FRONT: i32 = 26;
pub const PX0: i32 = FRONT;
pub const PZ0: i32 = MARGIN;
pub const NX: usize = (FRONT + PW + MARGIN) as usize;
pub const NZ: usize = (PD + 2 * MARGIN) as usize;
pub const PADY: i32 = 18; // altura (y) del piso del pub sobre la base
pub const NY: usize = (PADY + 22) as usize;
/// Altura de la calle (nivel de la acera) sobre el piso del pub: se baja por las gradas.
const STREET_Y: i32 = 4;

pub struct World {
    pub blocks: Vec<u8>,
    solid: Vec<bool>,
    pub lights: Vec<Light>,
}

impl World {
    fn new() -> World {
        World {
            blocks: vec![0; NX * NY * NZ],
            solid: vec![false; NX * NY * NZ],
            lights: Vec::new(),
        }
    }
    #[inline]
    fn idx(x: i32, y: i32, z: i32) -> Option<usize> {
        if x < 0 || y < 0 || z < 0 || x >= NX as i32 || y >= NY as i32 || z >= NZ as i32 {
            None
        } else {
            Some((y as usize * NZ + z as usize) * NX + x as usize)
        }
    }
    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        match World::idx(x, y, z) {
            Some(i) => self.blocks[i],
            None => 0,
        }
    }
    /// Bloque opaco (para oclusion ambiental).
    #[inline]
    pub fn is_solid(&self, x: i32, y: i32, z: i32) -> bool {
        match World::idx(x, y, z) {
            Some(i) => self.solid[i],
            None => false,
        }
    }
    fn set(&mut self, x: i32, y: i32, z: i32, blk: u8) {
        if let Some(i) = World::idx(x, y, z) {
            self.blocks[i] = blk;
        }
    }
    fn fill(&mut self, a: (i32, i32, i32), c: (i32, i32, i32), blk: u8) {
        let (x0, x1) = (a.0.min(c.0), a.0.max(c.0));
        let (y0, y1) = (a.1.min(c.1), a.1.max(c.1));
        let (z0, z1) = (a.2.min(c.2), a.2.max(c.2));
        for y in y0..=y1 {
            for z in z0..=z1 {
                for x in x0..=x1 {
                    self.set(x, y, z, blk);
                }
            }
        }
    }
    /// Calcula la tabla de opacos y la lista de luces (bloques lampara y velas).
    fn finalize(&mut self, mats: &[Material]) {
        for i in 0..self.blocks.len() {
            let blk = self.blocks[i] as usize;
            self.solid[i] = blk != 0 && mats[blk].transparency < 0.5;
        }
        for y in 0..NY as i32 {
            for z in 0..NZ as i32 {
                for x in 0..NX as i32 {
                    let here = self.get(x, y, z);
                    if here == b::LAMP {
                        self.lights.push(Light {
                            pos: v3(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5),
                            color: v3(1.0, 0.72, 0.36) * 2.6,
                        });
                    } else if here == b::FLAME {
                        self.lights.push(Light {
                            pos: v3(x as f32 + 0.5, y as f32 + 0.6, z as f32 + 0.5),
                            color: v3(1.0, 0.55, 0.22) * 1.3,
                        });
                    }
                }
            }
        }
    }
}

/// Distancia (Chebyshev) de una columna a la huella del pub. 0 = dentro.
fn pad_dist(x: i32, z: i32) -> i32 {
    let dx = (PX0 - x).max(x - (PX0 + PW - 1)).max(0);
    let dz = (PZ0 - z).max(z - (PZ0 + PD - 1)).max(0);
    dx.max(dz)
}

pub fn generate(seed: u32, mats: &[Material]) -> World {
    let mut w = World::new();

    // ---- 1) Acera de piedra alrededor del pub (nivel PADY-1, como el zocalo de una maqueta) ----
    for z in 0..NZ as i32 {
        for x in 0..NX as i32 {
            if pad_dist(x, z) > 0 {
                w.set(x, PADY - 1, z, b::PAVEMENT);
            }
        }
    }

    // ---- 2) Terreno procedural: roca erosionada bajo la plataforma (heightmap Perlin fBm,
    //          48x44 columnas). Es la base sobre la que "flota" el diorama: se ve intencionalmente
    //          en los bordes y por debajo, con estratos de tierra y piedra. ----
    for z in 0..NZ as i32 {
        for x in 0..NX as i32 {
            let depth = 5.0 + fbm(x as f32 * 0.11, z as f32 * 0.11, seed, 3).abs() * 10.0;
            let bottom = (PADY - 2 - depth as i32).max(0);
            for y in bottom..(PADY - 1) {
                let blk = if y >= PADY - 7 { b::DIRT } else { b::STONE };
                w.set(x, y, z, blk);
            }
        }
    }

    // ---- 3) El pub ----
    build_pub(&mut w);
    w.finalize(mats);
    w
}

// ---------------- Construccion del pub (coordenadas locales, finas) ----------------

struct P<'a> {
    w: &'a mut World,
}
impl<'a> P<'a> {
    fn s(&mut self, x: i32, y: i32, z: i32, blk: u8) {
        self.w.set(PX0 + x, PADY + y, PZ0 + z, blk);
    }
    fn f(&mut self, a: (i32, i32, i32), c: (i32, i32, i32), blk: u8) {
        self.w.fill((PX0 + a.0, PADY + a.1, PZ0 + a.2), (PX0 + c.0, PADY + c.1, PZ0 + c.2), blk);
    }

    /// Silla con 4 patas delgadas, asiento y respaldo (2x2 de huella).
    /// `back`: 0=respaldo en +z, 1=en -z, 2=en +x, 3=en -x (queda mirando al lado opuesto).
    fn chair(&mut self, x: i32, z: i32, back: u8) {
        for &(dx, dz) in &[(0, 0), (1, 0), (0, 1), (1, 1)] {
            self.f((x + dx, 1, z + dz), (x + dx, 2, z + dz), b::DARKWOOD);
        }
        self.f((x, 3, z), (x + 1, 3, z + 1), b::GREENSEAT);
        match back {
            0 => self.f((x, 4, z + 1), (x + 1, 6, z + 1), b::DARKWOOD),
            1 => self.f((x, 4, z), (x + 1, 6, z), b::DARKWOOD),
            2 => self.f((x + 1, 4, z), (x + 1, 6, z + 1), b::DARKWOOD),
            _ => self.f((x, 4, z), (x, 6, z + 1), b::DARKWOOD),
        };
    }

    /// Mesa redonda con 4 patas delgadas y una vela (para el salon principal).
    fn leg_table(&mut self, x: i32, z: i32) {
        for &(dx, dz) in &[(0, 0), (2, 0), (0, 2), (2, 2)] {
            self.f((x + dx, 1, z + dz), (x + dx, 4, z + dz), b::DARKWOOD);
        }
        self.f((x - 1, 5, z - 1), (x + 3, 5, z + 3), b::BARTOP);
        self.s(x + 1, 6, z + 1, b::FLAME);
    }

    /// Mesa de pedestal central, para el hueco angosto entre los bancos de una cabina.
    fn pedestal_table(&mut self, x: i32, z: i32) {
        self.f((x, 1, z), (x, 4, z), b::DARKWOOD);
        self.f((x - 1, 5, z - 1), (x + 1, 5, z + 1), b::BARTOP);
        self.s(x, 6, z, b::FLAME);
    }

    /// Banco de la barra: pata central, aro dorado como reposapies y asiento redondeado.
    /// La altura queda a proposito debajo de la tapa de la barra (y5) para verse realista.
    fn barstool(&mut self, x: i32, z: i32) {
        self.f((x, 1, z), (x, 3, z), b::DARKWOOD);
        for &(dx, dz) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            self.s(x + dx, 2, z + dz, b::GOLD);
        }
        for &(dx, dz) in &[(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            self.s(x + dx, 4, z + dz, b::GREENSEAT);
        }
    }

    /// Foco colgante tipo "spotlight": cable delgado desde el techo, cubierta oscura
    /// y una pantalla en forma de cruz que da una luz calida hacia abajo.
    fn pendant(&mut self, x: i32, z: i32, ceiling_y: i32, shade_y: i32) {
        self.s(x, ceiling_y, z, b::STONE);
        self.f((x, shade_y + 1, z), (x, ceiling_y - 1, z), b::DARKWOOD);
        for &(dx, dz) in &[(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            self.s(x + dx, shade_y, z + dz, b::LAMP);
        }
    }

    /// Cabina de cuero rojo: dos bancas encaradas con una mesa de pedestal al centro.
    fn booth(&mut self, bx: i32, bz: i32) {
        for x in bx..=bx + 7 {
            for &z in &[bz, bz + 8] {
                self.s(x, 1, z, b::DARKWOOD);
                self.f((x, 2, z), (x, 4, z), b::LEATHER);
                self.s(x, 5, z, b::DARKWOOD);
            }
            for &z in &[bz + 1, bz + 2, bz + 6, bz + 7] {
                self.f((x, 1, z), (x, 2, z), b::LEATHER);
            }
        }
        self.pedestal_table(bx + 3, bz + 4);
    }

    /// Ventana con mainel (marco blanco cada `period` bloques) en la pared x=0.
    fn window_x(&mut self, z0: i32, z1: i32, y0: i32, y1: i32, period: i32) {
        for z in z0..=z1 {
            for y in y0..=y1 {
                let frame = z == z0 || z == z1 || y == y0 || y == y1 || (z - z0) % period == 0 || (y - y0) % period == 0;
                self.s(0, y, z, if frame { b::WHITE } else { b::GLASS });
            }
        }
    }
    /// Ventana con mainel en la pared z=0.
    fn window_z(&mut self, x0: i32, x1: i32, y0: i32, y1: i32, period: i32) {
        for x in x0..=x1 {
            for y in y0..=y1 {
                let frame = x == x0 || x == x1 || y == y0 || y == y1 || (x - x0) % period == 0 || (y - y0) % period == 0;
                self.s(x, y, 0, if frame { b::WHITE } else { b::GLASS });
            }
        }
    }

    /// Figura sencilla de una persona (torso + cabeza + cabello), sentada o de pie.
    /// `base_y` es la altura del asiento (o el piso, si esta de pie).
    fn person(&mut self, x: i32, z: i32, base_y: i32, shirt: u8, standing: bool) {
        let torso_top = if standing { base_y + 3 } else { base_y + 2 };
        self.f((x, base_y + 1, z), (x, torso_top, z), shirt);
        self.s(x, torso_top + 1, z, b::SKIN);
        self.s(x, torso_top + 2, z, b::HAIR);
    }
}

/// Entrada exterior: plaza hundida frente a la puerta, gradas subiendo hasta el nivel de la
/// calle, muros de ladrillo a los lados (como una "areaway" de verdad), baranda verde,
/// farol y un bote de basura -- para que se vea como la entrada del bar de la serie
/// (bajas unas gradas desde la acera para llegar a la puerta).
fn build_entrance_exterior(p: &mut P) {
    // Plaza hundida justo frente a la puerta, al mismo nivel que el piso del pub.
    p.f((-3, 0, 1), (-1, 0, 4), b::STONE);

    // Gradas: 4 escalones solidos subiendo desde la plaza (y=0) hasta la calle (y=STREET_Y).
    for step in 0..STREET_Y {
        let x0 = -4 - step * 2;
        let x1 = x0 - 1;
        p.f((x1, 0, 1), (x0, step + 1, 4), b::PAVEMENT);
    }
    let stair_front = -4 - (STREET_Y - 1) * 2 - 1; // borde hacia la calle de la ultima grada

    // Muros de ladrillo a los lados del hueco de las gradas (retienen la tierra, como una
    // areaway real), desde la plaza hasta el nivel de la calle.
    p.f((stair_front, 0, 0), (-1, STREET_Y, 0), b::BRICK);
    p.f((stair_front, 0, 5), (-1, STREET_Y, 5), b::BRICK);

    // Baranda verde en el borde de la calle, enmarcando el hueco de las gradas.
    p.f((stair_front, STREET_Y + 1, 0), (-1, STREET_Y + 1, 0), b::RAILING);
    p.f((stair_front, STREET_Y + 1, 5), (-1, STREET_Y + 1, 5), b::RAILING);
    p.f((stair_front, STREET_Y + 1, 0), (stair_front, STREET_Y + 1, 5), b::RAILING);

    // Acera de la calle: una plataforma solida mas ancha, al nivel STREET_Y.
    let (sx0, sx1) = (stair_front - 15, stair_front - 1);
    let (sz0, sz1) = (-4, 9);
    p.f((sx0, 0, sz0), (sx1, STREET_Y - 1, sz1), b::STONE);
    p.f((sx0, STREET_Y, sz0), (sx1, STREET_Y, sz1), b::PAVEMENT);

    // Farol de calle en una esquina de la acera.
    p.f((sx0 + 1, STREET_Y + 1, sz0 + 1), (sx0 + 1, STREET_Y + 5, sz0 + 1), b::DARKWOOD);
    for &(dx, dz) in &[(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
        p.s(sx0 + 1 + dx, STREET_Y + 6, sz0 + 1 + dz, b::LAMP);
    }

    // Bote de basura pegado a la esquina del muro de las gradas.
    p.f((stair_front, STREET_Y + 1, sz0), (stair_front, STREET_Y + 2, sz0), b::STONE);

    // Luz baja junto a la puerta, para que las gradas no queden a oscuras.
    p.s(-2, 3, 0, b::LAMP);
    p.s(-2, 3, 5, b::LAMP);
}

fn build_pub(w: &mut World) {
    let mut p = P { w };
    let (ww, dd) = (PW, PD);

    // ---- Piso, zocalo, paredes y moldura ----
    p.f((0, 0, 0), (ww - 1, 0, dd - 1), b::FLOOR);
    p.f((0, 4, 0), (ww - 1, 13, 0), b::GREENWALL);
    p.f((0, 4, 0), (0, 13, dd - 1), b::GREENWALL);
    p.f((0, 1, 0), (ww - 1, 3, 0), b::WAINSCOT);
    p.f((0, 1, 0), (0, 3, dd - 1), b::WAINSCOT);
    p.f((0, 14, 0), (ww - 1, 14, 0), b::WHITE);
    p.f((0, 14, 0), (0, 14, dd - 1), b::WHITE);
    // Techo (cierra el volumen para que la luz artificial se vea dramatica, como en un
    // bar real de noche): tapa toda la huella, incluidos los dos lados abiertos.
    p.f((0, 15, 0), (ww - 1, 15, dd - 1), b::ROOF);

    // ---- Ventanal grande (pared x=0) y ventana chica (pared z=0) ----
    p.window_x(6, 15, 4, 13, 3);
    p.window_z(2, 7, 6, 11, 3);

    // ---- Entrada de ladrillo con puerta abierta (pared x=0) ----
    p.f((0, 1, 0), (0, 14, 5), b::BRICK);
    p.f((0, 1, 1), (0, 9, 4), b::AIR);
    // Ventana chica arriba de la puerta (como el piso de arriba del edificio) y un letrero
    // iluminado horizontal, con un aplique de luz a cada lado (igual que la referencia real).
    p.window_x(1, 4, 10, 12, 2);
    p.f((1, 9, 0), (4, 9, 0), b::NEON);
    p.s(0, 8, -1, b::LAMP);
    p.s(0, 8, 6, b::LAMP);

    build_entrance_exterior(&mut p);

    // ---- Repisa y espejo detras de la barra (pared z=0) ----
    p.f((11, 4, 0), (27, 13, 0), b::MIRROR);
    for &x in &[11, 19, 27] {
        p.f((x, 1, 1), (x, 14, 1), b::DARKWOOD);
    }
    for &y in &[3, 7, 11] {
        p.f((11, y, 1), (27, y, 1), b::DARKWOOD);
    }
    for &(a, bnd) in &[(12, 18), (20, 26)] {
        for x in a..bnd {
            for &y in &[5, 9, 12] {
                let r = rnd(x, y, 55);
                let blk = if r < 0.18 {
                    b::AIR
                } else if r < 0.5 {
                    b::AMBER
                } else if r < 0.78 {
                    b::GREENGLASS
                } else {
                    b::BLUEGLASS
                };
                if blk != b::AIR {
                    p.s(x, y, 1, blk);
                }
            }
        }
    }

    // ---- Puerta roja con picaporte + aire acondicionado (pared z=0) ----
    p.f((30, 1, 0), (33, 9, 0), b::DOOR);
    p.f((29, 1, 0), (29, 9, 0), b::WHITE);
    p.f((34, 1, 0), (34, 9, 0), b::WHITE);
    p.s(29, 5, 0, b::GOLD);
    p.f((29, 11, 1), (33, 13, 1), b::WHITE);
    p.f((30, 12, 1), (32, 12, 1), b::STONE);

    // ---- Barra: mueble, tapa con vuelo, pasamanos dorado y bancos ----
    p.f((11, 1, 6), (27, 4, 8), b::DARKWOOD);
    p.f((10, 5, 5), (28, 5, 9), b::BARTOP);
    p.f((11, 1, 9), (27, 1, 9), b::GOLD);
    p.s(15, 3, 7, b::AMBER);
    p.s(16, 3, 7, b::AMBER);
    p.s(22, 3, 7, b::MIRROR);
    p.s(23, 3, 7, b::MIRROR);
    p.pendant(15, 7, 14, 10);
    p.pendant(23, 7, 14, 10);
    for &x in &[13, 17, 21, 25] {
        p.barstool(x, 11);
    }
    // Clientes en la barra (uno de los 4 bancos queda libre) y el cantinero detras
    p.person(13, 11, 4, b::SHIRT_BLUE, false);
    p.person(21, 11, 4, b::SHIRT_MUSTARD, false);
    p.person(25, 11, 4, b::SHIRT_MAROON, false);
    p.person(19, 3, 1, b::SHIRT_MAROON, true);

    // ---- Salon principal: cabinas y mesas con sillas ----
    p.booth(1, 15);
    p.booth(1, 23);

    p.leg_table(14, 18);
    p.chair(10, 19, 2);
    p.chair(19, 19, 3);
    p.pendant(15, 19, 15, 11);
    p.person(10, 19, 3, b::SHIRT_MUSTARD, false);

    p.leg_table(26, 18);
    p.chair(22, 19, 2);
    p.chair(31, 19, 3);
    p.pendant(27, 19, 15, 11);
    p.person(22, 19, 3, b::SHIRT_BLUE, false);

    p.leg_table(20, 27);
    p.chair(16, 28, 2);
    p.chair(25, 28, 3);
    p.chair(21, 24, 0);
    p.pendant(21, 28, 15, 11);
    p.person(25, 28, 3, b::SHIRT_MAROON, false);

    p.pendant(4, 19, 15, 12);
    p.pendant(4, 27, 15, 12);
    p.person(2, 16, 2, b::SHIRT_BLUE, false);
    p.person(2, 29, 2, b::SHIRT_MUSTARD, false);

    // ---- Iluminacion adicional: aplique de pared y tira de neon sobre la repisa ----
    p.s(1, 10, 14, b::LAMP);
    p.f((11, 14, 0), (27, 14, 0), b::NEON);

    // ---- Cuadros enmarcados en el tramo libre de pared verde (junto a la ventana chica) ----
    for &(px, py) in &[(9, 7), (9, 10), (28, 8)] {
        p.s(px, py, 0, b::PICTURE);
    }
}
