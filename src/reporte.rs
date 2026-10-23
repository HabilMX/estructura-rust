//! Convierte resultados en algo que se lee: una tabla o JSON.
//!
//! La tabla es para personas; el JSON, para otros programas. No consulta nada
//! ni decide estados: recibe resultados ya hechos.

use reqwest::Url;
use serde::Serialize;

use crate::servicio::{Estado, Resultado};

/// La tabla para la terminal, con el resumen al final.
///
/// ```text
/// SERVICIO       ESTADO  TIEMPO  DETALLE
/// rust-lang.org  ARRIBA   102ms  200
/// no-existe      CAIDO        —  no se pudo conectar
///
/// 2 servicios · 1 arriba · 0 lentos · 1 caído
/// ```
pub fn tabla(resultados: &[Resultado]) -> String {
    let ancho = resultados
        .iter()
        .map(|r| r.servicio.nombre.chars().count())
        .chain(["SERVICIO".len()])
        .max()
        .unwrap_or(0);
    let mut salida = format!(
        "{:<ancho$}  {:<6}  {:>6}  DETALLE\n",
        "SERVICIO", "ESTADO", "TIEMPO"
    );
    for r in resultados {
        salida.push_str(&format!(
            "{:<ancho$}  {:<6}  {:>6}  {}\n",
            r.servicio.nombre,
            r.estado.como_texto(),
            tiempo(r.tiempo_ms),
            r.detalle
        ));
    }
    salida.push('\n');
    salida.push_str(&resumen(resultados));
    salida.push('\n');
    salida
}

/// El reporte para otros programas. Las URL salen limpias, sin secretos.
pub fn json(resultados: &[Resultado]) -> String {
    let (arriba, lento, caido) = contar(resultados);
    let reporte = ReporteJson {
        servicios: resultados
            .iter()
            .map(|r| FilaJson {
                nombre: &r.servicio.nombre,
                url: url_publica(&r.servicio.url),
                estado: r.estado.como_texto(),
                tiempo_ms: r.tiempo_ms,
                detalle: &r.detalle,
            })
            .collect(),
        resumen: ResumenJson {
            total: resultados.len(),
            arriba,
            lento,
            caido,
        },
    };
    serde_json::to_string_pretty(&reporte).expect("el reporte siempre se puede escribir como JSON")
}

#[derive(Serialize)]
struct ReporteJson<'a> {
    servicios: Vec<FilaJson<'a>>,
    resumen: ResumenJson,
}

#[derive(Serialize)]
struct FilaJson<'a> {
    nombre: &'a str,
    url: String,
    estado: &'static str,
    tiempo_ms: Option<u64>,
    detalle: &'a str,
}

#[derive(Serialize)]
struct ResumenJson {
    total: usize,
    arriba: usize,
    lento: usize,
    caido: usize,
}

/// La URL sin usuario, contraseña, query string ni fragmento.
///
/// 🔒 Lo que sale en un reporte termina en bitácoras ajenas, y
/// `https://usuario:clave@host` o `?token=...` llevan un secreto. La ruta sí se
/// conserva, porque dice qué se revisa; por eso un token tampoco va en la ruta.
///
/// Es PRIVADA a propósito: es un detalle de cómo se arma el reporte, no algo que
/// otro programa necesite. Sus pruebas están abajo, en este mismo archivo, y
/// `herramientas/probar-frontera.sh` comprueba que desde `tests/` no se alcanza.
fn url_publica(url: &str) -> String {
    match Url::parse(url) {
        Ok(mut limpia) => {
            // Estas dos solo fallan en URL sin servidor, que config ya rechaza.
            let _ = limpia.set_username("");
            let _ = limpia.set_password(None);
            limpia.set_query(None);
            limpia.set_fragment(None);
            limpia.to_string()
        }
        Err(_) => "(URL ilegible)".to_string(),
    }
}

/// `142ms`, `2.3s` o `—` si no hubo respuesta.
fn tiempo(tiempo_ms: Option<u64>) -> String {
    match tiempo_ms {
        None => "—".to_string(),
        Some(ms) if ms < 1000 => format!("{ms}ms"),
        Some(ms) => format!("{:.1}s", ms as f64 / 1000.0),
    }
}

/// `3 servicios · 1 arriba · 1 lento · 1 caído`.
fn resumen(resultados: &[Resultado]) -> String {
    let (arriba, lento, caido) = contar(resultados);
    let total = resultados.len();
    let plural = |n: usize| if n == 1 { "" } else { "s" };
    format!(
        "{total} servicio{} · {arriba} arriba · {lento} lento{} · {caido} caído{}",
        plural(total),
        plural(lento),
        plural(caido)
    )
}

/// Cuántos quedaron en cada estado: (arriba, lentos, caídos).
fn contar(resultados: &[Resultado]) -> (usize, usize, usize) {
    let cuantos = |estado| resultados.iter().filter(|r| r.estado == estado).count();
    (
        cuantos(Estado::Arriba),
        cuantos(Estado::Lento),
        cuantos(Estado::Caido),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::servicio::Servicio;

    fn resultado(
        nombre: &str,
        url: &str,
        estado: Estado,
        tiempo_ms: Option<u64>,
        detalle: &str,
    ) -> Resultado {
        Resultado {
            servicio: Servicio {
                nombre: nombre.to_string(),
                url: url.to_string(),
                timeout_ms: 2000,
            },
            estado,
            tiempo_ms,
            detalle: detalle.to_string(),
        }
    }

    fn caso() -> Vec<Resultado> {
        vec![
            resultado(
                "pagos",
                "https://pagos.ejemplo.com/salud?token=SECRETO",
                Estado::Arriba,
                Some(87),
                "200",
            ),
            resultado(
                "catalogo-central",
                "http://catalogo.ejemplo.com",
                Estado::Lento,
                Some(2345),
                "200",
            ),
            resultado(
                "nomina",
                "https://nomina.ejemplo.com",
                Estado::Caido,
                None,
                "no se pudo conectar",
            ),
        ]
    }

    #[test]
    fn la_tabla_va_alineada_y_cierra_con_el_resumen() {
        let esperado = "\
SERVICIO          ESTADO  TIEMPO  DETALLE
pagos             ARRIBA    87ms  200
catalogo-central  LENTO     2.3s  200
nomina            CAIDO        —  no se pudo conectar

3 servicios · 1 arriba · 1 lento · 1 caído
";
        assert_eq!(tabla(&caso()), esperado);
    }

    #[test]
    fn el_json_trae_cada_servicio_y_el_resumen() {
        let valor: serde_json::Value = serde_json::from_str(&json(&caso())).unwrap();
        assert_eq!(valor["servicios"][0]["nombre"], "pagos");
        assert_eq!(valor["servicios"][0]["estado"], "ARRIBA");
        assert_eq!(valor["servicios"][0]["tiempo_ms"], 87);
        assert!(
            valor["servicios"][2]["tiempo_ms"].is_null(),
            "sin respuesta, sin tiempo"
        );
        assert_eq!(valor["resumen"]["total"], 3);
        assert_eq!(valor["resumen"]["caido"], 1);
    }

    #[test]
    fn el_json_no_lleva_secretos() {
        let texto = json(&caso());
        assert!(
            !texto.contains("SECRETO"),
            "el JSON filtró el query string: {texto}"
        );
        assert!(texto.contains("https://pagos.ejemplo.com/salud"), "{texto}");
    }

    // Esta prueba usa url_publica, que es privada: se puede porque vive en
    // `tests`, un submódulo hijo de `reporte`, y un hijo ve lo privado de sus
    // ancestros. Desde tests/ no se podría (lo comprueba la frontera).
    #[test]
    fn url_publica_quita_credenciales_query_y_fragmento() {
        assert_eq!(
            url_publica("https://usuario:clave@pagos.ejemplo.com:8443/v1/salud?token=x#arriba"),
            "https://pagos.ejemplo.com:8443/v1/salud"
        );
        assert_eq!(url_publica("http://[::1]:8080/?a=b"), "http://[::1]:8080/");
        assert_eq!(url_publica("no es una url"), "(URL ilegible)");
    }

    #[test]
    fn el_tiempo_se_escribe_corto() {
        assert_eq!(tiempo(None), "—");
        assert_eq!(tiempo(Some(999)), "999ms");
        assert_eq!(tiempo(Some(1000)), "1.0s");
    }

    #[test]
    fn el_resumen_concuerda_en_singular_y_plural() {
        let uno = vec![resultado(
            "a",
            "https://a.ejemplo.com",
            Estado::Lento,
            Some(1500),
            "200",
        )];
        assert_eq!(resumen(&uno), "1 servicio · 0 arriba · 1 lento · 0 caídos");
        assert_eq!(resumen(&[]), "0 servicios · 0 arriba · 0 lentos · 0 caídos");
    }
}
