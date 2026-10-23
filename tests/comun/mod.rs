//! Ayudas que comparten las pruebas de integración.
//!
//! Viven en `tests/comun/mod.rs`, y no en `tests/comun.rs`, a propósito: Cargo
//! compila cada archivo de la raíz de `tests/` como una suite aparte, así que
//! `tests/comun.rs` saldría en cada corrida como una suite con «running 0 tests».
//! Las subcarpetas de `tests/` no son suites: cada prueba trae este módulo con
//! `mod comun;`.
//!
//! El servidor es de mentira y solo usa la biblioteca estándar. Escucha en
//! `127.0.0.1`, en un puerto que elige el sistema, así que las pruebas no salen
//! a internet ni chocan entre sí.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

/// Levanta el servidor en un hilo y devuelve su dirección, como `127.0.0.1:41233`.
///
/// Rutas:
/// - `/ok`: 200 al instante.
/// - `/error`: 500 al instante.
/// - `/lento/<ms>`: 200 después de esperar esos milisegundos.
/// - `/mudo`: acepta la conexión y nunca contesta.
/// - `/enorme`: 200 y un cuerpo que no se acaba nunca.
/// - `/goteo`: 200, promete un cuerpo grande y lo manda de a un byte cada 100 ms.
pub fn servidor() -> String {
    let escucha = TcpListener::bind("127.0.0.1:0").expect("hay un puerto libre");
    let direccion = escucha
        .local_addr()
        .expect("el puerto tiene dirección")
        .to_string();
    thread::spawn(move || {
        for conexion in escucha.incoming().flatten() {
            thread::spawn(move || atender(conexion));
        }
    });
    direccion
}

fn atender(mut conexion: TcpStream) {
    let mut bufer = [0u8; 2048];
    let leidos = conexion.read(&mut bufer).unwrap_or(0);
    let peticion = String::from_utf8_lossy(&bufer[..leidos]);
    // La ruta sin su query string: /ok?token=x se atiende como /ok.
    let ruta = peticion
        .split_whitespace()
        .nth(1)
        .and_then(|r| r.split('?').next())
        .unwrap_or("/")
        .to_string();

    match ruta.as_str() {
        "/ok" => responder(&mut conexion, "200 OK"),
        "/error" => responder(&mut conexion, "500 Internal Server Error"),
        "/mudo" => thread::sleep(Duration::from_secs(30)),
        "/enorme" => {
            // Sin Content-Length y con Connection: close, el cuerpo dura hasta
            // que alguien cierre. Se escribe hasta que el cliente cuelgue.
            let _ = conexion.write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n");
            let trozo = [b'x'; 16 * 1024];
            while conexion.write_all(&trozo).is_ok() {}
        }
        "/goteo" => {
            let _ = conexion.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\nConnection: close\r\n\r\n",
            );
            while conexion.write_all(b"x").is_ok() {
                thread::sleep(Duration::from_millis(100));
            }
        }
        otra => match otra.strip_prefix("/lento/").and_then(|ms| ms.parse().ok()) {
            Some(ms) => {
                thread::sleep(Duration::from_millis(ms));
                responder(&mut conexion, "200 OK");
            }
            None => responder(&mut conexion, "404 Not Found"),
        },
    }
}

fn responder(conexion: &mut TcpStream, estado: &str) {
    let respuesta =
        format!("HTTP/1.1 {estado}\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok");
    let _ = conexion.write_all(respuesta.as_bytes());
}
