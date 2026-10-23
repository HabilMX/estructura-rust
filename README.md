> El código es MIT; el texto y las explicaciones son CC BY-SA 4.0.

# estructura-rust

**Una plantilla de Rust que se clona, corre a la primera y deja probar desde fuera lo que importa**:
la lógica en una biblioteca (`src/lib.rs`), un `main.rs` delgado, las pruebas unitarias junto al
código y las de integración en `tests/`, y nada más hasta que haga falta.

Es una **semilla**, no un molde completo: trae una funcionalidad real de punta a punta (una línea de
comandos que revisa si otros servicios responden por HTTP), sus pruebas y un CI que comprueba la
frontera que el compilador vigila por ti: lo que no declaras `pub` no se ve desde fuera. Lo demás
crece cuando duela, y la sección [«Cómo crece»](#cómo-crece-cuándo-agregar-cada-cosa) dice cuándo.

Es parte de la serie «La estructura de un proyecto», de Hábil. El ejemplo es el `revisor` del
[curso de Rust](https://www.habil.mx/es/cursos/rust/).

---

## Córrelo

Necesitas Rust 1.85 o más nuevo (`cargo --version`), instalado con `rustup`, más Git y `bash`.

- **Linux Mint 22**: no uses `apt install cargo`. Mint 22 se basa en Ubuntu 24.04, que trae Rust
  1.75, y con él esta plantilla no arranca: ``feature `edition2024` is required`` (medido). Ese mismo
  error sugiere agregar `cargo-features = ["edition2024"]` a `Cargo.toml`: no lo hagas, actualiza
  Rust. Si ya lo instalaste con `apt`, quítalo primero; si no, el instalador de `rustup` avisa
  ``warn: It looks like you have an existing installation of Rust at: /usr/bin`` (medido). Luego
  instala un compilador de C y `rustup`, como dicen el Rust Book y rust-lang.org:

  ```bash
  sudo apt remove cargo rustc          # solo si lo instalaste con apt
  sudo apt install build-essential curl
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

  Sin `build-essential`, la compilación se detiene en ``error: linker `cc` not found`` (medido). La
  [lección 0 del curso de Rust](https://www.habil.mx/es/cursos/rust/00-preparacion/) lo explica paso a
  paso.
- **macOS**: instala las herramientas de línea de comandos de Apple (traen el compilador de C) y luego
  `rustup`:

  ```bash
  xcode-select --install
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Si ya tienes `rustup`**: `rustup update stable`.
- **Después de instalar**, abre otra terminal y comprueba que `cargo --version` diga 1.85 o más. En la
  misma terminal donde corriste el instalador, `cargo` todavía no se encuentra, o sigue siendo el viejo
  (medido).
- **Windows**: usa WSL; el guion de la frontera pide `bash`.

**1. Corre las pruebas** (la primera vez baja y compila las dependencias; tarda un par de minutos):

```bash
cargo test --locked
```

`--locked` usa exactamente las versiones de `Cargo.lock`. La salida trae cinco bloques, uno por cada
cosa que Cargo prueba, y cada uno cierra con su `test result:`. Éstas son las líneas que importan, en
orden (las rutas, recortadas; los tiempos cambian):

```
     Running unittests src/lib.rs (…)
test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running unittests src/main.rs (…)
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/binario.rs (…)
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.33s
     Running tests/integracion.rs (…)
test result: ok. 13 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.13s
   Doc-tests revisor
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Son 49 pruebas. Cuéntalas en esas líneas, no en el código de salida: si un archivo de `tests/` no
compila, su bloque no aparece.

**2. Comprueba la frontera**:

```bash
bash herramientas/probar-frontera.sh
```

Termina en `✅ frontera: se sostiene` (más abajo se explica qué hace). En macOS funciona también con
el bash que trae el sistema (`/bin/bash`, la versión 3.2), y el CI lo comprueba en una Mac.

**3. Úsalo.** Lee `servicios.yaml` de la carpeta actual:

```bash
cargo run --locked
```

Lo que contestó el 22-oct-2026 (los tiempos cambian, y los dos primeros salen `ARRIBA` solo si tienes
internet):

```
SERVICIO           ESTADO  TIEMPO  DETALLE
rust-lang.org      ARRIBA   625ms  200
doc.rust-lang.org  ARRIBA   274ms  200
no-existe          CAIDO        —  no se pudo conectar

3 servicios · 2 arriba · 0 lentos · 1 caído
```

Sale con código `1` porque uno está caído (`echo $?`). Los códigos son tres: `0` todo arriba, `1`
alguno lento o caído, `2` no se pudo revisar (opciones inválidas o configuración rota). También:
`cargo run -- --formato json`, `cargo run -- --config otra.yaml`, `cargo run -- --paralelo 10` y
`cargo run -- --version`.

---

## Qué es cada carpeta y por qué existe

```
estructura-rust/
├── Cargo.toml                   el paquete: nombre, edición, versión mínima de Rust, dependencias
├── Cargo.lock                   las versiones exactas con que se probó: se versiona
├── servicios.yaml               el ejemplo que usa «Córrelo»
├── src/
│   ├── main.rs                  EL PROGRAMA: opciones, llamada a la biblioteca, código de salida
│   ├── lib.rs                   LA BIBLIOTECA: declara los módulos y qué es público
│   ├── servicio.rs              el vocabulario: Servicio, Estado, Resultado. Sin red ni archivos
│   ├── config.rs                lee y valida servicios.yaml
│   ├── revisar.rs               consulta muchos servicios a la vez, con tiempo límite
│   └── reporte.rs               tabla para personas, JSON para programas
├── tests/                       pruebas de integración: cada archivo es un crate aparte
│   ├── integracion.rs           la biblioteca vista desde fuera
│   ├── binario.rs               el programa ejecutado entero
│   └── comun/mod.rs             el servidor HTTP de prueba que comparten las dos
├── herramientas/
│   └── probar-frontera.sh       demuestra que lo que no es pub no se ve desde tests/
└── .github/workflows/verificar.yml
```

**`src/lib.rs` y `src/main.rs`.** Cargo encuentra los dos solo, sin configurar nada: son sus rutas
convencionales. Uno es la biblioteca y el otro el programa, y el programa usa la biblioteca como lo
haría cualquier otro crate: solo con lo que es `pub`. ¿Por qué partirlo así? Porque una prueba de
integración solo puede importar una biblioteca. El Rust Book lo dice en su capítulo de pruebas: si el
proyecto solo tiene `src/main.rs`, no se pueden importar sus funciones desde `tests/`, porque «Only
library crates expose functions that other crates can use». Medido: aunque la función de `main.rs`
sea `pub`, `tests/` contesta ``error[E0432]: unresolved import `revisor` ``.

**`src/main.rs`.** Delgado a propósito: lee las opciones con `clap`, arma las piezas de la biblioteca,
imprime y decide el código de salida. Si algo de ahí merece una prueba propia, ya no pertenece ahí.
Cargo también lo prueba como bloque (`unittests src/main.rs`, 0 pruebas); al programa entero lo
ejecuta `tests/binario.rs`.

**`src/servicio.rs`.** El dominio. No importa a ningún otro módulo del proyecto, y todos los demás lo
importan a él. Trae el único ejemplo de la documentación que también es prueba (`clasificar`): Cargo
corre los ejemplos de la documentación de la biblioteca, no los del programa.

**Un archivo por módulo, sin `mod.rs`.** Cada `pub mod servicio;` de `lib.rs` le dice al compilador
que busque `src/servicio.rs`. El Rust Book llama a esa ruta la más idiomática; `src/servicio/mod.rs`
es el estilo anterior, que todavía funciona. Si un módulo tiene los dos archivos a la vez, el compilador
se detiene: ``error[E0761]: file for module `reporte` found at both "src/reporte.rs" and
"src/reporte/mod.rs"`` (medido).

**`src/revisar.rs`.** La consulta concurrente, con `reqwest` y `tokio`, hasta `--paralelo` a la vez.
Cada consulta tiene **un tiempo límite que cubre todo**: el nombre del servidor, la conexión, los
encabezados y el cuerpo. Lee como mucho 64 KiB del cuerpo y deja de leer. Y **ningún detalle de error
repite la URL**: el texto de los errores de `reqwest` trae la URL completa, con su query string, así
que se traduce a frases fijas. Cada una de esas protecciones tiene su prueba. Un matiz medido: si el
servidor de nombres no contesta, el reporte sale a tiempo, pero el programa tarda en **terminar** lo
que tarde el sistema en rendirse (en Linux, unos 10 s). La consulta del nombre corre en un hilo del
sistema que nadie puede interrumpir, y `tokio` espera a que acabe antes de cerrar.

**`src/reporte.rs`.** La URL de cada servicio sale **sin usuario, contraseña, query string ni
fragmento**: lo que sale en un reporte termina en bitácoras ajenas. La ruta sí sale, porque dice qué se
revisa; por eso un token tampoco va en la ruta. La función que limpia la URL, `url_publica`, es
**privada**: sus pruebas viven al final del mismo archivo, que es donde Rust deja probar lo privado.

**`src/config.rs`.** Rechaza un campo desconocido (`tiemout`), un nombre repetido, un esquema que no
sea `http` o `https`, una URL con credenciales (`https://usuario:clave@...`) y un tiempo límite fuera
de 1 a 60,000 ms. Ningún error repite la URL. Ojo con un detalle medido: los errores de tipo de
`yaml_serde` repiten el valor que encontraron (`invalid type: string "https://...?token=..."`). Por
eso la plantilla nunca imprime el texto de esos errores: lee el archivo como YAML genérico, de un error
de sintaxis dice solo la línea y la columna, y revisa cada campo con mensajes propios
(`servicio 1: timeout_ms debe ser un número entero de milisegundos`). Tapar lo que va entre comillas
no basta: un valor con comillas escapadas (`\"`) despista al que tapa, y el secreto sale (medido).

**`tests/`.** Cargo compila cada archivo de la raíz de `tests/` como un crate aparte, que solo ve lo
`pub` de la biblioteca: así la prueban como la usaría otro programa. Las pruebas de red usan un servidor
HTTP de prueba en `127.0.0.1`, en un puerto libre: no salen a internet.

**`tests/comun/mod.rs`.** Las ayudas compartidas van en una subcarpeta. Un `tests/comun.rs` también
compilaría, pero Cargo lo trataría como una suite más y saldría en cada corrida como
`Running tests/comun.rs` con `running 0 tests` (medido).

**`Cargo.lock`.** Se versiona. Con él, `cargo test --locked` usa en cada máquina y en el CI las mismas
versiones con que se probó la plantilla; sin él, `--locked` se niega a correr:
`cannot create the lock file … because --locked was passed to prevent this` (medido).

---

## La frontera que sabe fallar

`herramientas/probar-frontera.sh` trabaja en una copia temporal del proyecto (tu carpeta no se toca).
Siembra `tests/frontera.rs`, una prueba que llama a `revisor::reporte::url_publica`, y la compila dos
veces sobre la misma copia. Entre las dos solo cambia una palabra de `src/reporte.rs`: `pub`.

1. **Control**, con `pub fn url_publica`: la prueba tiene que compilar y pasar. Si no, el experimento
   está mal montado y el guion para en rojo.
2. **Violación**, con `src/reporte.rs` tal como está en tu proyecto: la misma prueba tiene que **no
   compilar**, y por la razón exacta, `error[E0603]`, en la prueba y no en la biblioteca.

Ésta es la salida real:

```
1/2 control: con «pub fn url_publica», una prueba de tests/ la llama
   ✅ compiló y pasó (1 de 1): la prueba sembrada está bien escrita
2/2 violación: la misma prueba, con src/reporte.rs tal como está
     error[E0603]: function `url_publica` is private
        --> tests/frontera.rs:6:27
   ✅ el compilador la rechazó: lo que no es pub no se ve desde tests/
✅ frontera: se sostiene
```

Si alguien vuelve pública la función, sale `❌ COMPILÓ: una prueba de tests/ pudo llamar a
revisor::reporte::url_publica. La frontera no existe.` Si la prueba falla por otra razón, también sale
en rojo. El conteo del control se lee del informe de la prueba (`test result: ok. 1 passed`), no del
código de salida de cargo.

**Rómpelo tú.** Escribe una prueba que use la función privada:

```bash
cat > tests/mia.rs <<'EOF'
#[test]
fn quita_el_token() {
    assert_eq!(
        revisor::reporte::url_publica("https://pagos.ejemplo.com/salud?token=x"),
        "https://pagos.ejemplo.com/salud"
    );
}
EOF
cargo test --locked --test mia
```

La primera línea del error:

```
error[E0603]: function `url_publica` is private
```

`pub(crate)` tampoco alcanza: hace visible la función en todo el crate de la biblioteca, y `tests/mia.rs`
es otro crate (el mismo `E0603`, medido). Con `pub` sí compila, y la frontera se pone en rojo. Para
regresar todo: `git checkout -- . && git clean -fd`. Ojo: `git clean -fd` también borra cualquier
archivo nuevo que hayas creado en el repositorio, como tus notas.

---

## Qué NO trae y por qué

| No trae | Por qué |
|---|---|
| Workspace | Un workspace junta varios paquetes que comparten `Cargo.lock` y carpeta de compilación. Aquí hay un paquete |
| `examples/`, `benches/` | No hay nada que mostrar aparte ni nada que medir todavía. Ver «Cómo crece» |
| `build.rs` | No se genera código ni se compila C propio antes de compilar |
| `src/bin/` | Hay un solo programa |
| `mod.rs` | Un archivo por módulo, con el nombre del módulo (ver arriba) |
| `utils.rs`, `helpers.rs`, `common.rs` | Un nombre que no dice qué hay adentro se vuelve el cajón donde cae todo. Cada módulo se llama por lo que hace |
| `pub` de más | Lo que es `pub` en la biblioteca es lo que otros pueden usar. Si la publicas, quitarlo es un cambio mayor; y aunque no la publiques, el compilador deja de avisarte si nadie lo usa (con `pub(crate)` dice ``function … is never used``; con `pub`, se calla). Lo demás se queda privado |
| Base de datos, Docker, logging configurable | Fuera del ejemplo. Ver «Cómo crece» |

---

## Cómo crece: cuándo agregar cada cosa

| Cuando… | Agrega… | Y deja… |
|---|---|---|
| Un módulo crece y mezcla dos temas | dos módulos con nombre de lo que hacen (`src/notificar.rs`) | sin `utils.rs` |
| Un módulo ya tiene submódulos | `src/notificar.rs` con sus `mod` y la carpeta `src/notificar/` para ellos | sin `mod.rs` |
| Una función debe verse en todo el crate pero no fuera | `pub(crate)` | `pub` para lo que otro crate de verdad usa |
| Quieres mostrar cómo se usa la biblioteca | `examples/` (Cargo compila sus ejemplos con `cargo test`) | la documentación con su ejemplo |
| Necesitas medir el rendimiento | `benches/` | — |
| Hay un segundo programa | `src/bin/<nombre>.rs`, que usa la misma biblioteca | `src/main.rs` para el principal |
| Hay un segundo paquete (otra biblioteca, otro programa con su propio `Cargo.toml`) | un workspace en la raíz, con los dos paquetes | un `Cargo.lock` para todos |
| Guardas estado en una base de datos | un módulo que hable con la base | `servicio.rs` sin SQL |
| Se necesita un contenedor | un `Dockerfile` en la raíz que copie el binario de `cargo build --release` | — |

Regla para decidir: **una carpeta nueva tiene que resolver un problema que ya tienes**. Si no puedes
decir cuál, todavía no la necesitas.

---

## Cómo quitar el ejemplo

El `revisor` está para demostrar la estructura. Para empezar tu propio proyecto:

1. En `Cargo.toml`, cambia `name` y `description`, y quita las dependencias que no uses
   (`reqwest`, `tokio`, `yaml_serde`…). Corre `cargo build` para que `Cargo.lock` se ajuste.
2. Borra `servicio.rs`, `config.rs`, `revisar.rs`, `reporte.rs`, `servicios.yaml` y sus líneas
   `pub mod` en `lib.rs`. Escribe tus módulos con la misma forma: un archivo por módulo y sus pruebas
   unitarias al final.
3. Deja `main.rs` delgado: opciones, llamada a la biblioteca, código de salida.
4. Reescribe `tests/integracion.rs` y `tests/binario.rs` para tu biblioteca y tu programa; cambia el
   servidor de `tests/comun/mod.rs` por las ayudas que necesites, o bórralo.
5. En `herramientas/probar-frontera.sh`, cambia `archivo`, `funcion` y `camino` por una función privada
   tuya, y la llamada de la prueba sembrada por una que tenga sentido para ella.
6. Corre `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked && bash herramientas/probar-frontera.sh`.
7. Reescribe este README. El código y `LICENSE` son MIT; si conservas el texto de los `.md`, mira
   `LICENSE-CONTENIDO.md`.

---

## Versiones

Medidas el 22 de octubre de 2026.

| Pieza | Versión |
|---|---|
| Rust | Edición 2024, `rust-version = "1.85"` (la primera con la edición 2024). Probado con 1.99.1, la estable de ese día, en macOS arm64 (con el `/bin/bash` 3.2 del sistema) y en Linux arm64 (contenedor `ubuntu:24.04`), y con 1.85.0 en macOS arm64. Linux x86-64 todavía no se ha probado |
| Dependencias | clap 4.6.7, reqwest 0.13.5, serde 1.0.229, serde_json 1.0.154, tokio 1.54.1, yaml_serde 0.10.7; las exactas, con las de ellas, en `Cargo.lock` |
| CI | GitHub Actions: `actions/checkout@v4`, `dtolnay/rust-toolchain`, con Rust estable y 1.85.0 en Linux, y la estable en macOS con `/bin/bash`; corre en cada cambio, a mano y cada lunes. GitHub apaga la corrida de los lunes si el repositorio público pasa 60 días sin actividad |

---

## Licencia

El código es [MIT](LICENSE). El texto y las explicaciones son
[CC BY-SA 4.0](LICENSE-CONTENIDO.md).
