# MacLaren's Pub — Diorama con Ray Tracing

Diorama inspirado en el bar MacLaren's Pub de *How I Met Your Mother*. La escena está construida con cubos texturizados y se renderiza mediante un ray tracer desarrollado en Rust.

Todo el renderizado se realiza en CPU y únicamente se utiliza la biblioteca estándar de Rust, sin crates externos ni aceleración mediante GPU.

## Video


[Ver video del diorama](https://drive.google.com/file/d/1lOE4sDLCmE3hvcmQ9zsSwkAJpF52JPet/view?usp=sharing)

## Escena

El diorama representa el interior del pub e incluye la barra, mesas, sillas, cabinas, lámparas, ventanas, botellas y personajes. También cuenta con una entrada exterior con gradas y una base de terreno generada proceduralmente.

La iluminación está ambientada como un bar durante la noche. Las lámparas, velas y el letrero iluminado permiten apreciar las sombras, los reflejos y los distintos materiales de la escena.

## Ejecución

Se necesita tener instalado Rust y Cargo.

### Generar una imagen

```bash
cargo run --release -- --width 1280 --height 720 --aa 3
```

### Generar la animación

Este comando genera 180 imágenes dentro de la carpeta `output`:

```bash
cargo run --release -- --frames 180 --width 960 --height 540 --aa 2
```

Durante la animación, la cámara gira alrededor del diorama y modifica su distancia.

Para convertir los frames en un video:

```bash
ffmpeg -framerate 30 -i output/frame_%04d.png -c:v libx264 -pix_fmt yuv420p diorama.mp4
```

FFmpeg solamente se utiliza para unir las imágenes generadas por el programa.

### Modo en primera persona

```bash
cargo run --release -- --live
```

Controles:

- `W`, `A`, `S`, `D`: mover la cámara
- `J`, `L`: girar
- `I`, `K`: mirar arriba o abajo
- `R`: reiniciar la posición
- `Q`: salir

## Características implementadas

| Requisito | Implementación |
|---|---|
| Cubos texturizados | Los bloques utilizan texturas procedurales de 16 × 16 píxeles. |
| Materiales | La escena contiene madera, ladrillo, piedra, vidrio y metal, cada uno con sus propios parámetros. |
| Refracción | Se utiliza en las ventanas y botellas de vidrio. |
| Reflexión | Se puede observar en el espejo, el vidrio y algunas superficies barnizadas. |
| Mapas normales | Se aplican al ladrillo y a la piedra para simular relieve. |
| Material emisivo | Las velas, lámparas y el letrero iluminado emiten luz. |
| Skybox | El fondo nocturno contiene un degradado de cielo, luna y estrellas. |
| Terreno procedural | La base utiliza un terreno mayor de 16 × 16 cubos generado mediante ruido. |
| Rotación y zoom | La animación modifica el ángulo y la distancia de la cámara. |
| Programación paralela | Las filas de la imagen se distribuyen entre varios hilos. |

## Optimizaciones

- Renderizado paralelo mediante `std::thread`.
- Recorrido de voxeles con el algoritmo DDA.
- Corte anticipado de rayos de sombra.
- Evaluación de luces según su distancia.
- Texturas y materiales precalculados.
- Compilación optimizada mediante el perfil `release`.

## Estructura del proyecto

```text
src/
├── main.rs       # Argumentos y cámaras
├── render.rs     # Ray tracing, iluminación y paralelización
├── scene.rs      # Construcción del diorama y materiales
├── textures.rs   # Texturas y mapas normales
├── sky.rs        # Skybox
├── math.rs       # Vectores y ruido procedural
└── png.rs        # Generación de archivos PNG
``