//! Carga de configuraciones TOML con ajustes de la línea de comandos (CLAUDE §5: el responsable
//! dirige los experimentos cambiando la config, no el código). Un ajuste `ruta=valor` cambia o añade
//! un valor del archivo antes de validarlo, sin editar el archivo. Lo usa, por ejemplo, la batería de
//! escenarios de referencia (`herramientas/escenarios.py`).

use serde::de::DeserializeOwned;
use toml::{Table, Value};

use crate::ConfigError;

/// Lee la config de `path`, le aplica los `ajustes` en orden y la convierte en `T`.
///
/// Cada ajuste es `ruta=valor`:
/// - la ruta va por puntos, y un número elige un elemento de una lista: `run.total_ticks`,
///   `especies.1.ajustes.p_conc`;
/// - las tablas que falten se crean: `habitat.sequia.factor=0.3`;
/// - el valor es un literal TOML (`40.0`, `true`, `"texto"`, `[1, 2]`); si no lo es, se toma como
///   texto (`run.name=prueba`).
///
/// Una clave mal escrita no pasa: las configs rechazan los campos desconocidos
/// (`deny_unknown_fields`).
pub fn cargar<T: DeserializeOwned>(path: &str, ajustes: &[String]) -> Result<T, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_string(),
        source,
    })?;
    let parse = |source| ConfigError::Parse {
        path: path.to_string(),
        source,
    };
    let mut raiz = Value::Table(text.parse::<Table>().map_err(parse)?);
    for ajuste in ajustes {
        aplicar(&mut raiz, ajuste)?;
    }
    raiz.try_into().map_err(parse)
}

/// Aplica un ajuste `ruta=valor` al árbol de la config.
fn aplicar(raiz: &mut Value, ajuste: &str) -> Result<(), ConfigError> {
    let invalido = |motivo: String| ConfigError::Invalid(format!("ajuste '{ajuste}': {motivo}"));
    let (ruta, texto) = ajuste
        .split_once('=')
        .ok_or_else(|| invalido("falta '=' (se espera ruta=valor)".into()))?;
    let valor = texto
        .trim()
        .parse::<Value>()
        .unwrap_or_else(|_| Value::String(texto.trim().to_string()));
    let partes: Vec<&str> = ruta.trim().split('.').collect();
    if partes.iter().any(|p| p.is_empty()) {
        return Err(invalido("la ruta tiene un tramo vacío".into()));
    }
    let Some((ultima, camino)) = partes.split_last() else {
        return Err(invalido("ruta vacía".into()));
    };
    let mut nodo = raiz;
    for parte in camino {
        nodo = match nodo {
            Value::Table(t) => t
                .entry(parte.to_string())
                .or_insert_with(|| Value::Table(Table::new())),
            Value::Array(lista) => {
                let n = lista.len();
                let i: usize = parte
                    .parse()
                    .map_err(|_| invalido(format!("'{parte}' no es un índice de lista")))?;
                lista.get_mut(i).ok_or_else(|| {
                    invalido(format!("índice {i} fuera de la lista ({n} elementos)"))
                })?
            }
            _ => return Err(invalido(format!("'{parte}' no es una tabla ni una lista"))),
        };
    }
    match nodo {
        Value::Table(t) => {
            t.insert(ultima.to_string(), valor);
        }
        Value::Array(lista) => {
            let n = lista.len();
            let i: usize = ultima
                .parse()
                .map_err(|_| invalido(format!("'{ultima}' no es un índice de lista")))?;
            *lista.get_mut(i).ok_or_else(|| {
                invalido(format!("índice {i} fuera de la lista ({n} elementos)"))
            })? = valor;
        }
        _ => return Err(invalido(format!("'{ultima}' cuelga de un valor simple"))),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Cfg {
        nombre: String,
        x: f64,
        zonas: Vec<Zona>,
        #[serde(default)]
        sequia: Option<Sequia>,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Zona {
        area: f64,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct Sequia {
        factor: f64,
    }

    fn con(ajustes: &[&str]) -> Result<Cfg, ConfigError> {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("abm-core-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("c{n}.toml"));
        std::fs::write(
            &path,
            "nombre = \"base\"\nx = 1.5\n[[zonas]]\narea = 10.0\n[[zonas]]\narea = 20.0\n",
        )
        .unwrap();
        let ajustes: Vec<String> = ajustes.iter().map(|s| s.to_string()).collect();
        cargar(path.to_str().unwrap(), &ajustes)
    }

    #[test]
    fn los_ajustes_cambian_y_crean_valores() {
        let c = con(&[
            "x=40",
            "zonas.1.area=2.5",
            "sequia.factor=0.3",
            "nombre=prueba",
        ])
        .unwrap();
        assert_eq!(c.x, 40.0);
        assert_eq!(c.zonas[1].area, 2.5);
        assert_eq!(c.sequia, Some(Sequia { factor: 0.3 }));
        assert_eq!(c.nombre, "prueba");
        assert_eq!(con(&[]).unwrap().x, 1.5);
    }

    #[test]
    fn los_ajustes_mal_escritos_no_pasan() {
        assert!(con(&["xx=1.0"]).is_err()); // campo desconocido
        assert!(con(&["x"]).is_err()); // sin '='
        assert!(con(&["zonas.5.area=1.0"]).is_err()); // fuera de la lista
        assert!(con(&["x.y=1.0"]).is_err()); // dentro de un valor simple
        assert!(con(&["x=texto"]).is_err()); // tipo equivocado
    }
}
