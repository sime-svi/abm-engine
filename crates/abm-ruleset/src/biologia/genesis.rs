//! Génesis de la biología (§6.0): crea el mundo con el catálogo, el hábitat y la población inicial.
//! Todo sorteo usa el azar por contador con claves fijas, así que la génesis es una función pura de
//! la configuración.

use bevy_ecs::prelude::*;

use abm_core::ConfigError;
use abm_core::time::Tick;

use super::azar::{Proposito, hash, uniforme};
use super::componentes::{Celda, Diario, Edad, EspecieRef, IdBio, Masa, Repro, Reserva, Sexo};
use super::config::{BioConfig, EdadesIniciales};
use super::habitat::Habitat;
use super::recursos::{Buffers, Catalogo, Libro};

type Nuevo = (
    IdBio,
    EspecieRef,
    Sexo,
    Edad,
    Masa,
    Reserva,
    Repro,
    Celda,
    Diario,
);

/// Construye el mundo inicial.
pub fn build(cfg: &BioConfig) -> Result<World, ConfigError> {
    build_con_orden(cfg, None)
}

/// Igual que `build`, pero con la población inicial creada en un orden barajado (prueba de que el
/// resultado no depende del orden de proceso, §10). El contenido del mundo es el mismo.
pub fn build_con_orden(cfg: &BioConfig, barajado: Option<u64>) -> Result<World, ConfigError> {
    cfg.validate()?;
    let cat = Catalogo::nuevo(cfg)?;
    let hab = Habitat::nuevo(cfg, &cat.especies);
    let semilla = cfg.run.seed;
    let po = &cfg.poblacion;

    // Distribución estable aproximada de edades: ∝ supervivencia l(a) (D7), acumulada.
    let acumuladas: Vec<Vec<f64>> = cat
        .especies
        .iter()
        .map(|e| {
            let l = e.supervivencia();
            let mut acc = Vec::with_capacity(e.a_max as usize);
            let mut s = 0.0;
            for x in l.iter().take(e.a_max as usize) {
                s += x;
                acc.push(s);
            }
            acc.iter_mut().for_each(|x| *x /= s);
            acc
        })
        .collect();

    let mut nuevos: Vec<Nuevo> = Vec::new();
    for c in 0..hab.n_celdas {
        for (k, e) in cat.especies.iter().enumerate() {
            let n = match po.n_inicial_por_celda {
                Some(n) => n as u64,
                None => {
                    let n_max = e.n_max(
                        hab.area_m2[c],
                        hab.ppn[c],
                        cfg.forraje.rho_veg,
                        hab.f_ecto[c],
                    );
                    (po.fraccion_inicial * n_max + 0.5).floor() as u64
                }
            };
            let clave = ((c as u64) << 16) | k as u64;
            for i in 0..n {
                let id = hash(semilla, clave, 0, Proposito::IdInicial, i);
                let macho = uniforme(semilla, id, 0, Proposito::SexoInicial, 0) < e.rho_sexo;
                let a = match po.edades_iniciales {
                    EdadesIniciales::Cero => 0,
                    EdadesIniciales::Estable => {
                        let u = uniforme(semilla, id, 0, Proposito::EdadInicial, 0);
                        acumuladas[k].partition_point(|&x| x <= u) as u32
                    }
                };
                let a = a.min(e.a_max.saturating_sub(1));
                let sexo = if macho { Sexo::Macho } else { Sexo::Hembra };
                // Masa y reserva con la masa adulta y la grasa de su sexo (revisión 1 §3.4).
                let m = e.masa_por_edad_s(sexo.i(), a);
                // Fin de la vida reproductiva: su edad de esterilidad (revisión 1 §4).
                let a_fin = if macho {
                    e.a_fin_rep
                } else {
                    e.a_est(semilla, id)
                };
                nuevos.push((
                    IdBio(id),
                    EspecieRef(k as u16),
                    sexo,
                    Edad(a),
                    Masa(m),
                    Reserva(po.reserva_inicial * e.e_max_s(sexo.i(), m)),
                    Repro::por_edad(macho, a, e.a_rep, a_fin),
                    Celda(c as u32),
                    Diario::default(),
                ));
            }
        }
    }
    if let Some(s) = barajado {
        nuevos.sort_by_key(|x| hash(s, x.0.0, 0, Proposito::Barajado, 0));
    }

    let mut world = World::new();
    world.insert_resource(Tick(0));
    world.spawn_batch(nuevos);
    world.insert_resource(cat);
    world.insert_resource(hab);
    world.insert_resource(Libro::default());
    world.insert_resource(Buffers::default());
    Ok(world)
}
