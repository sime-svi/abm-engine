//! Configuración de la simulación de terreno (CLAUDE §5: todo parámetro vive en `config/*.toml`).
//!
//! Los valores por defecto son los del documento madre **reescalados al tick diario** (`T_anio=365`):
//! toda tasa por tick se multiplica por 48/365, toda cadencia por 365/48 (diseño §7).

use serde::{Deserialize, Serialize};

use abm_core::ConfigError;

/// Configuración completa de una ejecución de terreno.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TerrenoConfig {
    pub run: RunCfg,
    pub planet: PlanetCfg,
    pub worldgen: WorldgenCfg,
    pub climate: ClimateCfg,
    pub soil: SoilCfg,
    pub vegetation: VegCfg,
    pub lod: LodCfg,
    pub erosion: ErosionCfg,
    pub tectonica: TectonicaCfg,
    pub persistence: PersistenceCfg,
}

/// Generador del relieve de nacimiento. Interruptor mientras se mide el relieve tectónico frente al
/// anterior (CLAUDE §5); se retirará cuando decida el responsable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relieve {
    /// Ruido fractal con la curva hipsométrica `lf³` (incrementos 1 a 3).
    #[default]
    Ruido,
    /// Tectónica de placas con deriva (incremento 4, `docs/design/terreno/diseno_relieve_tectonico.md`).
    Tectonico,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunCfg {
    pub name: String,
    pub seed: u64,
    pub total_ticks: u64,
    pub output_path: String,
    /// Vuelca además un snapshot por celda del mundo generado (para inspección).
    pub dump_world: bool,
    /// Añade al snapshot el clima de cada celda: factor orográfico, agotamiento de humedad y lluvia
    /// media anual (nota de sombra pluviométrica).
    pub dump_climate: bool,
    /// Carpeta de la caché de la génesis ("" = sin caché). La génesis depende solo de la semilla, la
    /// config y la versión del código: se calcula una vez y se reutiliza.
    pub genesis_cache: String,
    /// Añade al diagnóstico el bloque ampliado del relieve (esquema v6) también con el relieve de
    /// ruido, para compararlo con el tectónico (con el tectónico va siempre).
    pub diagnostico_relieve: bool,
}

impl Default for RunCfg {
    fn default() -> Self {
        Self {
            name: "terreno".to_string(),
            seed: 42,
            total_ticks: 365,
            output_path: "output".to_string(),
            dump_world: false,
            dump_climate: false,
            genesis_cache: String::new(),
            diagnostico_relieve: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PlanetCfg {
    pub r_km: f64,
    /// Celdas por arista de cara en L=0.
    pub n0: u32,
    pub sea_level: f64,
    pub h_min: f64,
    pub h_max: f64,
}

impl Default for PlanetCfg {
    fn default() -> Self {
        Self {
            r_km: 6371.0,
            n0: 48,
            sea_level: 0.0,
            h_min: -11000.0,
            h_max: 9000.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WorldgenCfg {
    pub noise_octaves: u32,
    pub persistence: f64,
    pub lacunarity: f64,
    pub base_freq: f64,
    /// Octavas de ruido que se añaden por cada nivel por encima de `lod.l_min` (incremento 2 §3.1):
    /// con lacunaridad 2, una por nivel mantiene la razón tamaño de celda / detalle más fino.
    pub octaves_per_level: u32,
    /// Exponente hipsométrico de la tierra emergida (>1 => domina la tierra baja, tipo Tierra).
    pub land_exp: f64,
    /// Umbral de área de drenaje (km²) para que una celda sea río.
    pub river_threshold_km2: f64,
    /// Densidad del recurso genérico (fracción de celdas de tierra con depósito, 0..1).
    pub resource_density: f64,
    /// Generador del relieve: `"ruido"` (incrementos 1 a 3) o `"tectonico"` (incremento 4).
    pub relieve: Relieve,
}

impl Default for WorldgenCfg {
    fn default() -> Self {
        Self {
            noise_octaves: 6,
            persistence: 0.5,
            lacunarity: 2.0,
            base_freq: 1.6,
            octaves_per_level: 1,
            land_exp: 3.0,
            river_threshold_km2: 80_000.0,
            resource_density: 0.05,
            relieve: Relieve::Ruido,
        }
    }
}

/// Relieve tectónico (incremento 4): deriva de placas durante cientos de millones de años antes de
/// la génesis geológica. Diseño: `docs/design/terreno/diseno_relieve_tectonico.md` (del responsable)
/// y `incremento-4-diseno.md` (técnico). Valores físicos medidos por defecto (CLAUDE §2.8).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TectonicaCfg {
    // --- Planeta y agua (§5.1) ---
    /// Capa global de agua: volumen del océano actual / superficie del planeta (m). Sin el agua del
    /// hielo mientras no haya casquetes (decisión del responsable).
    pub gel_m: f64,
    /// Fracción de la superficie con corteza continental al empezar la deriva. El valor final es un
    /// resultado a validar, no un objetivo (decisión del responsable).
    pub f_cont: f64,
    // --- Corteza e isostasia (§5.2) ---
    pub rho_c: f64,
    pub rho_m: f64,
    /// Densidad de la corteza oceánica (kg/m³): flotación de los arcos sobre fondo oceánico.
    pub rho_oc: f64,
    /// Densidad del agua de mar (kg/m³): la parte sumergida de una columna flota con agua encima.
    pub rho_w: f64,
    /// Densidad del sedimento compactado de los márgenes (kg/m³).
    pub rho_s: f64,
    pub h_oc_km: f64,
    pub h0_km: f64,
    pub sigma_h0_km: f64,
    pub h_max_km: f64,
    /// Espesor continental que queda al nivel del mar (relación empírica terrestre, §6.7).
    pub h_nivel_km: f64,
    pub h_cont_min_km: f64,
    // --- Fondo oceánico (Parsons y Sclater, 1977; §5.3) ---
    pub d_r_m: f64,
    pub c_hs: f64,
    pub d_inf_m: f64,
    pub a_pl_m: f64,
    pub tau_pl_myr: f64,
    pub t_x_myr: f64,
    pub d_fosa_m: f64,
    pub w_fosa_km: f64,
    // --- Bordes de placa (§5.4) ---
    pub d_arco_km: f64,
    pub w_arco_km: f64,
    // --- Balance de la corteza continental (adenda_balance_corteza.md §5) ---
    /// Corteza que fabrica un arco por cada km² de placa que se hunde (km³/km²): sobre placa
    /// oceánica y sobre margen continental (Clift et al. 2009, divididos por la convergencia media).
    pub p_arc_oc: f64,
    pub p_arc_cont: f64,
    /// Erosión por subducción por cada km² de placa que se hunde (km³/km²).
    pub p_se: f64,
    /// Un margen es erosivo si converge a más de `v_ero` y le llega menos de `s_ero` de sedimento
    /// (Clift y Vannucchi 2004).
    pub v_ero_cm_anio: f64,
    pub s_ero_km: f64,
    /// Fracción del sedimento de fosa que se pega a un margen acrecionario (calibrado).
    pub f_acr: f64,
    /// Fracción de la corteza solapada que se queda en la placa de arriba en un choque continente–
    /// continente (India–Asia) y en uno arco–continente (Luzón–Taiwán).
    pub eta_col: f64,
    pub eta_arco: f64,
    /// Delaminación: por encima de `h_del` la base de la corteza pasa a eclogita y se desprende con
    /// tiempo característico `tau_del` (calibrado).
    pub h_del_km: f64,
    pub tau_del_myr: f64,
    pub w_min_orog_km: f64,
    pub w_margen_km: f64,
    pub f_rift: f64,
    // --- Dinámica de placas (§5.5) ---
    pub f_sp: f64,
    pub f_rp: f64,
    pub t_sat_myr: f64,
    /// Resistencia de colisión: fuerza por km de frente = `c_col · c_b · v_conv`, es decir, el
    /// rozamiento basal de `c_col` km² de placa por cada km de frente. Se calibra con un choque que
    /// frene la convergencia a ~1/3 en 10–20 Myr (India–Asia, §5.5).
    pub c_col: f64,
    pub kappa_cont: f64,
    /// Velocidad media cuadrática de la litosfera (cm/año): NNR-MORVEL56, Argus et al. (2011).
    pub v_obj_cm_anio: f64,
    /// Cada cuántos Myr se recalibra el rozamiento basal `c_b` a `v_obj`.
    pub recalibrar_myr: f64,
    /// Empujón inicial: velocidad media de cada placa sorteada en este rango (cm/año).
    pub v_inicial_min_cm_anio: f64,
    pub v_inicial_max_cm_anio: f64,
    /// Fracción de la velocidad relativa por debajo de la cual un borde cuenta como transformante
    /// (sen 15°: bordes a menos de 15° del desgarre puro).
    pub f_transformante: f64,
    // --- Eventos de placas (§5.6) ---
    pub n_p0: u32,
    pub n_cratones: u32,
    pub n_min: u32,
    pub a_min_frac: f64,
    pub tau_rift_myr: f64,
    pub v_rift_cm_anio: f64,
    pub v_sutura_cm_anio: f64,
    pub t_sutura_myr: f64,
    pub tau_orog_myr: f64,
    /// Deformación de los bordes de Voronoi, de los cratones y de las líneas de rift (fracción del
    /// radio): los bordes reales no son rectos.
    pub deformacion: f64,
    // --- Malla, tiempo y detalle (§5.7) ---
    /// Nivel del cubo esférico del terreno en que corre la tectónica (2 = ~52 km con n0 = 48).
    pub nivel_malla: u32,
    pub dt_myr: f64,
    pub t_deriva_myr: f64,
    /// Tramo inicial de la deriva que no cuenta para la validación (Myr).
    pub t_rodaje_myr: f64,
    /// Ruido de detalle: desviación típica (m) según la altura de base, en las cotas dadas
    /// (interpolación lineal). Continúa el espectro del relieve real (ETOPO, por escala y tipo).
    pub detalle_cotas_m: Vec<f64>,
    pub detalle_sigma_m: Vec<f64>,
    /// Razón de amplitud entre octavas sucesivas del ruido de detalle (2^−H, H = exponente de Hurst).
    pub detalle_persistencia: f64,
    /// Radio de flexión para el rebote isostático de la erosión (km).
    pub r_flex_km: f64,
}

impl Default for TectonicaCfg {
    fn default() -> Self {
        Self {
            gel_m: 2617.0,
            f_cont: 0.40,
            rho_c: 2800.0,
            rho_m: 3300.0,
            rho_oc: 2900.0,
            rho_w: 1030.0,
            rho_s: 2400.0,
            h_oc_km: 7.0,
            h0_km: 35.0,
            sigma_h0_km: 3.0,
            h_max_km: 80.0,
            h_nivel_km: 32.0,
            h_cont_min_km: 20.0,
            d_r_m: 2500.0,
            c_hs: 350.0,
            d_inf_m: 6400.0,
            a_pl_m: 3200.0,
            tau_pl_myr: 62.8,
            t_x_myr: 20.0,
            d_fosa_m: 3000.0,
            w_fosa_km: 50.0,
            d_arco_km: 200.0,
            w_arco_km: 150.0,
            p_arc_oc: 1.2,
            p_arc_cont: 0.8,
            p_se: 0.4,
            v_ero_cm_anio: 6.0,
            s_ero_km: 1.0,
            f_acr: 0.3,
            eta_col: 0.5,
            eta_arco: 0.9,
            h_del_km: 50.0,
            tau_del_myr: 50.0,
            w_min_orog_km: 150.0,
            w_margen_km: 150.0,
            f_rift: 0.7,
            f_sp: 1.0,
            f_rp: 0.2,
            t_sat_myr: 80.0,
            c_col: 90_000.0,
            kappa_cont: 3.0,
            v_obj_cm_anio: 4.39,
            recalibrar_myr: 50.0,
            v_inicial_min_cm_anio: 1.0,
            v_inicial_max_cm_anio: 10.0,
            f_transformante: 0.26,
            n_p0: 12,
            n_cratones: 8,
            n_min: 8,
            a_min_frac: 0.005,
            tau_rift_myr: 150.0,
            v_rift_cm_anio: 2.0,
            v_sutura_cm_anio: 0.5,
            t_sutura_myr: 10.0,
            tau_orog_myr: 63.0,
            deformacion: 0.06,
            nivel_malla: 2,
            dt_myr: 1.0,
            t_deriva_myr: 500.0,
            t_rodaje_myr: 50.0,
            detalle_cotas_m: vec![-4000.0, -200.0, 0.0, 500.0, 2000.0, 5000.0],
            detalle_sigma_m: vec![200.0, 150.0, 150.0, 150.0, 1000.0, 1500.0],
            detalle_persistencia: 0.5,
            r_flex_km: 100.0,
        }
    }
}

impl TectonicaCfg {
    /// Factores de isostasia (km de altura por km de espesor): al aire `(ρ_m − ρ)/ρ_m` y bajo el
    /// agua `(ρ_m − ρ)/(ρ_m − ρ_w)` (corrección de realismo C1, `incremento-4-diseno.md`).
    pub fn flotacion(&self, rho: f64) -> (f64, f64) {
        (
            (self.rho_m - rho) / self.rho_m,
            (self.rho_m - rho) / (self.rho_m - self.rho_w),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ClimateCfg {
    pub t_equator: f64,
    pub k_lat: f64,
    pub lapse: f64,
    pub seas_amp_pole: f64,
    pub t_year: f64,
    pub t_freeze: f64,
    pub k_melt: f64,
    pub p_ref: f64,
    pub k_oro: f64,
    pub m_oro_min: f64,
    pub m_oro_max: f64,
    pub moisture_length_km: f64,
    pub seas_precip_amp: f64,
    /// Escala (km) a la que la lluvia "ve" el relieve (nota de lluvia orográfica): el factor
    /// orográfico usa la pendiente de un relieve de referencia a esta escala, igual en malla fina o
    /// gruesa. También es el paso del camino del viento del agotamiento de humedad.
    pub d_oro_km: f64,
    /// Agotamiento de humedad a favor del viento (nota de sombra pluviométrica §3): el aire que
    /// subió una cordillera soltó su vapor y no lo recupera al bajar. `H_v` (m): subida que deja al
    /// aire con ~37 % de su vapor.
    pub h_v_m: f64,
    /// `L_recarga` (km): distancia de recarga de humedad sobre tierra (las subidas lejanas pesan menos).
    pub l_recarga_km: f64,
    /// `L_traj` (km): longitud máxima del camino del viento hacia atrás.
    pub l_traj_km: f64,
    /// Conservación de la lluvia total sobre tierra (nota de sombra pluviométrica §2): si la
    /// orografía y el agotamiento la desvían más de `tol_lluvia` de la de referencia (sin efecto
    /// orográfico ni agotamiento), se calcula en la génesis un factor global `c_P` sobre la lluvia
    /// de la tierra que recupera el total. **Provisional** (CLAUDE §2.8): lo realista será calibrar
    /// la lluvia de base con datos de la Tierra cuando el relieve sea realista.
    pub tol_lluvia: f64,
}

impl Default for ClimateCfg {
    fn default() -> Self {
        Self {
            t_equator: 30.0,
            k_lat: 0.6,
            lapse: 6.5,
            seas_amp_pole: 25.0,
            t_year: 365.0,
            t_freeze: 0.0,
            k_melt: 0.26, // 2 mm/(°C·tick) · 48/365
            // Reescalado mecánico daría 2.6; se calibra a 5.0 (≈1825 mm/año en el ecuador) para que
            // emerja la selva tropical, que necesita lluvia alta. Ver README de la fase.
            p_ref: 5.0,
            // Nota de sombra pluviométrica §2: el valor físico (laderas reales de ~0,04 dan ×2,5 en
            // barlovento). Con este planeta solo da ×1,4 porque su relieve es ~4 veces más suave que
            // el real; por realismo (CLAUDE §2.8) no se fuerza k_oro: se corregirá el relieve (§7).
            k_oro: 40.0,
            m_oro_min: 0.2,
            m_oro_max: 3.0,
            moisture_length_km: 1500.0,
            seas_precip_amp: 0.0,
            d_oro_km: 25.0,
            h_v_m: 2500.0,
            l_recarga_km: 3000.0,
            l_traj_km: 2000.0,
            tol_lluvia: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SoilCfg {
    pub runoff_base: f64,
    pub k_slope: f64,
    pub k_sat: f64,
    pub k_et: f64,
    pub k_lat_flow: f64,
    pub w_cap_min: f64,
    pub w_cap_max: f64,
    /// Profundidad de suelo inicial (m). Con `z = z_init` la capacidad de campo es la de la textura.
    pub z_init: f64,
    /// Meteorización máxima (m/año): ritmo al que la roca se convierte en suelo (incr. 3 §7).
    pub k_weath: f64,
    /// Profundidad de referencia de la producción de suelo (m): más suelo => se forma más despacio.
    pub z_ref: f64,
    /// Temperatura de referencia de la meteorización (°C).
    pub t_ref: f64,
    /// Lluvia de referencia de la meteorización (mm/año): por encima, la humedad no acelera más.
    pub p_ref_soil: f64,
    /// Profundidad de raíces (m): más suelo (o sedimento) no retiene más agua útil.
    pub z_root: f64,
    /// Fracción de la capacidad de campo que conserva la roca desnuda en las celdas del nivel 0
    /// (grietas, suelo disperso en vaguadas): una celda de decenas de km nunca es solo roca.
    pub f_rock: f64,
    /// Ritmo al que baja esa fracción con el nivel: `f_rock(L) = f_rock · 2^(−f_rock_beta·L)`. A
    /// escala de 1 km un acantilado sí es solo roca.
    pub f_rock_beta: f64,
}

impl Default for SoilCfg {
    fn default() -> Self {
        Self {
            runoff_base: 0.10,
            k_slope: 0.5,
            k_sat: 0.4,
            k_et: 0.13,         // 1.0 mm/(°C·tick) · 48/365
            k_lat_flow: 0.0066, // 0.05 · 48/365
            w_cap_min: 50.0,
            w_cap_max: 300.0,
            z_init: 1.0,
            k_weath: 1e-4,
            z_ref: 0.5,
            t_ref: 15.0,
            p_ref_soil: 1000.0,
            z_root: 1.5,
            f_rock: 0.3,
            f_rock_beta: 0.5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct VegCfg {
    pub theta_opt: f64,
    pub sigma_theta: f64,
    pub t_opt: f64,
    pub sigma_t: f64,
    pub k_max: f64,
    pub r_veg: f64,
    pub n_veg: u64,
    /// Biomasa inicial de la tierra (génesis) y de la tierra nueva que aparece al afinar la costa.
    pub v_init: f64,
}

impl Default for VegCfg {
    fn default() -> Self {
        Self {
            theta_opt: 0.6,
            sigma_theta: 0.25,
            t_opt: 22.0,
            sigma_t: 12.0,
            k_max: 100.0,
            r_veg: 0.026, // 0.2 /tick · 48/365
            n_veg: 30,    // 4 ticks · 365/48
            v_init: 1.0,
        }
    }
}

/// LOD dinámico (incremento 2, `docs/design/terreno/incremento-2-diseno.md` §8 y §11).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LodCfg {
    /// Nivel base uniforme de arranque (sustituye a `planet.l_unif` del incremento 1).
    pub l_min: u32,
    /// Nivel máximo de subdivisión. `l_max = l_min` => LOD apagado (malla uniforme).
    pub l_max: u32,
    /// Cadencia del paso de topología (ticks).
    pub n_lod: u64,
    /// Variación de altura con algún vecino de tierra que dispara subdividir (m).
    pub dh_sub: f64,
    /// Variación de temperatura con algún vecino de tierra que dispara subdividir (°C).
    pub dt_sub: f64,
    /// Variación de humedad relativa del suelo con algún vecino de tierra que dispara subdividir.
    pub dtheta_sub: f64,
    /// Histéresis: solo se colapsa si las variaciones bajan de `h_hist · umbral` (0 < h_hist ≤ 1).
    pub h_hist: f64,
    /// Refinar los rasgos geográficos: costa, ríos y lagos.
    pub refine_features: bool,
    /// Tope de subdivisiones que pide el driver en cada revisión, como fracción de las hojas
    /// (0 = sin tope). No se aplica al refinado inicial de la génesis, que debe converger.
    pub max_sub_frac: f64,
}

impl Default for LodCfg {
    fn default() -> Self {
        Self {
            l_min: 0,
            l_max: 3,
            n_lod: 60,
            dh_sub: 500.0,
            dt_sub: 4.0,
            dtheta_sub: 0.3,
            h_hist: 0.5,
            refine_features: true,
            max_sub_frac: 0.02,
        }
    }
}

impl LodCfg {
    /// `true` si el LOD dinámico está activo (hay margen para subdividir).
    pub fn enabled(&self) -> bool {
        self.l_max > self.l_min
    }
}

/// Relieve vivo (incremento 3, `docs/design/terreno/incremento-3-diseno.md` §2 y §12). Tasas en
/// unidades por año (no por tick): el mismo paso sirve para la génesis geológica y la ejecución.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ErosionCfg {
    /// Pasos de la génesis geológica antes del tick 0 (0 = sin génesis).
    pub geo_steps: u32,
    /// Años por paso de la génesis geológica.
    pub geo_dt_years: f64,
    /// Rondas erosión ↔ refinado LOD en que se reparten los pasos de la génesis.
    pub geo_rounds: u32,
    /// Erodibilidad `K` de la potencia de arroyo (m^(1−3m)·año^(m−1)), con el caudal en m³/año.
    pub k_ero: f64,
    /// Exponente del caudal `m` (el de la pendiente es `n = 1`, fijo: esquema implícito lineal).
    pub m_ero: f64,
    /// Difusividad de ladera efectiva (m²/año): procesos por debajo del tamaño de celda.
    pub kappa_dif: f64,
    /// Cada cuántos ticks se erosiona durante la simulación (0 = nunca).
    pub run_every: u64,
    /// Acelerador del tiempo geológico en ejecución (1 = realista).
    pub run_accel: f64,
}

impl ErosionCfg {
    /// `true` si el relieve está vivo (hay génesis geológica o erosión en ejecución).
    pub fn enabled(&self) -> bool {
        self.geo_steps > 0 || self.run_every > 0
    }
}

impl Default for ErosionCfg {
    fn default() -> Self {
        Self {
            geo_steps: 200,
            geo_dt_years: 5000.0,
            geo_rounds: 2,
            k_ero: 1e-6,
            m_ero: 0.5,
            kappa_dif: 10.0,
            run_every: 365,
            run_accel: 1.0,
        }
    }
}

/// Destino de la salida (reutiliza el sink del motor: aquí solo JSON en este incremento).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SinkKind {
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PersistenceCfg {
    pub sink: SinkKind,
}

impl Default for PersistenceCfg {
    fn default() -> Self {
        Self {
            sink: SinkKind::Json,
        }
    }
}

impl TerrenoConfig {
    /// Carga y valida la configuración desde un archivo TOML.
    pub fn load(path: &str) -> Result<TerrenoConfig, ConfigError> {
        Self::load_con(path, &[])
    }

    /// Carga la config con ajustes `ruta=valor` encima (los `--set` de la línea de comandos; ver
    /// `abm_core::config::cargar`) y la valida.
    pub fn load_con(path: &str, ajustes: &[String]) -> Result<TerrenoConfig, ConfigError> {
        let cfg: TerrenoConfig = abm_core::config::cargar(path, ajustes)?;
        cfg.validate()?;
        Ok(cfg)
    }

    /// Número de celdas por arista de cara al nivel base: `n0 · 2^l_min`.
    pub fn face_n(&self) -> u32 {
        self.planet.n0 << self.lod.l_min
    }

    /// Comprueba invariantes que el TOML por sí solo no garantiza.
    pub fn validate(&self) -> Result<(), ConfigError> {
        let invalid = |m: &str| ConfigError::Invalid(m.to_string());
        if self.planet.n0 == 0 {
            return Err(invalid("planet.n0 debe ser > 0"));
        }
        if self.lod.l_min > self.lod.l_max {
            return Err(invalid("se requiere lod.l_min <= lod.l_max"));
        }
        if self.lod.l_max > 12 {
            return Err(invalid("lod.l_max fuera de rango razonable (0..=12)"));
        }
        // La clave de celda reserva 28 bits por coordenada y la vecindad mira un nivel más (§2).
        if (self.planet.n0 as u64) << (self.lod.l_max + 1) > 1u64 << 28 {
            return Err(invalid("planet.n0 · 2^(lod.l_max+1) no cabe en 28 bits"));
        }
        if self.lod.n_lod == 0 {
            return Err(invalid("lod.n_lod debe ser >= 1"));
        }
        if !(self.lod.h_hist > 0.0 && self.lod.h_hist <= 1.0) {
            return Err(invalid("se requiere 0 < lod.h_hist <= 1"));
        }
        if !(self.lod.dh_sub > 0.0 && self.lod.dt_sub > 0.0 && self.lod.dtheta_sub > 0.0) {
            return Err(invalid(
                "los umbrales lod.dh_sub, lod.dt_sub y lod.dtheta_sub deben ser > 0",
            ));
        }
        if self.vegetation.v_init < 0.0 {
            return Err(invalid("vegetation.v_init debe ser >= 0"));
        }
        // La precipitación media anual analítica de la erosión exige que el factor estacional no
        // se recorte en cero (incr. 3 §4).
        if !(0.0..=1.0).contains(&self.climate.seas_precip_amp) {
            return Err(invalid("se requiere 0 <= climate.seas_precip_amp <= 1"));
        }
        // El agotamiento de humedad recorre anillos con puntos cada d_oro: por debajo de 10 km
        // serían más de 5 millones de puntos (nota de sombra pluviométrica §6.1).
        if !(self.climate.d_oro_km >= 10.0 && self.climate.d_oro_km < 0.5 * self.planet.r_km) {
            return Err(invalid("se requiere 10 <= climate.d_oro_km < r_km/2"));
        }
        let c = &self.climate;
        if !(c.h_v_m > 0.0 && c.l_recarga_km > 0.0 && c.l_traj_km >= 0.0) {
            return Err(invalid(
                "se requiere climate.h_v_m > 0, l_recarga_km > 0 y l_traj_km >= 0",
            ));
        }
        if !(c.tol_lluvia >= 0.0 && c.tol_lluvia.is_finite()) {
            return Err(invalid("se requiere climate.tol_lluvia >= 0"));
        }
        let e = &self.erosion;
        if e.geo_steps > 0 && (e.geo_rounds == 0 || e.geo_rounds > e.geo_steps) {
            return Err(invalid(
                "se requiere 1 <= erosion.geo_rounds <= erosion.geo_steps",
            ));
        }
        if e.geo_dt_years <= 0.0 || e.k_ero < 0.0 || e.kappa_dif < 0.0 || e.run_accel < 0.0 {
            return Err(invalid(
                "se requiere erosion.geo_dt_years > 0 y k_ero, kappa_dif, run_accel >= 0",
            ));
        }
        if !(e.m_ero > 0.0 && e.m_ero <= 2.0) {
            return Err(invalid("se requiere 0 < erosion.m_ero <= 2"));
        }
        let s = &self.soil;
        if !(s.z_init > 0.0 && s.z_ref > 0.0 && s.t_ref > 0.0 && s.p_ref_soil > 0.0)
            || s.k_weath < 0.0
        {
            return Err(invalid(
                "se requiere soil.z_init, z_ref, t_ref, p_ref_soil > 0 y soil.k_weath >= 0",
            ));
        }
        // Con el suelo intacto (z = z_init) la capacidad debe ser la de la textura: z_init dentro
        // de [f_rock·z_init, z_root].
        if !(s.z_root >= s.z_init && s.f_rock > 0.0 && s.f_rock <= 1.0 && s.f_rock_beta >= 0.0) {
            return Err(invalid(
                "se requiere soil.z_root >= soil.z_init, 0 < soil.f_rock <= 1 y soil.f_rock_beta >= 0",
            ));
        }
        if !(0.0..=1.0).contains(&self.lod.max_sub_frac) {
            return Err(invalid("se requiere 0 <= lod.max_sub_frac <= 1"));
        }
        if self.planet.h_max <= self.planet.h_min {
            return Err(invalid("planet.h_max debe ser > planet.h_min"));
        }
        if self.climate.t_year < 24.0 {
            return Err(invalid(
                "climate.t_year debe ser >= 24 (anti-aliasing estacional, diseño §9)",
            ));
        }
        if self.soil.w_cap_max < self.soil.w_cap_min || self.soil.w_cap_min <= 0.0 {
            return Err(invalid("se requiere 0 < soil.w_cap_min <= soil.w_cap_max"));
        }
        if self.vegetation.n_veg == 0 {
            return Err(invalid("vegetation.n_veg debe ser >= 1"));
        }
        if self.worldgen.relieve == Relieve::Tectonico {
            self.tectonica.validate(self.planet.n0)?;
        }
        Ok(())
    }
}

impl TectonicaCfg {
    /// Invariantes de la config tectónica (solo se exigen con `worldgen.relieve = "tectonico"`).
    pub fn validate(&self, n0: u32) -> Result<(), ConfigError> {
        let invalid = |m: &str| ConfigError::Invalid(format!("tectonica: {m}"));
        if !(self.dt_myr > 0.0 && self.t_deriva_myr >= 0.0 && self.t_rodaje_myr >= 0.0) {
            return Err(invalid(
                "se requiere dt_myr > 0, t_deriva_myr >= 0 y t_rodaje_myr >= 0",
            ));
        }
        if self.nivel_malla > 4 || (n0 as u64) << self.nivel_malla > 1 << 12 {
            return Err(invalid("nivel_malla demasiado fino (n0 · 2^nivel <= 4096)"));
        }
        if ![self.f_cont, self.eta_col, self.eta_arco, self.f_acr]
            .iter()
            .all(|x| (0.0..=1.0).contains(x))
        {
            return Err(invalid(
                "se requiere que f_cont, eta_col, eta_arco y f_acr estén entre 0 y 1",
            ));
        }
        if !(self.p_arc_oc >= 0.0
            && self.p_arc_cont >= 0.0
            && self.p_se >= 0.0
            && self.tau_del_myr > 0.0
            && self.h_del_km > 0.0
            && self.s_ero_km >= 0.0
            && self.v_ero_cm_anio >= 0.0)
        {
            return Err(invalid(
                "se requiere p_arc_oc, p_arc_cont, p_se, s_ero_km, v_ero_cm_anio >= 0 y tau_del_myr, h_del_km > 0",
            ));
        }
        if !(self.rho_w < self.rho_s
            && self.rho_s < self.rho_c
            && self.rho_c < self.rho_m
            && self.rho_oc < self.rho_m)
        {
            return Err(invalid(
                "se requiere rho_w < rho_s < rho_c < rho_m y rho_oc < rho_m",
            ));
        }
        if !(self.h_oc_km > 0.0
            && self.h_cont_min_km > self.h_oc_km
            && self.h0_km > 0.0
            && self.h_max_km > self.h0_km)
        {
            return Err(invalid(
                "se requiere 0 < h_oc < h_cont_min y 0 < h0 < h_max",
            ));
        }
        if self.n_p0 < 2 || self.n_min < 2 || self.n_cratones > self.n_p0 {
            return Err(invalid(
                "se requiere n_p0 >= 2, n_min >= 2 y n_cratones <= n_p0",
            ));
        }
        if !(self.v_inicial_min_cm_anio > 0.0
            && self.v_inicial_max_cm_anio >= self.v_inicial_min_cm_anio
            && self.v_obj_cm_anio > 0.0)
        {
            return Err(invalid(
                "velocidades iniciales y objetivo deben ser > 0 (min <= max)",
            ));
        }
        if self.detalle_cotas_m.len() != self.detalle_sigma_m.len()
            || self.detalle_cotas_m.is_empty()
            || self.detalle_cotas_m.windows(2).any(|w| w[1] <= w[0])
            || self.detalle_sigma_m.iter().any(|&s| s < 0.0)
        {
            return Err(invalid(
                "detalle_cotas_m (creciente) y detalle_sigma_m (>= 0) deben tener la misma longitud",
            ));
        }
        if !(self.detalle_persistencia > 0.0 && self.detalle_persistencia < 1.0) {
            return Err(invalid("se requiere 0 < detalle_persistencia < 1"));
        }
        if !(self.tau_rift_myr > 0.0 && self.tau_orog_myr > 0.0 && self.t_sat_myr > 0.0) {
            return Err(invalid("se requiere tau_rift, tau_orog y t_sat > 0"));
        }
        Ok(())
    }
}
