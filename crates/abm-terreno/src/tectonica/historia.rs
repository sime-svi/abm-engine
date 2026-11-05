//! Historia geológica del planeta (§7 y §8 del diseño): eventos discretos, métricas de cada Myr y
//! cuentas de conservación de la corteza continental. Es un producto de datos en sí mismo.

use serde::{Deserialize, Serialize};

/// Un evento tectónico de la deriva.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventoTectonico {
    /// Tiempo (Myr; negativo = antes del final de la deriva).
    pub myr: f64,
    /// `rift`, `inicio_subduccion`, `inicio_colision`, `sutura`, `absorcion`, `salto_dorsal`,
    /// `acrecion_arco`, `supercontinente_formado` o `supercontinente_roto`.
    pub tipo: String,
    /// Placas implicadas (madre e hijas, las dos del borde, absorbida y receptora…).
    pub placas: Vec<u64>,
    /// Longitud de la línea o del borde (km; 0 si no aplica).
    pub longitud_km: f64,
    /// Área implicada (km²; continental partida en un rift, acrecionada en un arco…).
    pub area_km2: f64,
    /// Punto representativo (grados).
    pub lat_deg: f64,
    pub lon_deg: f64,
}

/// Métricas agregadas de un paso de la deriva (§8, «cada Myr»).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricaMyr {
    pub myr: f64,
    /// `true` en el tramo de rodaje, que no cuenta para la validación (decisión del responsable).
    pub rodaje: bool,
    pub n_placas: u32,
    /// Velocidad media cuadrática de la superficie (cm/año).
    pub v_rms_cm_anio: f64,
    pub dorsales_km: f64,
    pub fosas_km: f64,
    pub colisiones_km: f64,
    pub edad_media_oceano_myr: f64,
    pub edad_max_oceano_myr: f64,
    /// Volumen de corteza continental (km³; el sedimento, con su masa de roca).
    pub volumen_continental_km3: f64,
    /// Fracción de la superficie con corteza continental.
    pub area_continental_frac: f64,
    /// Fracción de la corteza continental en la mayor masa conectada (índice de supercontinente).
    pub supercontinente: f64,
    /// Nivel del mar respecto al dato (m).
    pub nivel_mar_m: f64,
}

/// Cuentas de la corteza continental (km³) a lo largo de la deriva (§8, conservación).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CuentasCorteza {
    pub volumen_inicial_km3: f64,
    /// Engrosamiento de corteza continental en los arcos.
    pub arcos_km3: f64,
    /// Corteza oceánica de arco que pasa a continental (su volumen entero).
    pub acrecion_km3: f64,
    /// Corteza de la placa de abajo que engruesa a la de arriba en los choques.
    pub choque_engrosamiento_km3: f64,
    /// Corteza de la placa de abajo que se pierde al manto en los choques.
    pub choque_perdida_km3: f64,
    /// Corteza que no encontró sitio en la placa de arriba (todo a `H_max`).
    pub choque_sin_sitio_km3: f64,
    /// Corteza continental arrastrada bajo otra placa sin choque (no debería haber).
    pub subducida_km3: f64,
    /// Adelgazamiento de los rifts (H ← f_rift·H).
    pub rift_km3: f64,
    /// Erosión tectónica de las cordilleras (roca).
    pub erosion_km3: f64,
    /// Sedimento depositado en los márgenes (masa de roca).
    pub sedimento_km3: f64,
    /// Sedimento que no cabe en los márgenes y sale al fondo oceánico (se pierde).
    pub sedimento_perdido_km3: f64,
    pub volumen_final_km3: f64,
    /// Inventario de las placas (todas sus muestras, también las que el mundo no ve este paso): es
    /// lo que tocan todos los procesos, así que su balance cierra salvo las copias de corteza.
    pub inventario_inicial_km3: f64,
    pub inventario_final_km3: f64,
    /// Lo que no explican las cuentas en el inventario: el remuestreo de la malla (§8), es decir,
    /// la corteza que se copia al tapar agujeros de los mapas de las placas y al fundir placas.
    pub remuestreo_km3: f64,
}

impl CuentasCorteza {
    /// Cambio que explican las cuentas. El engrosamiento de los choques es un traspaso de la placa
    /// de abajo a la de arriba: no cambia el total.
    pub fn explicado(&self) -> f64 {
        self.arcos_km3 + self.acrecion_km3
            - self.choque_perdida_km3
            - self.choque_sin_sitio_km3
            - self.subducida_km3
            - self.rift_km3
            - self.erosion_km3
            + self.sedimento_km3
    }
}

/// Distribución de edades del fondo oceánico al final (área por franjas de 10 Myr).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdadesOceano {
    pub media_myr: f64,
    pub mediana_myr: f64,
    pub p99_myr: f64,
    pub max_myr: f64,
    pub frac_por_10_myr: Vec<f64>,
}

/// Una placa al final de la deriva.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacaFinal {
    pub id: u64,
    pub area_frac: f64,
    pub continental_frac: f64,
    pub v_rms_cm_anio: f64,
    /// Longitud de su borde donde subduce (km).
    pub fosa_km: f64,
}

/// Resumen de la génesis tectónica (va a la salida del planeta).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumenTectonica {
    pub nivel_malla: u32,
    pub celdas: u64,
    pub lado_km: f64,
    pub pasos: u64,
    /// Nivel del mar que sale del volumen de agua (m, respecto al dato) al final de la deriva.
    pub nivel_mar_m: f64,
    /// Nivel del mar recalculado en la malla del planeta al acabar el prerrodaje (G7; m respecto al
    /// nivel del mar de la tectónica): mide lo que mueven el detalle fino, las fosas estrechas y la
    /// erosión. No se aplica (cambiaría la clasificación de la costa); se informa.
    pub nivel_mar_tras_prerrodaje_m: f64,
    pub f_cont_inicial: f64,
    pub f_cont_final: f64,
    pub cuentas: CuentasCorteza,
    pub edades_oceano: EdadesOceano,
    /// Velocidad media cuadrática media, sin el rodaje (cm/año).
    pub v_rms_media_cm_anio: f64,
    /// Correlación (Spearman) entre la velocidad de las placas y su fracción de borde en fosa
    /// (Forsyth y Uyeda: positiva) y su fracción continental (negativa), sin el rodaje.
    pub forsyth_uyeda_fosa: f64,
    pub forsyth_uyeda_continente: f64,
    pub placas_final: Vec<PlacaFinal>,
    pub eventos: Vec<EventoTectonico>,
    pub metricas: Vec<MetricaMyr>,
}
