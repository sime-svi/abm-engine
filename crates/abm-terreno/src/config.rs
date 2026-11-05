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
    pub persistence: PersistenceCfg,
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
        }
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
            // Nota de sombra pluviométrica §2: con 0,5 (la visión madre) el factor orográfico
            // quedaba en 0,99–1,01 y no había sombra de lluvia. La nota propone 40 y pide calibrar
            // por objetivos: con el relieve de este planeta, la media en las laderas de barlovento
            // de las cordilleras de más de 2 000 m llega a 2–3 con ~150 (nota §6.2).
            k_oro: 150.0,
            m_oro_min: 0.2,
            m_oro_max: 3.0,
            moisture_length_km: 1500.0,
            seas_precip_amp: 0.0,
            d_oro_km: 25.0,
            h_v_m: 2500.0,
            l_recarga_km: 1000.0,
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
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_string(),
            source,
        })?;
        let cfg: TerrenoConfig = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_string(),
            source,
        })?;
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
        Ok(())
    }
}
