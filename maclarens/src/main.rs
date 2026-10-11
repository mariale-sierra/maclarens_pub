//! MacLaren's Pub - diorama de voxeles renderizado con ray tracing en CPU (solo std de Rust).
mod math;
mod png;
mod render;
mod scene;
mod sky;
mod textures;

use math::*;
use render::{Camera, Renderer};
use std::f32::consts::TAU;
use std::fmt::Write as FmtWrite;
use std::io::{self, BufRead, Read, Write};
use std::process::Command;
use std::time::Instant;

struct Opts {
    width: usize,
    height: usize,
    aa: usize,
    threads: usize,
    seed: u32,
    out: String,
    frames: usize,
    az: f32,
    el: f32,
    dist: f32,
    interactive: bool,
    pov: bool,
    live: bool,
    cols: usize,
    rows: usize,
}

fn parse_args() -> Opts {
    let mut o = Opts {
        width: 960,
        height: 540,
        aa: 3,
        threads: std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4),
        seed: 7,
        out: "output".to_string(),
        frames: 0,
        az: 245.0,
        el: 23.0,
        dist: 85.0,
        interactive: false,
        pov: false,
        live: false,
        cols: 120,
        rows: 42,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let val = |i: usize| args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--width" => o.width = val(i).parse().unwrap_or(o.width),
            "--height" => o.height = val(i).parse().unwrap_or(o.height),
            "--aa" => o.aa = val(i).parse().unwrap_or(o.aa),
            "--threads" => o.threads = val(i).parse().unwrap_or(o.threads),
            "--seed" => o.seed = val(i).parse().unwrap_or(o.seed),
            "--out" => o.out = val(i),
            "--frames" => o.frames = val(i).parse().unwrap_or(0),
            "--az" => o.az = val(i).parse().unwrap_or(o.az),
            "--el" => o.el = val(i).parse().unwrap_or(o.el),
            "--dist" => o.dist = val(i).parse().unwrap_or(o.dist),
            "--interactive" => {
                o.interactive = true;
                i += 1;
                continue;
            }
            "--pov" => {
                o.pov = true;
                i += 1;
                continue;
            }
            "--live" => {
                o.live = true;
                i += 1;
                continue;
            }
            "--cols" => o.cols = val(i).parse().unwrap_or(o.cols),
            "--rows" => o.rows = val(i).parse().unwrap_or(o.rows),
            "--help" | "-h" => {
                println!(
                    "Uso: maclarens_pub [--width N] [--height N] [--aa N] [--threads N] [--seed N]\n\
                     \x20                  [--az grados] [--el grados] [--dist d]   (una imagen: <out>/frame.png)\n\
                     \x20                  [--frames N]                              (animacion orbital + zoom)\n\
                     \x20                  [--interactive]                           (camara orbital por teclado)\n\
                     \x20                  [--pov]                                   (caminar, una imagen por comando)\n\
                     \x20                  [--live] [--cols N] [--rows N]            (caminar en tiempo real, en la terminal)\n\
                     \x20                  [--out carpeta]"
                );
                std::process::exit(0);
            }
            _ => {
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    o
}

/// Centro del diorama completo (vista alejada): entre las gradas de la entrada y la barra.
fn world_center() -> V3 {
    v3(32.0, 26.0, 22.0)
}
/// Punto de interes al hacer zoom: el centro del salon (a media altura, para mirar hacia adentro
/// por los lados abiertos del pub).
fn pub_center() -> V3 {
    v3(46.0, 23.0, 20.0)
}

/// Altura de los ojos sobre el piso, en unidades del mundo (para el modo --pov).
const EYE_HEIGHT: f32 = 7.0;

/// Punto de partida al caminar: arriba de las gradas, en la acera -- caminas hacia la
/// puerta y bajas las gradas para entrar al bar, como en la escena real.
fn pov_start() -> (V3, f32) {
    let pos = v3(
        (scene::PX0 - 18) as f32,
        (scene::PADY + 4 + 1) as f32 + EYE_HEIGHT,
        (scene::PZ0 + 2) as f32,
    );
    (pos, 90.0) // yaw=90 -> mirando hacia +x, hacia las gradas y la puerta
}

/// Cuanto puede subir o bajar el piso de un paso a otro sin que se sienta como una pared
/// (las gradas de la entrada suben 1 unidad cada 2 de ancho, asi que con esto alcanza).
const MAX_STEP: i32 = 2;

/// Borde invisible de la plataforma: el diorama tiene dos paredes abiertas (para verse como
/// una maqueta al orbitar), asi que sin esto se podria "caminar" fuera de la base.
fn in_bounds(x: i32, z: i32) -> bool {
    x >= 1 && x <= scene::NX as i32 - 2 && z >= 1 && z <= scene::NZ as i32 - 2
}

/// Busca el primer bloque solido hacia abajo desde `near+8` y devuelve la altura del piso
/// (el primer espacio libre encima de ese bloque). Asi la camara "sigue el terreno": baja
/// las gradas escalon por escalon y sube el pequeno escalon de la puerta, en vez de quedar
/// flotando siempre a la misma altura.
fn ground_feet(world: &scene::World, x: i32, z: i32, near: i32) -> i32 {
    let top = (near + 8).min(scene::NY as i32 - 2);
    let mut y = top;
    while y > 0 && !world.is_solid(x, y, z) {
        y -= 1;
    }
    y + 1
}

/// Movimiento horizontal (w/a/s/d): sigue el piso (sube/baja escalones) y bloquea si hay
/// una pared de verdad (un cambio de altura mayor a MAX_STEP) o si falta espacio de cabeza.
fn try_walk(world: &scene::World, pos: &mut V3, dx: f32, dz: f32) {
    let cur_feet = (pos.y - EYE_HEIGHT).round() as i32;
    for (ndx, ndz) in [(dx, 0.0), (0.0, dz)] {
        if ndx == 0.0 && ndz == 0.0 {
            continue;
        }
        let (nx, nz) = (pos.x + ndx, pos.z + ndz);
        let (gx, gz) = (nx.floor() as i32, nz.floor() as i32);
        if !in_bounds(gx, gz) {
            continue;
        }
        let new_feet = ground_feet(world, gx, gz, cur_feet);
        if (new_feet - cur_feet).abs() > MAX_STEP {
            continue; // pared, o un hueco/caida demasiado brusca
        }
        let mut head_blocked = false;
        for y in new_feet..(new_feet + 5) {
            if world.is_solid(gx, y, gz) {
                head_blocked = true;
                break;
            }
        }
        if head_blocked {
            continue;
        }
        pos.x = nx;
        pos.z = nz;
        pos.y = new_feet as f32 + EYE_HEIGHT;
    }
}

/// Subir/bajar manual (u/o): vuela libremente, solo revisando que no se meta dentro de un
/// bloque solido (para agacharse bajo algo o pararse encima a proposito).
fn try_fly(world: &scene::World, pos: &mut V3, dy: f32) {
    let np = *pos + v3(0.0, dy, 0.0);
    let (x, z) = (np.x.floor() as i32, np.z.floor() as i32);
    let feet = (np.y - EYE_HEIGHT).floor() as i32;
    if !world.is_solid(x, feet, z) && !world.is_solid(x, feet + 1, z) {
        pos.y = np.y;
    }
}

/// Pone la terminal en modo "crudo" (sin esperar Enter, sin eco de teclas) usando el
/// comando `stty` del sistema -- no es una libreria de Rust, es la misma herramienta que
/// cualquier terminal de Linux/WSL ya trae. Al soltarse (Drop) devuelve la terminal a la
/// normalidad, incluso si el programa termina por error.
struct RawMode;
impl RawMode {
    fn enable() -> Self {
        let _ = Command::new("stty").args(["raw", "-echo"]).status();
        print!("\x1b[?25l\x1b[2J"); // oculta el cursor y limpia la pantalla una vez
        io::stdout().flush().ok();
        RawMode
    }
}
impl Drop for RawMode {
    fn drop(&mut self) {
        print!("\x1b[?25h\r\n");
        io::stdout().flush().ok();
        let _ = Command::new("stty").args(["sane"]).status();
    }
}

/// Dibuja un buffer RGB en la terminal usando el caracter "medio bloque superior" (▀):
/// el color de frente pinta el pixel de arriba y el de fondo el de abajo, asi que cada
/// fila de texto muestra 2 filas de pixeles reales (color de 24 bits, sin libreria grafica).
fn print_ansi_frame(img: &[u8], w: usize, h: usize) {
    let mut out = String::with_capacity(w * (h / 2) * 24 + h);
    out.push_str("\x1b[H");
    for row in 0..h / 2 {
        for col in 0..w {
            let ti = (row * 2 * w + col) * 3;
            let bi = ((row * 2 + 1) * w + col) * 3;
            let _ = write!(
                out,
                "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m\u{2580}",
                img[ti], img[ti + 1], img[ti + 2], img[bi], img[bi + 1], img[bi + 2]
            );
        }
        out.push_str("\x1b[0m\r\n");
    }
    print!("{}", out);
    io::stdout().flush().ok();
}

fn main() {
    let o = parse_args();
    std::fs::create_dir_all(&o.out).expect("no se pudo crear la carpeta de salida");

    let t0 = Instant::now();
    let mats = scene::build_materials();
    let world = scene::generate(o.seed, &mats);
    let sky = sky::Skybox::new(512);
    let r = Renderer::new(&world, &mats, &sky);
    println!(
        "Escena lista en {:.2}s | {} materiales | {} luces | {} hilos",
        t0.elapsed().as_secs_f32(),
        mats.len() - 1,
        world.lights.len(),
        o.threads
    );

    let render_to = |cam: &Camera, w: usize, h: usize, path: &str| {
        let t = Instant::now();
        let img = r.render(cam, w, h, o.aa, o.threads);
        png::write_png(path, w, h, &img).expect("no se pudo escribir el PNG");
        println!("  {} ({:.2}s)", path, t.elapsed().as_secs_f32());
    };

    if o.live {
        // Modo en vivo: cada tecla se lee al instante (sin Enter) y la escena se vuelve a
        // trazar y se imprime en la misma terminal con color de 24 bits. Resolucion chica
        // y sin antialiasing para que salga rapido; el ray tracing sigue siendo real
        // (sombras, reflejos, refraccion), solo que a menos pixeles por cuadro.
        let (pw, ph) = (o.cols.max(20), o.rows.max(10) * 2);
        let (mut pos, mut yaw) = pov_start();
        let mut pitch = 0.0f32;
        let _raw = RawMode::enable();
        loop {
            let cam = Camera::first_person(pos, yaw, pitch, 75.0);
            let t = Instant::now();
            let img = r.render(&cam, pw, ph, 1, o.threads);
            print_ansi_frame(&img, pw, ph);
            print!(
                "\x1b[0mw/a/s/d mover, j/l girar, i/k mirar, u/o alto, r reset, q salir  ({:.0} ms)\r\n",
                t.elapsed().as_secs_f64() * 1000.0
            );
            io::stdout().flush().ok();
            let mut buf = [0u8; 1];
            if io::stdin().read_exact(&mut buf).is_err() {
                break;
            }
            let fwd = v3(yaw.to_radians().sin(), 0.0, yaw.to_radians().cos());
            let right = v3(fwd.z, 0.0, -fwd.x);
            match buf[0] {
                b'w' => try_walk(&world, &mut pos, fwd.x * 2.0, fwd.z * 2.0),
                b's' => try_walk(&world, &mut pos, fwd.x * -2.0, fwd.z * -2.0),
                b'd' => try_walk(&world, &mut pos, right.x * 2.0, right.z * 2.0),
                b'a' => try_walk(&world, &mut pos, right.x * -2.0, right.z * -2.0),
                b'u' => try_fly(&world, &mut pos, 1.5),
                b'o' => try_fly(&world, &mut pos, -1.5),
                b'j' => yaw -= 8.0,
                b'l' => yaw += 8.0,
                b'i' => pitch = (pitch + 6.0).min(60.0),
                b'k' => pitch = (pitch - 6.0).max(-60.0),
                b'r' => {
                    let (p0, y0) = pov_start();
                    pos = p0;
                    yaw = y0;
                    pitch = 0.0;
                }
                b'q' | 0x1b => break,
                _ => {}
            }
        }
    } else if o.pov {
        // Modo primera persona: te "metes" al bar caminando, con colision simple contra
        // paredes y muebles. Cada comando re-renderiza <out>/pov.png.
        let (w, h) = (o.width.min(800), o.height.min(450));
        let (mut pos, mut yaw) = pov_start();
        let mut pitch = 0.0f32;
        println!(
            "Modo POV: entras al bar caminando.\n\
             Comandos (+Enter, se pueden combinar, ej: 'wwd'):\n\
             \x20 w/s = caminar adelante/atras   a/d = strafe izquierda/derecha\n\
             \x20 j/l = girar (yaw)               i/k = mirar arriba/abajo (pitch)\n\
             \x20 u/o = subir/bajar (agacharse o pararte en algo)\n\
             \x20 r   = volver a la entrada        q = salir"
        );
        loop {
            let cam = Camera::first_person(pos, yaw, pitch, 75.0);
            render_to(&cam, w, h, &format!("{}/pov.png", o.out));
            println!(
                "  pos=({:.1},{:.1},{:.1}) yaw={:.0} pitch={:.0}",
                pos.x, pos.y, pos.z, yaw, pitch
            );

            print!("> ");
            io::stdout().flush().ok();
            let mut line = String::new();
            if io::stdin().lock().read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            for ch in line.trim().chars() {
                let fwd = v3(yaw.to_radians().sin(), 0.0, yaw.to_radians().cos());
                let right = v3(fwd.z, 0.0, -fwd.x);
                match ch {
                    'w' => try_walk(&world, &mut pos, fwd.x * 3.0, fwd.z * 3.0),
                    's' => try_walk(&world, &mut pos, fwd.x * -3.0, fwd.z * -3.0),
                    'd' => try_walk(&world, &mut pos, right.x * 3.0, right.z * 3.0),
                    'a' => try_walk(&world, &mut pos, right.x * -3.0, right.z * -3.0),
                    'u' => try_fly(&world, &mut pos, 2.0),
                    'o' => try_fly(&world, &mut pos, -2.0),
                    'j' => yaw -= 12.0,
                    'l' => yaw += 12.0,
                    'i' => pitch = (pitch + 8.0).min(60.0),
                    'k' => pitch = (pitch - 8.0).max(-60.0),
                    'r' => {
                        let (p0, y0) = pov_start();
                        pos = p0;
                        yaw = y0;
                        pitch = 0.0;
                    }
                    'q' => return,
                    _ => {}
                }
            }
        }
    } else if o.interactive {
        // Modo interactivo: cada comando re-renderiza una vista previa en <out>/preview.png
        let (mut az, mut el, mut dist) = (o.az, o.el, o.dist);
        let (w, h) = (o.width.min(640), o.height.min(360));
        println!("Comandos (+Enter): a/d = girar, w/s = subir/bajar camara, z = acercar, x = alejar, r = reset, q = salir");
        let stdin = io::stdin();
        loop {
            let zoom = ((85.0 - dist) / 45.0).clamp(0.0, 1.0);
            let target = world_center().lerp(pub_center(), zoom);
            render_to(&Camera::orbit(target, az, el, dist, 30.0), w, h, &format!("{}/preview.png", o.out));
            print!("> ");
            io::stdout().flush().ok();
            let mut line = String::new();
            if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
                break;
            }
            for ch in line.trim().chars() {
                match ch {
                    'a' => az -= 15.0,
                    'd' => az += 15.0,
                    'w' => el = (el + 5.0).min(80.0),
                    's' => el = (el - 5.0).max(5.0),
                    'z' => dist = (dist - 6.0).max(36.0),
                    'x' => dist = (dist + 6.0).min(140.0),
                    'r' => {
                        az = o.az;
                        el = o.el;
                        dist = o.dist;
                    }
                    'q' => return,
                    _ => {}
                }
            }
        }
    } else if o.frames > 0 {
        // Animacion: la camara da una vuelta completa y hace zoom in/out (bucle perfecto).
        for f in 0..o.frames {
            let t = f as f32 / o.frames as f32;
            let zoom = 0.5 - 0.5 * (TAU * t).cos(); // 0 -> 1 -> 0
            let az = o.az + 360.0 * t;
            // Al acercarse la camara baja de altura, para mirar por los lados abiertos del pub
            // en vez de toparse con el techo.
            let el = o.el + 4.0 * (TAU * t).sin() - 17.0 * zoom;
            let dist = o.dist - 40.0 * zoom;
            let target = world_center().lerp(pub_center(), zoom);
            println!("Frame {}/{}  az={:.0} dist={:.1}", f + 1, o.frames, az % 360.0, dist);
            render_to(
                &Camera::orbit(target, az, el, dist, 30.0),
                o.width,
                o.height,
                &format!("{}/frame_{:04}.png", o.out, f),
            );
        }
        println!("Listo. Video: ffmpeg -framerate 30 -i {}/frame_%04d.png -c:v libx264 -pix_fmt yuv420p diorama.mp4", o.out);
    } else {
        let cam = Camera::orbit(world_center(), o.az, o.el, o.dist, 30.0);
        render_to(&cam, o.width, o.height, &format!("{}/frame.png", o.out));
    }
}
