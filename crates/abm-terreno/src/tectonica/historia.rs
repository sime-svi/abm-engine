//! Historia geológica del planeta (§7 y §8 del diseño): eventos discretos, métricas de cada Myr y
//! cuentas de conservación de la corteza continental. Es un producto de datos en sí mismo.

use serde::{Deserialize, Serialize};

/// Un evento tectónico de la deriva.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventoTectonico {
    /// Tiempo (Myr; negativo = antes del final de la deriva).
    pub myr: f64,
    /// `rift`, `inicio_subduccion`, `inicio_colision`, `sutura`, `absorcion`, `salto_dorsal`,
    /// `acrecion_arco`, `supercontinente_formado`, `supercontinente_roto`, `inicio_choque`,
    /// `fin_choque`, `cambio_regimen` o `retroceso_margen` (adenda de balance §7).
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
    /// Detalle de los choques y de los márgenes (adenda de balance §7): `arco_continente` o
    /// `continente_continente` en un choque; `erosivo` o `acrecionario` en un cambio de régimen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clase: Option<String>,
    /// Eficacia del choque: fracción de la corteza solapada que se queda en la placa de arriba.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eta: Option<f64>,
    /// Volumen solapado y volumen perdido al manto durante todo el choque (km³), en su fin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub solapado_km3: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perdido_km3: Option<f64>,
    /// Fosas (nota de fosas §7): edad del fondo que se hunde al nacer, y edad de la fosa y longitud
    /// de su losa al morir.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edad_fondo_myr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edad_fosa_myr: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub losa_km: Option<f64>,
    /// Nota de fosas v4: tiempo desde el inicio del choque al romperse la losa, y duración de un
    /// choque al acabar (Myr).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiempo_choque_myr: Option<f64>,
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
    pub transformantes_km: f64,
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
    // --- Balance de la corteza continental (adenda de balance §8) ---
    /// Flujos del paso (AU = km³/año): arcos, erosión por subducción, sedimento subducido, choques
    /// (al manto), delaminación y sedimento que llega al mar.
    pub f_arc_au: f64,
    pub f_se_au: f64,
    pub f_sed_au: f64,
    pub f_col_au: f64,
    pub f_del_au: f64,
    pub f_den_au: f64,
    /// Adelgazamiento de los rifts (AU): sumidero provisional, hasta la nota de rifts.
    pub f_rift_au: f64,
    /// Sedimento de fosa que se pega al margen (AU; traslado interno).
    pub f_acr_au: f64,
    /// Material continental de las placas (km³): `V_cont` de la adenda, todas las muestras.
    pub inventario_km3: f64,
    /// Espesor máximo de la corteza y espesor medio de la continental (km).
    pub espesor_max_km: f64,
    pub espesor_medio_cont_km: f64,
    /// Fracción de la corteza continental de más de 70 km (en la Tierra, el centro del Tíbet y
    /// del Altiplano: del orden del 0,5 %).
    pub area_mas_70km_frac: f64,
    /// Fondo oceánico que se hunde y que se crea (km²/año).
    pub area_subducida_km2_anio: f64,
    pub area_creada_km2_anio: f64,
    /// Fracción del área subducida que entra por márgenes de régimen erosivo.
    pub fosa_erosiva_frac: f64,
    /// Sedimento total (km³, como roca) y espesor medio del sedimento sobre el fondo oceánico (km).
    pub sedimento_km3: f64,
    pub sedimento_medio_oceano_km: f64,
    /// Error relativo del balance del paso (§6.9).
    pub error_balance_rel: f64,
    // --- Bordes de placa (nota_bordes_placa.md §8) ---
    /// Perímetro total de los bordes (km), medido como `tierra_bordes.toml`.
    pub perimetro_km: f64,
    /// Dientes de tramos transformantes o divergentes recolocados en un hueco de su placa y sin
    /// hueco (esperan, ocultos en su placa, al paso siguiente; adenda v5), creación residual en
    /// huecos que no son de dorsal y área reasignada por el suavizado (km²/año).
    pub dientes_recolocados_km2_anio: f64,
    pub dientes_sin_hueco_km2_anio: f64,
    /// Muestras sueltas lejos de su placa: su material pasa a la placa donde están (km²/año).
    pub dientes_a_otra_placa_km2_anio: f64,
    pub creacion_residual_km2_anio: f64,
    pub area_reasignada_km2_anio: f64,
    /// Islas (trozos sueltos de una placa dentro de otra) absorbidas por la que las rodea (km²/año).
    pub area_islas_km2_anio: f64,
    /// Material continental de los dientes (AU): solapes en tramos no convergentes que no se
    /// consumen.
    pub dientes_continentales_au: f64,
    // --- Fosas (nota_ciclo_fosas.md §0 y §8) ---
    /// Convergencia perpendicular media en las fosas, ponderada por longitud (cm/año), y fracción
    /// de la longitud de fosa por debajo de 2 cm/año.
    pub convergencia_media_fosas_cm_anio: f64,
    pub fosa_lenta_frac: f64,
    /// Nota de fosas §8: longitud de convergencia difusa (km, la mitad de cada lado), fondo
    /// absorbido por ella (km²/año), edad media de las fosas activas (Myr) y fracción media del
    /// tirón de su losa, ponderadas por longitud.
    pub difusas_km: f64,
    pub area_difusa_km2_anio: f64,
    pub edad_media_fosas_myr: f64,
    pub losa_media_frac: f64,
    /// Nacimientos (inducidos, placas nuevas al separarse el océano de un margen invadido y
    /// espontáneos en transformante) y muertes (sin convergencia y por choque, al romperse la losa)
    /// del paso.
    pub nacimientos: [u32; 3],
    pub muertes: [u32; 2],
    /// Nota de fosas v4 (§8): longitud invadida en el paso por transformantes y por márgenes
    /// pasivos (km), roturas de losa, convergencia media de los bordes de choque (cm/año), pérdida
    /// de los choques con la losa aún colgando y sin ella (AU), márgenes pasivos (longitud, edad
    /// media y máxima) y los que terminan en el paso (longitud y Σ longitud·vida, km·Myr).
    #[serde(default)]
    pub invadido_km: [f64; 2],
    /// Intentos de separar un océano de delante de un margen invadido, fallidos y celdas de frente
    /// olvidadas (diagnóstico).
    #[serde(default)]
    pub separar: [u32; 3],
    #[serde(default)]
    pub roturas: u32,
    #[serde(default)]
    pub convergencia_media_choques_cm_anio: f64,
    #[serde(default)]
    pub f_col_con_losa_au: f64,
    #[serde(default)]
    pub f_col_sin_losa_au: f64,
    #[serde(default)]
    pub margen_pasivo_km: f64,
    #[serde(default)]
    pub edad_media_margen_myr: f64,
    #[serde(default)]
    pub edad_max_margen_myr: f64,
    #[serde(default)]
    pub margen_fin_km: f64,
    #[serde(default)]
    pub margen_fin_vida_km_myr: f64,
    /// Nota de fosas v5 (§6.4): nacimientos espontáneos en zonas de fractura del paso, intentos de
    /// partir la placa por ellas y fallidos, y longitud (km) de los escalones de edad de más de
    /// `Δa_fz` dentro de las placas oceánicas y de los que cumplen la regla del nacimiento.
    #[serde(default)]
    pub nacimientos_zf: u32,
    #[serde(default)]
    pub separar_zf: [u32; 2],
    #[serde(default)]
    pub zf_escalon_km: f64,
    #[serde(default)]
    pub zf_candidata_km: f64,
}

/// Cuentas del material continental (km³) a lo largo de la deriva (adenda de balance §6 y §8).
///
/// `V_cont = Σ (h_c + S·ρ_s/ρ_c)·A`: la corteza de tipo continental (también la que un arco añade
/// sobre fondo oceánico, sin los 7 km basálticos de base) y el sedimento con su masa de roca.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct CuentasCorteza {
    pub volumen_inicial_km3: f64,
    /// Corteza que fabrican los arcos (`F_arc`, §6.1).
    pub arcos_km3: f64,
    /// Producción de arcos sin franja donde ponerla (placa de arriba más estrecha que la franja).
    pub arcos_sin_franja_km3: f64,
    /// Erosión por subducción (`F_SE`, §6.3): pasa al manto.
    pub erosion_subduccion_km3: f64,
    /// Erosión por subducción sin corteza continental cerca de la fosa (no ocurre).
    pub erosion_subduccion_sin_corteza_km3: f64,
    /// Sedimento que llega a las fosas (§6.4): el que se pega al margen y el que va al manto
    /// (`F_sed`).
    pub sedimento_a_fosa_km3: f64,
    pub sedimento_acrecionado_km3: f64,
    pub sedimento_subducido_km3: f64,
    /// Choques (§6.5): corteza continental solapada, la que engruesa la placa de arriba, la que va
    /// al manto (`F_col`) y la que no cupo bajo `H_max` (también al manto).
    pub choque_solapado_km3: f64,
    pub choque_engrosamiento_km3: f64,
    pub choque_perdida_km3: f64,
    /// La pérdida de los choques según si la losa de la fosa aún colgaba (nota de fosas v4 §6.6) o
    /// ya se había roto (o no la hubo): diagnóstico, sus sumas son `choque_perdida_km3`.
    #[serde(default)]
    pub choque_perdida_con_losa_km3: f64,
    #[serde(default)]
    pub choque_perdida_sin_losa_km3: f64,
    /// Diagnóstico de la adenda v7 (§6.5): corteza solapada en los choques (sin la de arco) y la
    /// que se pierde, por la muestra que entra: continental de menos de `h_1`, de `h_1` a `h_2` y de
    /// más de `h_2`, y la que un arco o la acreción pusieron sobre fondo oceánico.
    #[serde(default)]
    pub choque_solapado_por_grosor_km3: [f64; 4],
    #[serde(default)]
    pub choque_perdida_por_grosor_km3: [f64; 4],
    pub choque_sin_sitio_km3: f64,
    /// Delaminación (`F_del`, §6.6).
    pub delaminacion_km3: f64,
    /// Corteza que el flujo lateral pasa de una columna gruesa a sus vecinas (§6.6b; traslado).
    pub flujo_lateral_km3: f64,
    /// Adelgazamiento de los rifts (`H ← f_rift·H`): sumidero provisional. En la Tierra estirar
    /// conserva el volumen; se revisa en la nota de rifts.
    pub rift_km3: f64,
    /// Erosión de la génesis (§6.7, traslado interno): llega al mar (`F_den`) o a cuencas cerradas.
    pub erosion_km3: f64,
    pub sedimento_al_mar_km3: f64,
    pub sedimento_en_cuencas_km3: f64,
    /// Parte del sedimento al mar que no cabía en el mar de su desembocadura (lleno hasta la
    /// plataforma) y siguió hasta el mar más cercano con sitio (traslado; diagnóstico).
    #[serde(default)]
    pub sedimento_desbordado_km3: f64,
    /// Sedimento sin celda donde ponerse (sin dueño; no debería haber): se pierde.
    pub sedimento_sin_destino_km3: f64,
    /// Área de arcos oceánicos que pasa a corteza continental (más de `H_cont_min`; km²).
    pub arco_a_continental_km2: f64,
    pub volumen_final_km3: f64,
    /// Inventario de las placas (todas sus muestras, también las que el mundo no ve este paso): es
    /// lo que tocan todos los procesos, así que su balance cierra exactamente.
    pub inventario_inicial_km3: f64,
    pub inventario_final_km3: f64,
    /// Lo que no explican las cuentas en el inventario: debe ser error de redondeo (§0).
    pub remuestreo_km3: f64,
    /// Peor error relativo del balance de un paso (§6.9).
    pub error_balance_max_rel: f64,
    /// Diagnóstico de la forma de los bordes: fondo oceánico consumido (km²) y corteza continental
    /// solapada en choques (km³) según el tipo del borde donde ocurre: 0 sin borde, 1 divergente,
    /// 2 subducción, 3 colisión, 4 transformante.
    pub area_subducida_por_borde_km2: [f64; 6],
    pub choque_solapado_por_borde_km3: [f64; 6],
}

impl CuentasCorteza {
    /// Cambio que explican las cuentas (§6.9): fuentes menos sumideros. El engrosamiento de los
    /// choques, el sedimento que se pega al margen y la erosión con su sedimento solo trasladan.
    pub fn explicado(&self) -> f64 {
        self.arcos_km3
            - self.erosion_subduccion_km3
            - self.sedimento_subducido_km3
            - self.choque_perdida_km3
            - self.choque_sin_sitio_km3
            - self.delaminacion_km3
            - self.rift_km3
            - self.sedimento_sin_destino_km3
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
    /// Medias del balance de la corteza sin el rodaje (adenda de balance §8 y §10).
    pub balance: BalanceMedio,
    pub placas_final: Vec<PlacaFinal>,
    pub eventos: Vec<EventoTectonico>,
    pub metricas: Vec<MetricaMyr>,
}

/// Medias del balance de la corteza continental tras el rodaje (adenda de balance §10).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BalanceMedio {
    /// Flujos medios (AU).
    pub arcos_au: f64,
    pub erosion_subduccion_au: f64,
    pub sedimento_subducido_au: f64,
    pub sedimento_acrecionado_au: f64,
    pub choques_au: f64,
    pub delaminacion_au: f64,
    pub sedimento_al_mar_au: f64,
    pub rift_au: f64,
    /// Fondo oceánico que se hunde y que se crea (km²/año).
    pub area_subducida_km2_anio: f64,
    pub area_creada_km2_anio: f64,
    pub fosa_erosiva_frac: f64,
    /// Longitud media de los bordes por tipo (km), medida como `tierra_bordes.toml`.
    pub dorsales_km: f64,
    pub fosas_km: f64,
    pub colisiones_km: f64,
    pub transformantes_km: f64,
    /// Espesor máximo de la corteza: medio y al final (km).
    pub espesor_max_medio_km: f64,
    pub espesor_max_final_km: f64,
    /// Espesor máximo en toda la deriva, con el rodaje (alarma de la adenda v3: no más de 80 km).
    pub espesor_max_todo_km: f64,
    /// Fracción de la corteza continental de más de 70 km: media y al final.
    pub area_mas_70km_frac: f64,
    pub area_mas_70km_final_frac: f64,
    /// Cambio relativo de `V_cont` desde el final del rodaje hasta el final de la deriva.
    pub v_cont_cambio_frac: f64,
    /// Espesor medio del sedimento sobre el fondo oceánico al final (km).
    pub sedimento_medio_oceano_km: f64,
    /// Bordes (nota de bordes §8): perímetro medio (km); dientes recolocados, sin hueco, creación
    /// residual y área reasignada (km²/año); material continental de los dientes (AU).
    pub perimetro_km: f64,
    pub dientes_recolocados_km2_anio: f64,
    pub dientes_sin_hueco_km2_anio: f64,
    pub creacion_residual_km2_anio: f64,
    pub area_reasignada_km2_anio: f64,
    pub dientes_continentales_au: f64,
    /// Fosas (nota de fosas §0): convergencia media ponderada por longitud (cm/año), fracción de
    /// longitud por debajo de 2 cm/año y reparto de la longitud por clases de convergencia (límites
    /// en `CLASES_CONVERGENCIA_CM_ANIO`, cm/año), sin el rodaje.
    pub convergencia_media_fosas_cm_anio: f64,
    pub fosa_lenta_frac: f64,
    pub fosas_por_convergencia_frac: Vec<f64>,
    /// Nota de fosas §8 (medias tras el rodaje): convergencia difusa (km y km²/año), edad media de
    /// las fosas (Myr), fracción media del tirón, y nacimientos (inducidos, en margen pasivo, en
    /// transformante) y muertes (sin convergencia, por choque) por cada 100 Myr.
    pub difusas_km: f64,
    pub area_difusa_km2_anio: f64,
    pub edad_media_fosas_myr: f64,
    pub losa_media_frac: f64,
    pub nacimientos_100myr: [f64; 3],
    pub muertes_100myr: [f64; 2],
    /// Nota de fosas v4 (medias tras el rodaje): invasión por transformantes y por márgenes
    /// (km/Myr), roturas de losa por 100 Myr, convergencia media de los bordes de choque (cm/año) y
    /// pérdida de los choques con la losa colgando y sin ella (AU).
    #[serde(default)]
    pub invadido_transf_km_myr: f64,
    #[serde(default)]
    pub invadido_margen_km_myr: f64,
    #[serde(default)]
    pub roturas_100myr: f64,
    #[serde(default)]
    pub convergencia_media_choques_cm_anio: f64,
    #[serde(default)]
    pub choques_con_losa_au: f64,
    #[serde(default)]
    pub choques_sin_losa_au: f64,
    /// Márgenes pasivos (v4 §8): longitud media (km); edad media ponderada por longitud y máxima
    /// (Myr), media de los pasos y al final; vida media de los que terminan (Myr) y su ritmo
    /// (km/Myr). Choques continente–continente que acaban: duración media ponderada por la corteza
    /// solapada (Myr) y cuántos.
    #[serde(default)]
    pub margen_pasivo_km: f64,
    #[serde(default)]
    pub edad_media_margen_myr: f64,
    #[serde(default)]
    pub edad_max_margen_myr: f64,
    #[serde(default)]
    pub edad_media_margen_final_myr: f64,
    #[serde(default)]
    pub edad_max_margen_final_myr: f64,
    #[serde(default)]
    pub vida_media_margen_myr: f64,
    #[serde(default)]
    pub margen_terminado_km_myr: f64,
    #[serde(default)]
    pub duracion_media_choques_myr: f64,
    #[serde(default)]
    pub choques_terminados: u32,
    /// Nota de fosas v5 (medias tras el rodaje): nacimientos en zonas de fractura por 100 Myr,
    /// fracción de los intentos de partir la placa que fallan, y longitud media de los escalones
    /// de edad de más de `Δa_fz` y de los candidatos (km).
    #[serde(default)]
    pub nacimientos_zf_100myr: f64,
    #[serde(default)]
    pub separar_zf_fallidos_frac: f64,
    #[serde(default)]
    pub zf_escalon_km: f64,
    #[serde(default)]
    pub zf_candidata_km: f64,
}
