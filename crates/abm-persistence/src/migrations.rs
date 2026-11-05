//! Aplicador de migraciones SQL numeradas (de `infra/migrations/`), embebidas en el binario. No
//! depende de herramientas externas (no hay `psql`). Registra lo aplicado en `schema_migrations`
//! (DATABASE_SCHEMA.md §5). Cada migración se aplica una sola vez, en su propia transacción.

use postgres::Client;

use crate::error::PersistenceError;

/// Migraciones en orden: `(versión, nombre, SQL)`. El SQL se embebe en tiempo de compilación.
const MIGRATIONS: &[(i64, &str, &str)] = &[(
    1,
    "initial",
    include_str!("../../../infra/migrations/V1__initial.sql"),
)];

/// Crea la tabla de control y aplica las migraciones pendientes.
pub fn run(client: &mut Client) -> Result<(), PersistenceError> {
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    BIGINT PRIMARY KEY,
            name       TEXT NOT NULL,
            applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )",
    )?;

    for &(version, name, sql) in MIGRATIONS {
        let already: i64 = client
            .query_one(
                "SELECT count(*) FROM schema_migrations WHERE version = $1",
                &[&version],
            )?
            .get(0);
        if already == 0 {
            let mut tx = client.transaction()?;
            tx.batch_execute(sql)?;
            tx.execute(
                "INSERT INTO schema_migrations (version, name) VALUES ($1, $2)",
                &[&version, &name],
            )?;
            tx.commit()?;
        }
    }
    Ok(())
}
