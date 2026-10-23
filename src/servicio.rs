//! El vocabulario del programa: qué es un servicio y en qué estado lo encontramos.
//!
//! Es el módulo del que dependen todos y que no depende de ninguno del
//! proyecto: no abre conexiones, no lee archivos y no sabe de YAML ni de JSON.
//! Por eso se prueba sin red y sin disco.

/// A partir de cuántos milisegundos una respuesta correcta se considera lenta.
pub const UMBRAL_LENTO_MS: u64 = 1000;

/// Lo que se le espera a un servicio que no declara su propio tiempo límite.
pub const TIMEOUT_POR_OMISION_MS: u64 = 2000;

/// Algo que sabemos consultar por HTTP.
///
/// La URL ya viene validada por [`crate::config`]: esquema `http` o `https`,
/// con servidor y sin usuario ni contraseña.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Servicio {
    pub nombre: String,
    pub url: String,
    pub timeout_ms: u64,
}

/// Cómo encontramos un servicio. Son tres, y ninguno más.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Estado {
    Arriba,
    Lento,
    Caido,
}

impl Estado {
    /// La etiqueta que sale en la tabla y en el JSON.
    pub fn como_texto(self) -> &'static str {
        match self {
            Estado::Arriba => "ARRIBA",
            Estado::Lento => "LENTO",
            Estado::Caido => "CAIDO",
        }
    }
}

/// Una fotografía de un servicio en el momento en que se consultó.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resultado {
    pub servicio: Servicio,
    pub estado: Estado,
    /// Cuánto tardó la respuesta; `None` si nunca llegó.
    pub tiempo_ms: Option<u64>,
    /// El código HTTP (`"200"`) o, si no hubo respuesta, por qué.
    pub detalle: String,
}

/// Decide el estado a partir del código HTTP y del tiempo de respuesta.
///
/// Sin código, o con un código 400 o mayor, el servicio está caído. Con una
/// respuesta correcta, está lento si tardó [`UMBRAL_LENTO_MS`] o más.
///
/// Este ejemplo es también una prueba: `cargo test` lo compila y lo corre.
///
/// ```
/// use revisor::servicio::{clasificar, Estado};
///
/// assert_eq!(clasificar(Some(200), Some(80)), Estado::Arriba);
/// assert_eq!(clasificar(Some(200), Some(1500)), Estado::Lento);
/// assert_eq!(clasificar(Some(503), Some(80)), Estado::Caido);
/// assert_eq!(clasificar(None, None), Estado::Caido);
/// ```
pub fn clasificar(codigo: Option<u16>, tiempo_ms: Option<u64>) -> Estado {
    match (codigo, tiempo_ms) {
        (None, _) => Estado::Caido,
        (Some(c), _) if c >= 400 => Estado::Caido,
        (Some(_), Some(ms)) if ms >= UMBRAL_LENTO_MS => Estado::Lento,
        (Some(_), _) => Estado::Arriba,
    }
}

/// `true` solo si todos los servicios están arriba: ni lentos ni caídos.
pub fn todos_arriba(resultados: &[Resultado]) -> bool {
    resultados.iter().all(|r| r.estado == Estado::Arriba)
}

// Las pruebas UNITARIAS viven junto al código, en un módulo que solo se compila
// con `cargo test`. Ese módulo, `tests`, es un submódulo hijo de éste, y un hijo
// ve lo privado de sus ancestros: «items in child modules can use the items in
// their ancestor modules» (Rust Book, 11.3).
#[cfg(test)]
mod tests {
    use super::*;

    fn resultado(estado: Estado) -> Resultado {
        Resultado {
            servicio: Servicio {
                nombre: "pagos".to_string(),
                url: "https://pagos.ejemplo.com".to_string(),
                timeout_ms: TIMEOUT_POR_OMISION_MS,
            },
            estado,
            tiempo_ms: Some(10),
            detalle: "200".to_string(),
        }
    }

    #[test]
    fn sin_codigo_esta_caido() {
        assert_eq!(clasificar(None, Some(10)), Estado::Caido);
    }

    #[test]
    fn un_4xx_o_5xx_esta_caido_aunque_sea_rapido() {
        assert_eq!(clasificar(Some(404), Some(10)), Estado::Caido);
        assert_eq!(clasificar(Some(500), Some(10)), Estado::Caido);
    }

    #[test]
    fn el_400_exacto_ya_esta_caido() {
        assert_eq!(clasificar(Some(399), Some(10)), Estado::Arriba);
        assert_eq!(clasificar(Some(400), Some(10)), Estado::Caido);
    }

    #[test]
    fn una_redireccion_cuenta_como_respuesta() {
        assert_eq!(clasificar(Some(301), Some(10)), Estado::Arriba);
    }

    #[test]
    fn el_umbral_exacto_ya_es_lento() {
        assert_eq!(
            clasificar(Some(200), Some(UMBRAL_LENTO_MS - 1)),
            Estado::Arriba
        );
        assert_eq!(clasificar(Some(200), Some(UMBRAL_LENTO_MS)), Estado::Lento);
    }

    #[test]
    fn las_etiquetas_son_las_del_reporte() {
        assert_eq!(Estado::Arriba.como_texto(), "ARRIBA");
        assert_eq!(Estado::Lento.como_texto(), "LENTO");
        assert_eq!(Estado::Caido.como_texto(), "CAIDO");
    }

    #[test]
    fn todos_arriba_solo_si_ninguno_esta_lento_ni_caido() {
        assert!(todos_arriba(&[resultado(Estado::Arriba)]));
        assert!(!todos_arriba(&[
            resultado(Estado::Arriba),
            resultado(Estado::Lento)
        ]));
        assert!(!todos_arriba(&[resultado(Estado::Caido)]));
    }
}
