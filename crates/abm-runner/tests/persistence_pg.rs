//! Integración con PostgreSQL. **Ignorado por defecto** (necesita una BD accesible), así los tests
//! normales siguen siendo herméticos. Ejecutar explícitamente con la BD levantada:
//!
//! ```bash
//! set -a; . infra/.env; set +a          # exporta POSTGRES_USER/PASSWORD/DB y PGPORT
//! cargo test -p abm-runner --test persistence_pg -- --ignored --nocapture
//! ```

use abm_runner::{Config, run_simulation};

/// Config por defecto del workspace (los tests corren desde la carpeta del paquete).
const CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../config/default.toml");

#[test]
#[ignore = "necesita una BD Postgres accesible (DATABASE_URL o variables POSTGRES_*)"]
fn escribe_run_y_metricas() {
    let cfg = Config::load(CONFIG).expect("cargar la config");
    let doc = run_simulation(&cfg).expect("ejecutar la simulación");

    let record = abm_runner::run_record(&cfg).expect("registro de la corrida");
    let run_id = abm_persistence::pg_sink::write_run(&record, &doc).expect("escribir en Postgres");

    // Verificación independiente: contar las filas de esta ejecución.
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        let u = std::env::var("POSTGRES_USER").expect("POSTGRES_USER o DATABASE_URL");
        let p = std::env::var("POSTGRES_PASSWORD").unwrap_or_default();
        let d = std::env::var("POSTGRES_DB").unwrap_or_else(|_| u.clone());
        let host = std::env::var("PGHOST").unwrap_or_else(|_| "localhost".into());
        let port = std::env::var("PGPORT").unwrap_or_else(|_| "25432".into());
        format!("host={host} port={port} user={u} password={p} dbname={d}")
    });
    let mut client = postgres::Client::connect(&url, postgres::NoTls).expect("conectar");
    let rid = run_id.to_string();

    let runs: i64 = client
        .query_one(
            "SELECT count(*) FROM simulation_runs WHERE run_id::text = $1",
            &[&rid],
        )
        .unwrap()
        .get(0);
    assert_eq!(
        runs, 1,
        "debe existir la fila de la ejecución en simulation_runs"
    );

    let metrics: i64 = client
        .query_one(
            "SELECT count(*) FROM metrics WHERE run_id::text = $1",
            &[&rid],
        )
        .unwrap()
        .get(0);
    assert_eq!(
        metrics as usize,
        doc.ticks.len() * 7,
        "deben persistirse 7 métricas por tick"
    );
}
