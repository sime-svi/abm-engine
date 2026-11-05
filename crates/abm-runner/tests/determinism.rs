//! Tests de determinismo y sanidad del modelo (CLAUDE §6, §10).
//!
//! Usan `abm_runner::run_simulation`, la misma función que el binario, de modo que se verifica el camino
//! real. En tests sí se permite `expect`/`unwrap` (CLAUDE §5).

use abm_runner::{Config, run_simulation};

/// Config por defecto del workspace (los tests corren desde la carpeta del paquete).
const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/default.toml");

/// Ejecuta una simulación y devuelve su JSON (string = bytes comparables).
fn run_to_json(cfg: &Config) -> String {
    let doc = run_simulation(cfg).expect("la simulación no debe fallar");
    serde_json::to_string_pretty(&doc).expect("serializar el documento a JSON")
}

/// La misma (semilla, versión, config) debe producir exactamente la misma salida.
#[test]
fn misma_semilla_misma_salida() {
    let cfg = Config::load(CONFIG).expect("cargar la config por defecto");
    let a = run_to_json(&cfg);
    let b = run_to_json(&cfg);
    assert_eq!(
        a, b,
        "misma (semilla, versión, config) debe dar salida idéntica byte a byte"
    );
}

/// Guardia: semillas distintas deben divergir (si no, la semilla no se estaría usando).
#[test]
fn semilla_distinta_salida_distinta() {
    let mut c1 = Config::load(CONFIG).expect("cargar la config por defecto");
    c1.run.seed = 1;
    let mut c2 = c1.clone();
    c2.run.seed = 2;
    assert_ne!(
        run_to_json(&c1),
        run_to_json(&c2),
        "semillas distintas deberían producir trayectorias distintas"
    );
}

/// Guardia: la simulación no es trivialmente vacía (no "pasa" por estar todo a cero).
#[test]
fn no_es_trivialmente_vacio() {
    let cfg = Config::load(CONFIG).expect("cargar la config por defecto");
    let doc = run_simulation(&cfg).expect("la simulación no debe fallar");

    assert_eq!(
        doc.ticks.len(),
        cfg.run.total_ticks as usize,
        "con metrics_every = 1 debe haber una fila de métricas por tick"
    );
    assert!(
        doc.ticks.iter().any(|t| t.population > 0),
        "debe existir población viva en algún tick"
    );
    assert!(
        doc.ticks.iter().all(|t| t.energy_total.is_finite()
            && t.energy_mean.is_finite()
            && t.age_mean.is_finite()
            && t.resource_total.is_finite()),
        "ninguna métrica debe ser NaN ni infinita"
    );
    assert!(
        doc.ticks.iter().any(|t| t.births > 0 || t.deaths > 0),
        "el modelo debe evolucionar (algún nacimiento o muerte)"
    );
}
