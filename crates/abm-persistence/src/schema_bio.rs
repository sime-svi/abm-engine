//! Formato de salida de la biología de los agentes (`output/<nombre>_agentes.json`). Diseño de
//! biología §8; incremento 1 §8.
//!
//! Los campos de la revisión 1 son opcionales: solo se escriben si su interruptor está encendido.
//! Con todos apagados, el documento es el del incremento 1, byte a byte (formato v1).

use serde::{Deserialize, Serialize};

/// Versión del formato.
pub const BIO_SCHEMA_VERSION: u32 = 1;

/// Documento completo de una corrida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BioDoc {
    pub schema_version: u32,
    pub run: BioRunMeta,
    /// Un registro cada `metrics_every` ticks (los recuentos, acumulados desde el anterior).
    pub ticks: Vec<BioTick>,
    /// Un resumen por año.
    pub anios: Vec<BioAnio>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BioRunMeta {
    pub name: String,
    pub seed: u64,
    pub engine_version: String,
    pub ruleset_version: String,
    pub total_ticks: u64,
    pub dias_anio: u32,
    pub metrics_every: u64,
    pub celdas: u64,
    pub area_total_km2: f64,
    pub especies: Vec<EspecieMeta>,
}

/// Parámetros derivados de una especie (para auditar la calibración).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EspecieMeta {
    pub nombre: String,
    pub termico: String,
    pub masa_kg: f64,
    pub m0_kg: f64,
    pub pal: f64,
    pub k_ap: f64,
    pub eps: f64,
    pub f_com: f64,
    pub f_grasa_max: f64,
    pub camada: f64,
    pub p_conc: f64,
    pub rho_sexo: f64,
    pub p_huerf: f64,
    pub a_max_dias: u32,
    pub a_crec90_dias: u32,
    pub a_rep_dias: u32,
    pub a_fin_rep_dias: u32,
    pub d_gest_dias: u32,
    pub d_destete_dias: u32,
    pub delta_a_dias: u32,
    pub c_w: f64,
    /// Siler `[α1, β1, α2, α3, β3]` (1/año).
    pub siler: [f64; 5],
    pub basal_adulto_kcal: f64,
    pub gasto_adulto_kcal: f64,
    pub e_max_adulto_kcal: f64,
    pub ayuno_dias: f64,
    /// Población máxima sostenible de todo el hábitat (§1).
    pub n_max: f64,
    pub poblacion_inicial: u64,
    /// Con dimorfismo (revisión 1 §3.4): masa adulta y días de ayuno de cada sexo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub masa_kg_hembra: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub masa_kg_macho: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ayuno_dias_hembra: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ayuno_dias_macho: Option<f64>,
    /// Con respuesta funcional (revisión 1 §2): semisaturación relativa a `K`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub h_medio: Option<f64>,
    /// Con esterilidad individual (revisión 1 §4): media y desviación típica de la edad de
    /// esterilidad (años).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mu_est_anios: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sigma_est_anios: Option<f64>,
    /// Con las métricas de demografía: límites (días) de los grupos de edad de la mortalidad
    /// (crías < destete ≤ jóvenes < madurez ≤ adultos < vejez ≤ viejos).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grupos_edad_dias: Option<[u32; 3]>,
}

/// Registro de un tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BioTick {
    pub tick: u64,
    pub especies: Vec<EspecieTick>,
    /// Balance de energía del intervalo (kcal).
    pub balance: Balance,
    /// Ids repetidos entre los vivos (debe ser 0).
    pub colisiones_id: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EspecieTick {
    pub n: u64,
    pub hembras: u64,
    pub machos: u64,
    pub dependientes: u64,
    pub gestantes: u64,
    pub lactantes: u64,
    pub nacimientos: u64,
    pub muertes_inanicion: u64,
    pub muertes_edad_max: u64,
    pub muertes_intrinseca: u64,
    pub muertes_orfandad: u64,
    pub concepciones: u64,
    pub abortos: u64,
    pub destetes: u64,
    /// Condición media `E/E_max` y fracción en déficit (del tick del registro).
    pub condicion_media: f64,
    pub frac_deficit: f64,
    pub reservas_kcal: f64,
    /// Forraje de la especie: stock, stock máximo, consumo y rebrote del intervalo (kg de materia seca).
    pub forraje_kg: f64,
    pub forraje_max_kg: f64,
    pub consumo_kg: f64,
    pub rebrote_kg: f64,
    /// Con destete forzoso (revisión 1 §3.2): madres que destetan a la fuerza, crías que pasan a
    /// comer solas y crías que mueren en el intervalo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destetes_forzosos: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crias_destetadas_forzoso: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muertes_destete: Option<u64>,
    /// Con las métricas de demografía: condición media `E/E_max` de las lactantes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condicion_lactantes: Option<f64>,
}

/// Balance de energía (diseño §8). `residuo = Δreservas − (asimilada + leche_recibida −
/// gasto_propio − leche_pagada − perdida_almacen − excedente − reservas_muertos)`, que debe ser 0 a
/// redondeo. `perdida_leche = leche_pagada − leche_recibida`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Balance {
    pub asimilada: f64,
    pub leche_recibida: f64,
    pub gasto_propio: f64,
    pub leche_pagada: f64,
    pub perdida_almacen: f64,
    pub perdida_leche: f64,
    pub excedente: f64,
    pub reservas_muertos: f64,
    pub delta_reservas: f64,
    pub residuo: f64,
}

/// Resumen anual.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BioAnio {
    pub anio: u64,
    pub especies: Vec<EspecieAnio>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EspecieAnio {
    pub poblacion_media: f64,
    pub densidad_km2: f64,
    pub n_max_km2: f64,
    pub nacimientos: u64,
    pub muertes_inanicion: u64,
    pub muertes_edad_max: u64,
    pub muertes_intrinseca: u64,
    pub muertes_orfandad: u64,
    /// Tasas brutas por 1 000 habitantes y año.
    pub tasa_natalidad_mil: f64,
    pub tasa_mortalidad_mil: f64,
    /// Esperanza de vida al nacer de la tabla de vida del periodo (años).
    pub e0_anios: Option<f64>,
    /// Tasa global de fecundidad (crías por hembra).
    pub tgf: Option<f64>,
    pub intervalo_partos_dias: Option<f64>,
    /// Consumo / rebrote del forraje.
    pub presion_herbivoria: Option<f64>,
    /// Pirámide al final del año, por clase de edad (`Δa`) y sexo.
    pub piramide_hembras: Vec<u64>,
    pub piramide_machos: Vec<u64>,
    /// Con destete forzoso: crías muertas en destetes forzosos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub muertes_destete: Option<u64>,
    /// Con las métricas de demografía (revisión 1): fecundidad por grupos de edad de la madre
    /// (crías por hembra y año; grupos de `metricas.ancho_fecundidad_anios`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fecundidad_grupos: Option<Vec<f64>>,
    /// Hembras que terminan su vida reproductiva en el año habiendo parido durante la corrida, y
    /// edad media de su último parto (años).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_ultimo_parto: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edad_ultimo_parto_anios: Option<f64>,
    /// Mortalidad anual (1/año) por grupo de edad (crías, jóvenes, adultos, viejos) y sexo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mortalidad_hembras: Option<[f64; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mortalidad_machos: Option<[f64; 4]>,
}
