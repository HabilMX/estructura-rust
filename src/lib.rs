//! El `revisor`: lee una lista de servicios, los consulta por HTTP y dice cuáles
//! responden.
//!
//! La lógica vive aquí, en la biblioteca, y `main.rs` solo lee las opciones y
//! llama. Así, todo lo de abajo se puede probar desde `tests/`: una prueba de
//! integración solo puede usar lo que una biblioteca declara `pub`.
//!
//! Cada línea `mod` le dice al compilador que busque `src/<nombre>.rs`. Sin esa
//! línea, el archivo no existe para el programa, aunque esté en la carpeta.
//!
//! - [`servicio`]: el vocabulario (`Servicio`, `Estado`, `Resultado`). Sin red ni archivos.
//! - [`config`]: lee y valida `servicios.yaml`.
//! - [`revisar`]: consulta un servicio por HTTP, o muchos a la vez.
//! - [`reporte`]: convierte los resultados en tabla o en JSON.

pub mod config;
pub mod reporte;
pub mod revisar;
pub mod servicio;
