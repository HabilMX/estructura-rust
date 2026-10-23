//! Pruebas de integración: solo ven lo que la biblioteca declara `pub`, igual que
//! lo vería otro programa que la usara. Cargo compila este archivo como un
//! crate aparte que depende de `revisor`.

mod comun;

use std::time::{Duration, Instant};

use revisor::reporte;
use revisor::revisar::{cliente, consultar, revisar_todos};
use revisor::servicio::{Estado, Servicio};

fn servicio(direccion: &str, ruta: &str, timeout_ms: u64) -> Servicio {
    Servicio {
        nombre: ruta.trim_start_matches('/').replace('/', "-"),
        url: format!("http://{direccion}{ruta}"),
        timeout_ms,
    }
}

/// Una consulta que no puede colgar la prueba: si pasa de `tope`, falla.
async fn consultar_con_tope(
    s: &Servicio,
    tope: Duration,
) -> (revisor::servicio::Resultado, Duration) {
    let inicio = Instant::now();
    let resultado = tokio::time::timeout(tope, consultar(&cliente().unwrap(), s))
        .await
        .unwrap_or_else(|_| panic!("la consulta a {} pasó de {tope:?}", s.nombre));
    (resultado, inicio.elapsed())
}

#[tokio::test]
async fn un_servicio_sano_sale_arriba() {
    let d = comun::servidor();
    let (r, _) = consultar_con_tope(&servicio(&d, "/ok", 2000), Duration::from_secs(5)).await;
    assert_eq!(r.estado, Estado::Arriba, "{r:?}");
    assert_eq!(r.detalle, "200");
    assert!(r.tiempo_ms.is_some());
}

#[tokio::test]
async fn un_500_esta_caido_con_su_codigo() {
    let d = comun::servidor();
    let (r, _) = consultar_con_tope(&servicio(&d, "/error", 2000), Duration::from_secs(5)).await;
    assert_eq!(r.estado, Estado::Caido, "{r:?}");
    assert_eq!(r.detalle, "500");
}

#[tokio::test]
async fn un_servicio_lento_pero_sano_sale_lento() {
    let d = comun::servidor();
    let (r, _) =
        consultar_con_tope(&servicio(&d, "/lento/1100", 3000), Duration::from_secs(6)).await;
    assert_eq!(r.estado, Estado::Lento, "{r:?}");
}

#[tokio::test]
async fn el_que_nunca_contesta_respeta_su_tiempo_limite() {
    let d = comun::servidor();
    let (r, tardo) = consultar_con_tope(&servicio(&d, "/mudo", 400), Duration::from_secs(10)).await;
    assert_eq!(r.estado, Estado::Caido, "{r:?}");
    assert_eq!(r.detalle, "se acabó el tiempo de espera");
    assert!(r.tiempo_ms.is_none(), "sin respuesta, sin tiempo");
    assert!(
        tardo >= Duration::from_millis(400),
        "cortó antes de tiempo: {tardo:?}"
    );
    assert!(
        tardo < Duration::from_millis(2400),
        "no respetó su tiempo límite: {tardo:?}"
    );
}

#[tokio::test]
async fn el_cuerpo_a_goteo_respeta_su_tiempo_limite() {
    let d = comun::servidor();
    let (r, tardo) =
        consultar_con_tope(&servicio(&d, "/goteo", 500), Duration::from_secs(10)).await;
    assert_eq!(r.estado, Estado::Caido, "{r:?}");
    assert_eq!(r.detalle, "se acabó el tiempo de espera");
    assert!(
        tardo < Duration::from_millis(2500),
        "siguió leyendo el goteo: {tardo:?}"
    );
}

#[tokio::test]
async fn el_drenado_tiene_tope() {
    // El cuerpo no se acaba nunca. Con tope, la consulta termina mucho antes de
    // su tiempo límite; sin tope, leería hasta que se acabara el tiempo.
    let d = comun::servidor();
    let (r, tardo) =
        consultar_con_tope(&servicio(&d, "/enorme", 5000), Duration::from_secs(10)).await;
    assert_eq!(r.estado, Estado::Arriba, "{r:?}");
    assert!(
        tardo < Duration::from_millis(2500),
        "no paró en el tope: {tardo:?}"
    );
}

#[tokio::test]
async fn un_puerto_cerrado_esta_caido_sin_repetir_la_url() {
    // Se toma un puerto libre y se suelta: nadie escucha ahí.
    let libre = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap();
    let s = Servicio {
        nombre: "nadie".to_string(),
        url: format!("http://{libre}/salud?token=SECRETO"),
        timeout_ms: 2000,
    };
    let (r, _) = consultar_con_tope(&s, Duration::from_secs(5)).await;
    assert_eq!(r.estado, Estado::Caido, "{r:?}");
    assert_eq!(r.detalle, "no se pudo conectar");
    assert!(!r.detalle.contains("SECRETO"));
}

#[tokio::test]
async fn el_reporte_de_una_falla_no_lleva_el_token() {
    let d = comun::servidor();
    let s = Servicio {
        nombre: "pagos".to_string(),
        url: format!("http://{d}/error?token=SECRETO"),
        timeout_ms: 2000,
    };
    let resultados = revisar_todos(&cliente().unwrap(), &[s], 1).await;
    for texto in [reporte::tabla(&resultados), reporte::json(&resultados)] {
        assert!(
            !texto.contains("SECRETO"),
            "el reporte filtró el token: {texto}"
        );
    }
}

#[tokio::test]
async fn revisar_todos_respeta_el_orden_de_los_servicios() {
    let d = comun::servidor();
    let servicios = vec![
        servicio(&d, "/lento/300", 2000),
        servicio(&d, "/error", 2000),
        servicio(&d, "/ok", 2000),
    ];
    let resultados = revisar_todos(&cliente().unwrap(), &servicios, 3).await;
    let nombres: Vec<&str> = resultados
        .iter()
        .map(|r| r.servicio.nombre.as_str())
        .collect();
    assert_eq!(nombres, ["lento-300", "error", "ok"]);
}

#[tokio::test]
async fn los_lentos_se_esperan_en_paralelo() {
    // Ocho servicios de 300 ms: a la vez tardan poco más de 300 ms; uno por
    // uno, 2.4 s o más.
    let d = comun::servidor();
    let servicios: Vec<Servicio> = (0..8).map(|_| servicio(&d, "/lento/300", 3000)).collect();
    let inicio = Instant::now();
    let resultados = revisar_todos(&cliente().unwrap(), &servicios, 8).await;
    let tardo = inicio.elapsed();
    assert!(
        resultados.iter().all(|r| r.estado == Estado::Arriba),
        "{resultados:?}"
    );
    assert!(
        tardo < Duration::from_millis(1500),
        "no fueron a la vez: {tardo:?}"
    );
}

#[tokio::test]
async fn el_tope_de_paralelo_se_respeta() {
    // Cuatro servicios de 300 ms, de dos en dos: al menos 600 ms.
    let d = comun::servidor();
    let servicios: Vec<Servicio> = (0..4).map(|_| servicio(&d, "/lento/300", 3000)).collect();
    let inicio = Instant::now();
    revisar_todos(&cliente().unwrap(), &servicios, 2).await;
    let tardo = inicio.elapsed();
    assert!(
        tardo >= Duration::from_millis(600),
        "pasaron más de dos a la vez: {tardo:?}"
    );
}

#[tokio::test]
async fn paralelo_cero_cuenta_como_uno() {
    let d = comun::servidor();
    let servicios = vec![servicio(&d, "/ok", 2000), servicio(&d, "/ok", 2000)];
    let resultados = tokio::time::timeout(
        Duration::from_secs(5),
        revisar_todos(&cliente().unwrap(), &servicios, 0),
    )
    .await
    .expect("con paralelo 0 la revisión se quedó esperando turno");
    assert!(
        resultados.iter().all(|r| r.estado == Estado::Arriba),
        "{resultados:?}"
    );
}

#[tokio::test]
async fn un_paralelo_enorme_no_entra_en_panico() {
    let d = comun::servidor();
    let servicios = vec![servicio(&d, "/ok", 2000)];
    let resultados = revisar_todos(&cliente().unwrap(), &servicios, usize::MAX).await;
    assert_eq!(resultados.len(), 1);
    assert_eq!(resultados[0].estado, Estado::Arriba);
}
