//! Lee la lista de servicios de un archivo YAML y la valida.
//!
//! No toca la red. [`leer`] recibe una ruta; [`desde_texto`] recibe el texto ya
//! leído, así que las pruebas no necesitan archivos.
//!
//! El archivo tiene esta forma:
//!
//! ```yaml
//! - nombre: rust-lang.org
//!   url: https://www.rust-lang.org
//!   timeout_ms: 5000      # opcional: por omisión, 2000
//! ```
//!
//! 🔒 Ningún error de este módulo repite una URL: `https://usuario:clave@...` o
//! `?token=...` llevan un secreto, y un mensaje de error termina en bitácoras
//! ajenas. Cada error dice en qué servicio está (por su nombre o su posición) y
//! qué le falta, nunca el valor.
//!
//! Por eso tampoco se imprime NUNCA el texto de los errores de `yaml_serde`: un
//! error de tipo repite el valor que encontró (`invalid type: string "..."`).
//! El archivo se lee primero como YAML genérico, y de un error de sintaxis solo
//! se dicen la línea y la columna. Después cada campo se revisa aquí, con
//! mensajes propios.

use std::fmt;
use std::path::Path;

use reqwest::Url;
use yaml_serde::Value;

use crate::servicio::{Servicio, TIMEOUT_POR_OMISION_MS};

/// El tiempo límite más largo que se acepta, en milisegundos.
pub const TIMEOUT_MAXIMO_MS: u64 = 60_000;

/// Un servicio tal como viene en el archivo, antes de validarlo.
struct Entrada {
    nombre: String,
    url: String,
    timeout_ms: Option<u64>,
}

/// Lo que puede salir mal al leer la lista de servicios.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorConfig {
    /// El archivo no existe o no se pudo leer.
    Lectura { ruta: String, causa: String },
    /// El texto no es YAML; solo se sabe dónde (línea y columna, si las hay).
    Sintaxis {
        linea: Option<usize>,
        columna: Option<usize>,
    },
    /// El YAML no es una lista de servicios.
    NoEsLista,
    /// Al servicio número `posicion` (desde 1) le falta un campo o trae uno
    /// desconocido o de otro tipo. `problema` es una frase fija de este módulo.
    Campo { posicion: usize, problema: String },
    /// La lista está vacía.
    SinServicios,
    /// Dos servicios con el mismo nombre.
    NombreRepetido { nombre: String },
    /// La URL no se puede interpretar o no tiene servidor.
    UrlInvalida { nombre: String },
    /// Un esquema que no es `http` ni `https`.
    EsquemaNoPermitido { nombre: String },
    /// La URL trae usuario o contraseña.
    ConCredenciales { nombre: String },
    /// Un tiempo límite de cero o mayor que [`TIMEOUT_MAXIMO_MS`].
    TimeoutFueraDeRango { nombre: String },
}

impl fmt::Display for ErrorConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorConfig::Lectura { ruta, causa } => write!(f, "no se pudo leer {ruta}: {causa}"),
            ErrorConfig::Sintaxis {
                linea: Some(linea),
                columna: Some(columna),
            } => write!(
                f,
                "el archivo no es YAML válido (línea {linea}, columna {columna})"
            ),
            ErrorConfig::Sintaxis { .. } => write!(f, "el archivo no es YAML válido"),
            ErrorConfig::NoEsLista => write!(f, "el archivo debe ser una lista de servicios"),
            ErrorConfig::Campo { posicion, problema } => {
                write!(f, "servicio {posicion}: {problema}")
            }
            ErrorConfig::SinServicios => write!(f, "el archivo no declara ningún servicio"),
            ErrorConfig::NombreRepetido { nombre } => {
                write!(f, "el nombre «{nombre}» está repetido")
            }
            ErrorConfig::UrlInvalida { nombre } => {
                write!(f, "servicio «{nombre}»: la URL no se puede usar")
            }
            ErrorConfig::EsquemaNoPermitido { nombre } => {
                write!(
                    f,
                    "servicio «{nombre}»: la URL debe empezar con http:// o https://"
                )
            }
            ErrorConfig::ConCredenciales { nombre } => write!(
                f,
                "servicio «{nombre}»: la URL lleva usuario o contraseña; quítalos del archivo"
            ),
            ErrorConfig::TimeoutFueraDeRango { nombre } => write!(
                f,
                "servicio «{nombre}»: timeout_ms debe estar entre 1 y {TIMEOUT_MAXIMO_MS}"
            ),
        }
    }
}

impl std::error::Error for ErrorConfig {}

/// Lee y valida el archivo de servicios.
pub fn leer(ruta: &Path) -> Result<Vec<Servicio>, ErrorConfig> {
    let texto = std::fs::read_to_string(ruta).map_err(|e| ErrorConfig::Lectura {
        ruta: ruta.display().to_string(),
        causa: match e.kind() {
            std::io::ErrorKind::NotFound => "no existe".to_string(),
            otro => otro.to_string(),
        },
    })?;
    desde_texto(&texto)
}

/// Convierte el texto YAML en una lista de servicios validada.
pub fn desde_texto(texto: &str) -> Result<Vec<Servicio>, ErrorConfig> {
    // El texto del error de la biblioteca no se usa: solo dónde ocurrió.
    let documento: Value = yaml_serde::from_str(texto).map_err(|e| {
        let lugar = e.location();
        ErrorConfig::Sintaxis {
            linea: lugar.as_ref().map(|l| l.line()),
            columna: lugar.as_ref().map(|l| l.column()),
        }
    })?;
    let lista = match documento {
        Value::Null => Vec::new(),
        Value::Sequence(lista) => lista,
        _ => return Err(ErrorConfig::NoEsLista),
    };
    if lista.is_empty() {
        return Err(ErrorConfig::SinServicios);
    }

    let mut servicios: Vec<Servicio> = Vec::with_capacity(lista.len());
    for (indice, crudo) in lista.iter().enumerate() {
        let entrada = entrada(indice + 1, crudo)?;
        if servicios.iter().any(|s| s.nombre == entrada.nombre) {
            return Err(ErrorConfig::NombreRepetido {
                nombre: entrada.nombre,
            });
        }
        servicios.push(validar(entrada)?);
    }
    Ok(servicios)
}

fn validar(entrada: Entrada) -> Result<Servicio, ErrorConfig> {
    let nombre = entrada.nombre;
    // El error de `Url::parse` no repite la URL, pero tampoco se usa su texto.
    let url = Url::parse(&entrada.url).map_err(|_| ErrorConfig::UrlInvalida {
        nombre: nombre.clone(),
    })?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(ErrorConfig::EsquemaNoPermitido { nombre });
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(ErrorConfig::UrlInvalida { nombre });
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ErrorConfig::ConCredenciales { nombre });
    }
    let timeout_ms = entrada.timeout_ms.unwrap_or(TIMEOUT_POR_OMISION_MS);
    if timeout_ms == 0 || timeout_ms > TIMEOUT_MAXIMO_MS {
        return Err(ErrorConfig::TimeoutFueraDeRango { nombre });
    }
    Ok(Servicio {
        nombre,
        url: entrada.url,
        timeout_ms,
    })
}

/// Saca los tres campos de un servicio sin repetir nunca un valor.
fn entrada(posicion: usize, crudo: &Value) -> Result<Entrada, ErrorConfig> {
    let problema = |texto: &str| ErrorConfig::Campo {
        posicion,
        problema: texto.to_string(),
    };
    let Value::Mapping(campos) = crudo else {
        return Err(problema("debe tener los campos nombre y url"));
    };
    let (mut nombre, mut url, mut timeout_ms) = (None, None, None);
    for (clave, valor) in campos {
        match clave.as_str() {
            Some("nombre") => {
                nombre = Some(
                    valor
                        .as_str()
                        .ok_or_else(|| problema("nombre debe ser texto"))?,
                );
            }
            Some("url") => {
                url = Some(
                    valor
                        .as_str()
                        .ok_or_else(|| problema("url debe ser texto"))?,
                );
            }
            Some("timeout_ms") => {
                timeout_ms = Some(valor.as_u64().ok_or_else(|| {
                    problema("timeout_ms debe ser un número entero de milisegundos")
                })?);
            }
            // Un campo mal escrito («tiemout») es un error, no un silencio. Su
            // nombre se muestra solo si parece un nombre de campo.
            otra => {
                return Err(problema(&match otra.filter(|c| parece_campo(c)) {
                    Some(campo) => format!("campo desconocido `{campo}`"),
                    None => "un campo desconocido".to_string(),
                }));
            }
        }
    }
    Ok(Entrada {
        nombre: nombre
            .ok_or_else(|| problema("falta el campo nombre"))?
            .to_string(),
        url: url
            .ok_or_else(|| problema("falta el campo url"))?
            .to_string(),
        timeout_ms,
    })
}

/// Letras, dígitos y guion bajo, hasta 32: lo que puede ser un nombre de campo.
fn parece_campo(clave: &str) -> bool {
    !clave.is_empty()
        && clave.len() <= 32
        && clave.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn error_de(texto: &str) -> String {
        desde_texto(texto).unwrap_err().to_string()
    }

    #[test]
    fn lee_una_lista_y_pone_el_tiempo_por_omision() {
        let servicios = desde_texto(
            "- nombre: a\n  url: https://a.ejemplo.com\n- nombre: b\n  url: http://b.ejemplo.com/salud\n  timeout_ms: 250\n",
        )
        .unwrap();
        assert_eq!(servicios.len(), 2);
        assert_eq!(servicios[0].timeout_ms, TIMEOUT_POR_OMISION_MS);
        assert_eq!(servicios[1].timeout_ms, 250);
        assert_eq!(servicios[1].url, "http://b.ejemplo.com/salud");
    }

    #[test]
    fn un_campo_mal_escrito_es_un_error() {
        let error = error_de("- nombre: a\n  url: https://a.ejemplo.com\n  tiemout: 3\n");
        assert_eq!(error, "servicio 1: campo desconocido `tiemout`");
    }

    #[test]
    fn una_lista_vacia_es_un_error() {
        assert_eq!(desde_texto("[]\n").unwrap_err(), ErrorConfig::SinServicios);
    }

    #[test]
    fn un_nombre_repetido_es_un_error() {
        let error = error_de(
            "- nombre: a\n  url: https://x.ejemplo.com\n- nombre: a\n  url: https://y.ejemplo.com\n",
        );
        assert!(error.contains("«a» está repetido"), "error: {error}");
    }

    #[test]
    fn rechaza_un_esquema_que_no_es_http() {
        let error = error_de("- nombre: a\n  url: ftp://a.ejemplo.com\n");
        assert!(error.contains("http:// o https://"), "error: {error}");
    }

    #[test]
    fn rechaza_una_url_sin_esquema() {
        let error = error_de("- nombre: a\n  url: a.ejemplo.com\n");
        assert!(error.contains("no se puede usar"), "error: {error}");
    }

    #[test]
    fn rechaza_credenciales_sin_repetirlas() {
        let error = error_de("- nombre: a\n  url: https://usuario:clave@a.ejemplo.com/salud\n");
        assert!(error.contains("usuario o contraseña"), "error: {error}");
        assert!(
            !error.contains("clave@"),
            "el error repitió la URL: {error}"
        );
        assert!(
            !error.contains("a.ejemplo.com"),
            "el error repitió la URL: {error}"
        );
    }

    #[test]
    fn un_error_de_tipo_no_repite_el_valor() {
        let error = error_de(
            "- nombre: a\n  url: https://a.ejemplo.com\n  timeout_ms: \"https://h/?token=SECRETO\"\n",
        );
        assert_eq!(
            error,
            "servicio 1: timeout_ms debe ser un número entero de milisegundos"
        );
        assert!(
            !error.contains("SECRETO"),
            "el error repitió el valor: {error}"
        );
    }

    #[test]
    fn un_valor_con_comillas_y_diagonales_escapadas_no_filtra_el_secreto() {
        // En YAML, "a\"b\\c?token=SECRETO" es el texto a"b\c?token=SECRETO. Las
        // comillas y la diagonal escapadas no deben hacer que el mensaje deje
        // salir ni un pedazo del valor.
        let error = error_de(
            "- nombre: a\n  url: https://a.ejemplo.com\n  timeout_ms: \"a\\\"b\\\\c?token=SECRETO\"\n",
        );
        assert!(error.contains("timeout_ms"), "error: {error}");
        for pedazo in ["SECRETO", "token", "c?"] {
            assert!(
                !error.contains(pedazo),
                "el error dejó salir «{pedazo}»: {error}"
            );
        }
    }

    #[test]
    fn rechaza_tiempos_fuera_de_rango() {
        for t in [0, TIMEOUT_MAXIMO_MS + 1] {
            let error = error_de(&format!(
                "- nombre: a\n  url: https://a.ejemplo.com\n  timeout_ms: {t}\n"
            ));
            assert!(error.contains("timeout_ms debe estar"), "error: {error}");
        }
    }

    #[test]
    fn acepta_los_extremos_del_rango() {
        for t in [1, TIMEOUT_MAXIMO_MS] {
            let servicios = desde_texto(&format!(
                "- nombre: a\n  url: https://a.ejemplo.com\n  timeout_ms: {t}\n"
            ))
            .unwrap();
            assert_eq!(servicios[0].timeout_ms, t);
        }
    }

    #[test]
    fn un_archivo_que_no_existe_dice_cual() {
        let error = leer(Path::new("no-existe-este-archivo.yaml"))
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            "no se pudo leer no-existe-este-archivo.yaml: no existe"
        );
    }

    #[test]
    fn un_error_de_sintaxis_solo_dice_donde() {
        // Comillas sin cerrar: libyaml no puede leerlo. El mensaje no lleva nada
        // del texto, solo la línea y la columna.
        let error = error_de("- nombre: a\n  url: \"https://h/?token=SECRETO\n");
        assert!(
            error.starts_with("el archivo no es YAML válido (línea "),
            "error: {error}"
        );
        assert!(!error.contains("SECRETO"), "error: {error}");
    }

    #[test]
    fn un_campo_desconocido_raro_no_se_repite() {
        let error = error_de(
            "- nombre: a\n  url: https://a.ejemplo.com\n  \"https://h/?token=SECRETO\": 1\n",
        );
        assert_eq!(error, "servicio 1: un campo desconocido");
    }

    #[test]
    fn la_forma_equivocada_se_explica() {
        assert_eq!(
            error_de("nombre: a\n"),
            "el archivo debe ser una lista de servicios"
        );
        assert_eq!(
            error_de("- url: https://a.ejemplo.com\n"),
            "servicio 1: falta el campo nombre"
        );
        assert_eq!(error_de("- nombre: a\n"), "servicio 1: falta el campo url");
        assert_eq!(
            error_de("- https://a.ejemplo.com\n"),
            "servicio 1: debe tener los campos nombre y url"
        );
        assert_eq!(desde_texto("").unwrap_err(), ErrorConfig::SinServicios);
    }
}
