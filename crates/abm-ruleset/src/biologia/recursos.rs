//! Recursos del modelo de biología: el catálogo de especies, el libro del tick (energía y eventos)
//! y los búferes reutilizados entre ticks (sin asignaciones en el bucle caliente, CLAUDE §5).

use bevy_ecs::prelude::*;

use super::azar::SumaExacta;
use super::componentes::{Celda, Sexo};
use super::config::{BioConfig, EnergeticaCfg, ForrajeCfg, SequiaCfg};
use super::especie::Especie;
use abm_core::ConfigError;

/// Especies derivadas y parámetros comunes.
#[derive(Resource, Debug, Clone)]
pub struct Catalogo {
    pub especies: Vec<Especie>,
    pub en: EnergeticaCfg,
    pub forraje: ForrajeCfg,
    pub sequia: Option<SequiaCfg>,
    pub semilla: u64,
    pub dias_anio: u32,
}

impl Catalogo {
    pub fn nuevo(cfg: &BioConfig) -> Result<Catalogo, ConfigError> {
        let especies = cfg
            .especies
            .iter()
            .map(|e| Especie::derivar(cfg, e))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Catalogo {
            especies,
            en: cfg.energetica.clone(),
            forraje: cfg.forraje.clone(),
            sequia: cfg.habitat.sequia.clone(),
            semilla: cfg.run.seed,
            dias_anio: cfg.tiempo.dias_anio,
        })
    }

    /// Factor estacional del rebrote `s(t)`: 1 con clima constante, salvo la sequía de prueba (D15).
    pub fn factor_estacional(&self, t: u64) -> f64 {
        match &self.sequia {
            Some(s) if t >= s.inicio_dia && t < s.inicio_dia + s.duracion_dias => s.factor,
            _ => 1.0,
        }
    }
}

/// Causa de muerte (§6.8), en orden de evaluación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Causa {
    Inanicion = 0,
    EdadMax = 1,
    Intrinseca = 2,
    Orfandad = 3,
}

/// Un muerto del tick (para las métricas: causa, edad y reserva).
#[derive(Debug, Clone, Copy)]
pub struct MuertoInfo {
    pub esp: u16,
    pub sexo: Sexo,
    pub edad: u32,
    pub causa: Causa,
    pub reserva: f64,
}

/// Un parto del tick.
#[derive(Debug, Clone, Copy)]
pub struct PartoInfo {
    pub esp: u16,
    pub edad_madre: u32,
    pub crias: u32,
    /// Días desde el parto anterior de la misma madre.
    pub intervalo: Option<u64>,
}

/// Recuentos del tick de una especie.
#[derive(Debug, Clone, Copy, Default)]
pub struct CuentasEspecie {
    pub nacimientos: u64,
    pub muertes: [u64; 4],
    pub concepciones: u64,
    pub abortos: u64,
    pub destetes: u64,
    pub en_deficit: u64,
    /// Forraje bruto consumido (kcal) y rebrote neto del stock (g).
    pub consumo_kcal: SumaExacta,
    pub rebrote_g: SumaExacta,
}

/// Libro del tick: flujos de energía (sumas exactas: no dependen del orden, D13) y eventos.
#[derive(Resource, Debug, Clone, Default)]
pub struct Libro {
    pub esp: Vec<CuentasEspecie>,
    pub reservas_inicio: SumaExacta,
    pub asimilada: SumaExacta,
    pub leche_recibida: SumaExacta,
    pub gasto_propio: SumaExacta,
    pub leche_pagada: SumaExacta,
    pub perdida_almacen: SumaExacta,
    pub excedente: SumaExacta,
    pub reservas_muertos: SumaExacta,
    pub muertos: Vec<MuertoInfo>,
    pub partos: Vec<PartoInfo>,
}

impl Libro {
    /// Pone el libro a cero para un tick nuevo (conserva la capacidad de los Vec).
    pub fn reiniciar(&mut self, n_esp: usize) {
        self.esp.clear();
        self.esp.resize(n_esp, CuentasEspecie::default());
        self.reservas_inicio = SumaExacta::default();
        self.asimilada = SumaExacta::default();
        self.leche_recibida = SumaExacta::default();
        self.gasto_propio = SumaExacta::default();
        self.leche_pagada = SumaExacta::default();
        self.perdida_almacen = SumaExacta::default();
        self.excedente = SumaExacta::default();
        self.reservas_muertos = SumaExacta::default();
        self.muertos.clear();
        self.partos.clear();
    }
}

/// Cría que nace este tick (se crea al final del paso de reproducción).
#[derive(Debug, Clone, Copy)]
pub struct CriaPendiente {
    pub id: u64,
    pub madre: u64,
    pub esp: u16,
    pub celda: Celda,
    pub macho: bool,
    pub reserva: f64,
}

/// Búferes reutilizados entre ticks.
#[derive(Resource, Debug, Default)]
pub struct Buffers {
    /// Leche que necesita cada cría: (id de la madre, φ·G_c), ordenado por madre.
    pub leche: Vec<(u64, f64)>,
    /// Demanda total, cociente del reparto justo y consumo por (celda, especie).
    pub demanda: Vec<SumaExacta>,
    pub reparto: Vec<f64>,
    pub consumo: Vec<SumaExacta>,
    /// Machos maduros por (celda, especie) y madre de cada cría viva (ordenado).
    pub machos: Vec<u32>,
    pub crias: Vec<u64>,
    pub nacimientos: Vec<CriaPendiente>,
    /// Muertos del tick (entidad, id) y huérfanos que sobreviven.
    pub muertos: Vec<(Entity, u64)>,
    pub ids_muertos: Vec<u64>,
    pub madres_muertas: Vec<u64>,
    pub huerfanos: Vec<Entity>,
}
