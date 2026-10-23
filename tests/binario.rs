//! Pruebas del programa de punta a punta. De `main.rs` no se puede importar nada,
//! así que se ejecuta entero: Cargo compila el binario antes de estas pruebas y
//! deja su ruta en `CARGO_BIN_EXE_revisor`.

mod comun;

use std::path::PathBuf;
use std::process::{Command, Output};

fn revisor(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_revisor"))
        .args(args)
        .output()
        .expect("el binario arranca")
}

/// Un archivo de servicios temporal, con un nombre que no choca con otras pruebas.
struct Archivo(PathBuf);

impl Archivo {
    fn con(nombre: &str, contenido: &str) -> Self {
        let ruta =
            std::env::temp_dir().join(format!("revisor-{}-{nombre}.yaml", std::process::id()));
        std::fs::write(&ruta, contenido).expect("se puede escribir en la carpeta temporal");
        Archivo(ruta)
    }

    fn ruta(&self) -> &str {
        self.0.to_str().expect("la ruta temporal es UTF-8")
    }
}

impl Drop for Archivo {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn texto(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn todo_arriba_sale_con_cero_y_tabla() {
    let d = comun::servidor();
    let archivo = Archivo::con(
        "sano",
        &format!(
            "- nombre: catalogo\n  url: http://{d}/ok\n- nombre: pagos\n  url: http://{d}/ok\n"
        ),
    );
    let r = revisor(&["--config", archivo.ruta()]);
    let salida = texto(&r.stdout);
    assert_eq!(r.status.code(), Some(0), "salida: {salida}");
    assert!(salida.starts_with("SERVICIO"), "salida: {salida}");
    assert!(
        salida.ends_with("2 servicios · 2 arriba · 0 lentos · 0 caídos\n"),
        "salida: {salida}"
    );
}

#[test]
fn uno_caido_sale_con_uno() {
    let d = comun::servidor();
    let archivo = Archivo::con(
        "caido",
        &format!("- nombre: bien\n  url: http://{d}/ok\n- nombre: mal\n  url: http://{d}/error\n"),
    );
    let r = revisor(&["--config", archivo.ruta()]);
    let salida = texto(&r.stdout);
    assert_eq!(r.status.code(), Some(1), "salida: {salida}");
    assert!(
        salida.contains("CAIDO") && salida.contains("500"),
        "salida: {salida}"
    );
}

#[test]
fn el_json_es_json_y_sin_secretos() {
    let d = comun::servidor();
    let archivo = Archivo::con(
        "json",
        &format!("- nombre: catalogo\n  url: http://{d}/ok?token=SECRETO\n"),
    );
    let r = revisor(&["--config", archivo.ruta(), "--formato", "json"]);
    let salida = texto(&r.stdout);
    let valor: serde_json::Value = serde_json::from_str(&salida).expect("la salida es JSON");
    assert_eq!(valor["servicios"][0]["url"], format!("http://{d}/ok"));
    assert!(!salida.contains("SECRETO"), "salida: {salida}");
    assert_eq!(r.status.code(), Some(0));
}

#[test]
fn un_archivo_que_no_existe_sale_con_dos_y_dice_cual() {
    let r = revisor(&["--config", "archivo-que-no-existe.yaml"]);
    let error = texto(&r.stderr);
    assert_eq!(r.status.code(), Some(2), "stderr: {error}");
    assert_eq!(
        error,
        "revisor: no se pudo leer archivo-que-no-existe.yaml: no existe\n"
    );
}

#[test]
fn credenciales_en_la_url_salen_con_dos_sin_repetirlas() {
    let archivo = Archivo::con(
        "credenciales",
        "- nombre: pagos\n  url: https://usuario:clave@pagos.ejemplo.com\n",
    );
    let r = revisor(&["--config", archivo.ruta()]);
    let error = texto(&r.stderr);
    assert_eq!(r.status.code(), Some(2), "stderr: {error}");
    assert!(error.contains("usuario o contraseña"), "stderr: {error}");
    assert!(!error.contains("clave@"), "stderr: {error}");
}

#[test]
fn paralelo_cero_sale_con_dos() {
    let r = revisor(&["--paralelo", "0"]);
    assert_eq!(r.status.code(), Some(2), "stderr: {}", texto(&r.stderr));
}

#[test]
fn version_dice_la_del_paquete() {
    let r = revisor(&["--version"]);
    assert_eq!(
        texto(&r.stdout),
        format!("revisor {}\n", env!("CARGO_PKG_VERSION"))
    );
}
