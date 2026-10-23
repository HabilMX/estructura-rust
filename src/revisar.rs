//! Consulta los servicios: uno por HTTP, o muchos a la vez con un tope.
//!
//! Cuida tres cosas que una plantilla no debe enseñar mal:
//!
//! - **Tiempo límite siempre.** Cada servicio trae el suyo, y cubre la consulta
//!   completa: el nombre del servidor, la conexión, los encabezados y el cuerpo.
//!   Sin él, un servicio que acepta la conexión y nunca contesta colgaría la
//!   revisión.
//! - **El cuerpo se drena con tope.** Se deja de leer en cuanto se juntan
//!   [`TOPE_DRENADO`] bytes: un servicio que contesta 200 con un cuerpo enorme no
//!   nos tiene leyendo hasta el tiempo límite.
//! - **Ningún detalle repite la URL.** El texto de los errores de reqwest
//!   incluye la URL completa, con su query string; aquí se traducen a frases
//!   fijas y su texto nunca se usa.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::servicio::{Resultado, Servicio, clasificar};

/// Lo más que se lee del cuerpo de una respuesta, en bytes, para tirarlo.
pub const TOPE_DRENADO: usize = 64 * 1024;

/// Cuántos servicios se consultan a la vez si nadie dice otra cosa.
pub const PARALELO_POR_OMISION: usize = 5;

const SIN_TIEMPO: &str = "se acabó el tiempo de espera";

/// El cliente HTTP que comparten todas las consultas.
///
/// Se arma una vez: reqwest reutiliza las conexiones entre consultas.
pub fn cliente() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(concat!("revisor/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Consulta UN servicio con un GET y mira qué contesta.
///
/// Nunca devuelve error: «este servicio no responde» es justo lo que el
/// programa quiere reportar, así que el fracaso va dentro del [`Resultado`].
pub async fn consultar(cliente: &reqwest::Client, servicio: &Servicio) -> Resultado {
    let inicio = Instant::now();
    let limite = Duration::from_millis(servicio.timeout_ms);
    let intento = tokio::time::timeout(limite, pedir_y_drenar(cliente, &servicio.url)).await;

    let (codigo, detalle) = match intento {
        Ok(Ok(codigo)) => (Some(codigo), codigo.to_string()),
        Ok(Err(error)) => (None, motivo(&error).to_string()),
        Err(_se_acabo) => (None, SIN_TIEMPO.to_string()),
    };
    let tiempo_ms = codigo.map(|_| u64::try_from(inicio.elapsed().as_millis()).unwrap_or(u64::MAX));
    Resultado {
        servicio: servicio.clone(),
        estado: clasificar(codigo, tiempo_ms),
        tiempo_ms,
        detalle,
    }
}

/// Pide la URL y lee el cuerpo hasta que termina o hasta [`TOPE_DRENADO`].
async fn pedir_y_drenar(cliente: &reqwest::Client, url: &str) -> Result<u16, reqwest::Error> {
    let mut respuesta = cliente.get(url).send().await?;
    let codigo = respuesta.status().as_u16();
    let mut leidos = 0;
    while leidos < TOPE_DRENADO {
        match respuesta.chunk().await? {
            Some(trozo) => leidos += trozo.len(),
            None => break,
        }
    }
    Ok(codigo)
}

/// Traduce un error de reqwest a una frase corta y sin la URL.
fn motivo(error: &reqwest::Error) -> &'static str {
    if error.is_timeout() {
        SIN_TIEMPO
    } else if error.is_connect() {
        "no se pudo conectar"
    } else if error.is_redirect() {
        "demasiadas redirecciones"
    } else if error.is_body() || error.is_decode() {
        "la respuesta llegó incompleta"
    } else {
        "no responde"
    }
}

/// Consulta todos los servicios, hasta `paralelo` a la vez (0 cuenta como 1, y
/// más que el número de servicios cuenta como ese número).
///
/// Devuelve los resultados en el mismo orden que los servicios. Cada consulta
/// tiene su propio tiempo límite, así que un servicio colgado no detiene a los
/// demás más allá del suyo.
pub async fn revisar_todos(
    cliente: &reqwest::Client,
    servicios: &[Servicio],
    paralelo: usize,
) -> Vec<Resultado> {
    // Acotado por los dos lados: con 0 nadie tendría turno y la revisión se
    // quedaría esperando para siempre; con un número enorme, el semáforo de
    // tokio entra en pánico (no acepta más de `Semaphore::MAX_PERMITS`).
    let turnos = Arc::new(Semaphore::new(paralelo.clamp(1, servicios.len().max(1))));
    let mut tareas = JoinSet::new();
    for (posicion, servicio) in servicios.iter().cloned().enumerate() {
        let cliente = cliente.clone();
        let turnos = Arc::clone(&turnos);
        tareas.spawn(async move {
            // Espera turno si ya hay `paralelo` consultas corriendo; lo suelta al terminar.
            let _turno = turnos
                .acquire_owned()
                .await
                .expect("el semáforo nunca se cierra");
            (posicion, consultar(&cliente, &servicio).await)
        });
    }

    let mut hechos = Vec::with_capacity(servicios.len());
    while let Some(hecho) = tareas.join_next().await {
        hechos.push(hecho.expect("una consulta no entra en pánico"));
    }
    hechos.sort_by_key(|(posicion, _)| *posicion);
    hechos.into_iter().map(|(_, resultado)| resultado).collect()
}
