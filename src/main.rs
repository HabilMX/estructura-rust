//! El programa `revisor`: lee las opciones, llama a la biblioteca y decide el
//! código de salida.
//!
//! Es delgado a propósito. Desde `tests/` no se puede importar nada de aquí:
//! un binario no exporta nada. Lo que merezca una prueba propia va en la
//! biblioteca (`src/lib.rs` y sus módulos); este archivo solo se prueba
//! ejecutándolo entero, como hace `tests/binario.rs`.
//!
//! Códigos de salida: 0 todo arriba · 1 alguno lento o caído · 2 no se pudo
//! revisar (opciones inválidas o configuración rota).

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use revisor::{config, reporte, revisar, servicio};

#[derive(Parser)]
#[command(version, about = "Revisa si una lista de servicios responde por HTTP")]
struct Opciones {
    /// El archivo YAML con la lista de servicios.
    #[arg(long, default_value = "servicios.yaml")]
    config: PathBuf,

    /// Cómo se escribe el reporte.
    #[arg(long, value_enum, default_value_t = Formato::Tabla)]
    formato: Formato,

    /// Cuántos servicios se consultan a la vez (1 o más).
    #[arg(long, default_value_t = NonZeroUsize::new(revisar::PARALELO_POR_OMISION).unwrap())]
    paralelo: NonZeroUsize,
}

#[derive(Clone, Copy, ValueEnum)]
enum Formato {
    Tabla,
    Json,
}

#[tokio::main]
async fn main() -> ExitCode {
    let opciones = Opciones::parse();

    let servicios = match config::leer(&opciones.config) {
        Ok(servicios) => servicios,
        Err(error) => {
            eprintln!("revisor: {error}");
            return ExitCode::from(2);
        }
    };
    let Ok(cliente) = revisar::cliente() else {
        eprintln!("revisor: no se pudo preparar el cliente HTTP");
        return ExitCode::from(2);
    };

    let resultados = revisar::revisar_todos(&cliente, &servicios, opciones.paralelo.get()).await;
    match opciones.formato {
        Formato::Tabla => print!("{}", reporte::tabla(&resultados)),
        Formato::Json => println!("{}", reporte::json(&resultados)),
    }

    if servicio::todos_arriba(&resultados) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
