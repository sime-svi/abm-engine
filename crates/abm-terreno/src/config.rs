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
    /// Qué mundo da los campos de nacimiento y el clima: el planeta inventado o la Tierra real.
    pub mundo: MundoCfg,
    /// Modo Tierra real (`docs/design/terreno/diseno_modo_tierra.md`, Fase 1).
    pub tierra: TierraCfg,
}

/// Proveedor de mundo (`diseno_modo_tierra.md` §3): el resto del motor recibe los mismos campos
/// venga el mundo de donde venga.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modo {
    /// El planeta inventado: génesis de `worldgen.relieve`, clima del modelo (incrementos 1–4).
    #[default]
    Inventado,
    /// La Tierra real: relieve, hielo y clima medidos (ETOPO 2022, CHELSA, CRU TS).
    Tierra,
}

/// Elección del mundo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MundoCfg {
    pub modo: Modo,
}

/// Modo Tierra real (`diseno_modo_tierra.md` §5). Las fuentes se bajan con
/// `herramientas/tierra/descargar.sh` a `output/referencias/` (ignorado por git).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TierraCfg {
    /// ETOPO 2022 superficie, cada 2' (float32 big-endian, 5 400 × 10 800, filas desde el sur).
    pub etopo_superficie: String,
    /// ETOPO 2022 roca en las franjas polares (latitud < −60° y > 59°), con el mismo formato.
    pub etopo_roca_sur: String,
    pub etopo_roca_norte: String,
    /// Carpeta con los 24 GeoTIFF mensuales de CHELSA V2.1 1981–2010 (`tas` y `pr`).
    pub chelsa: String,
    /// Series mensuales de CRU TS 4.09 (NetCDF-3 comprimido con gzip): temperatura y lluvia.
    pub cru_tmp: String,
    pub cru_pre: String,
    /// Tabla global de atributos del RGI 7.0 (centro y área de cada glaciar; adenda 1 §3).
    pub rgi: String,
    /// HydroSHEDS v1 a 15″: direcciones de flujo, acumulación y máscara (GeoTIFF; adenda 1 §1). La
    /// máscara distingue el fondo de una cuenca cerrada de la salida al mar: en las direcciones de la
    /// versión 1.1 los dos valen 0.
    pub hydrosheds_dir: String,
    pub hydrosheds_acc: String,
    pub hydrosheds_msk: String,
    /// HydroLAKES v1.0, polígonos (shapefile, con su .dbf al lado).
    pub hydrolakes: String,
    /// Los lagos más pequeños que esto caen enteros en la celda de su desagüe; los mayores se
    /// rasterizan cada `lagos_paso_grados`.
    pub lagos_raster_km2: f64,
    pub lagos_paso_grados: f64,
    /// Espesor mínimo (superficie − roca, m) que se cuenta como hielo, y solo con la superficie sobre el
    /// nivel del mar. Por debajo, la diferencia entre las dos versiones de ETOPO es ruido de sus
    /// fuentes (medido: con 0, 1,2 Mkm² de «hielo» sobre el mar del Ártico).
    pub hielo_min_m: f64,
    /// Nivel más fino de la malla al que se cargan los datos; por debajo, reparto conservativo.
    pub nivel_carga: u32,
    /// Años seguidos de cada bloque de años reales remuestreado.
    pub l_bloque: u32,
    /// Tope del cociente de precipitación de un mes frente a su climatología.
    pub r_max: f64,
    /// Solo se remuestrean los años con estaciones en al menos esta fracción de la tierra de CRU TS,
    /// en la temperatura y en la lluvia (adenda 1 §4).
    pub cobertura_min: f64,
    /// Periodo al que se refiere el clima, `[desde, hasta]` (años incluidos). La climatología de
    /// CHELSA es la de 1981–2010; otro periodo la corrige con CRU TS (§5 de la nota).
    pub periodo_base: [i32; 2],
    /// Año real que se usa en todos los años simulados (0 = remuestreo por bloques).
    pub anio_forzado: i32,
}

impl Default for TierraCfg {
    fn default() -> Self {
        Self {
            etopo_superficie: "output/referencias/etopo2022_superficie_2m.f32be".to_string(),
            etopo_roca_sur: "output/referencias/etopo2022_roca_sur_2m.f32be".to_string(),
            etopo_roca_norte: "output/referencias/etopo2022_roca_norte_2m.f32be".to_string(),
            chelsa: "output/referencias/chelsa_mensual".to_string(),
            cru_tmp: "output/referencias/cru_ts/cru_ts4.09.1901.2024.tmp.dat.nc.gz".to_string(),
            cru_pre: "output/referencias/cru_ts/cru_ts4.09.1901.2024.pre.dat.nc.gz".to_string(),
            rgi: "output/referencias/rgi7/RGI2000-v7.0-G-global-attributes.csv".to_string(),
            hydrosheds_dir: "output/referencias/hydrosheds/hyd_glo_dir_15s.tif".to_string(),
            hydrosheds_acc: "output/referencias/hydrosheds/hyd_glo_acc_15s.tif".to_string(),
            hydrosheds_msk: "output/referencias/hydrosheds/hyd_glo_msk_15s.tif".to_string(),
            hydrolakes:
                "output/referencias/hydrosheds/HydroLAKES_polys_v10_shp/HydroLAKES_polys_v10.shp"
                    .to_string(),
            lagos_raster_km2: 50.0,
            lagos_paso_grados: 0.01,
            hielo_min_m: 10.0,
            nivel_carga: 3,
            l_bloque: 10,
            r_max: 5.0,
            cobertura_min: 0.9,
            periodo_base: [1981, 2010],
            anio_forzado: 0,
        }
    }
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
    pub h_cont_min_km: f64,
    // --- Isostasia con litosfera (nota_isostasia_litosfera.md: la regla R4 del estudio de
    // isostasia continental, con los parámetros de su §0 y el sedimento contado como corteza) ---
    /// Densidad de la astenosfera (kg/m³; Lamb et al. 2020).
    pub rho_ast: f64,
    /// Densidad de la corteza (con su sedimento) hasta `z_t` km de profundidad y por debajo
    /// (kg/m³; Christensen y Mooney 1995).
    pub rho_c_sup: f64,
    pub rho_c_inf: f64,
    pub z_t_km: f64,
    /// Contraste neto del manto litosférico con la astenosfera (kg/m³) y el del manto arcaico, más
    /// empobrecido (edad tectonotérmica de más de `t_arc`).
    pub drho_ma: f64,
    pub drho_ma_arcaico: f64,
    pub t_arc_myr: f64,
    /// Grosor de la litosfera según la edad tectonotérmica `t`: la recta de Artemieva (2006),
    /// `L_lin = l0 + l_pend·t`, con recuperación tras el evento:
    /// `L = L_min + (L_lin − L_min)·(1 − e^(−t/τ_L))` (km, Myr).
    pub l0_lit_km: f64,
    pub l_pend_km_myr: f64,
    pub l_min_km: f64,
    pub tau_l_myr: f64,
    /// Constante de referencia de la altura (m): la fija el nivel del mar real en el ajuste.
    pub h0_lit_m: f64,
    /// Delaminación mínima en un paso para reiniciar la edad tectonotérmica (km).
    pub eps_del_km: f64,
    /// Hundimiento térmico de los márgenes estirados (nota de hundimiento de los márgenes; McKenzie
    /// 1978, primer término): `S = E_0·(β/π)·sin(π/β)·(1 − e^(−t/τ))`, con agua encima (km, Myr).
    pub e0_mck_km: f64,
    pub tau_mck_myr: f64,
    // --- Puntos calientes (nota de puntos calientes §5) ---
    /// Puntos calientes vivos (0 los apaga) y su vida (Myr).
    pub n_hs: u32,
    pub t_hs_myr: f64,
    /// Myr entre dos grandes erupciones en todo el planeta (λ_LIP = 1/esto); la fracción de
    /// nacimientos con gran erupción sale de ahí: `f_cabeza = λ_LIP·T_hs/N_hs`.
    pub lip_cada_myr: f64,
    /// Área de una gran erupción, log-uniforme entre estos (Mkm²), grosor que añade en el océano y
    /// en un continente (km) y su duración (Myr).
    pub a_lip_min_mkm2: f64,
    pub a_lip_max_mkm2: f64,
    pub h_lip_km: f64,
    pub h_cfb_km: f64,
    pub dt_lip_myr: f64,
    /// Caudal de cada traza (km³/Myr) y radio de la zona de la traza (km).
    pub q_traza_km3_myr: f64,
    pub r_traza_km: f64,
    /// Altura y anchura de los abombamientos, uniformes en sus rangos (km).
    pub h_sw_min_km: f64,
    pub h_sw_max_km: f64,
    pub w_sw_min_km: f64,
    pub w_sw_max_km: f64,
    // --- Succión de las losas (nota de succión de las losas §5) ---
    /// Distancia de la fosa a su losa profunda, hacia la placa de arriba (km), alcance de la succión
    /// (km) y fracción del peso de la losa profunda que llega a las placas como tracción (0 la apaga).
    pub d_off_km: f64,
    pub w_suc_km: f64,
    pub eps_suc: f64,
    /// Velocidad de hundimiento de las losas en el manto inferior (cm/año) y profundidad que les
    /// queda hasta dejar de tirar (km): `τ_hund = prof/v_hund`.
    pub v_hund_cm_anio: f64,
    pub prof_hund_km: f64,
    /// Edades tectonotérmicas de la corteza continental al empezar la deriva: la distribución de la
    /// Tierra de hoy con las edades de más de 500 Myr, menos 500 (bordes de las clases, Myr, y área
    /// de cada clase). Las más viejas, en el interior de cada continente.
    pub edad_tt_ini_bordes_myr: Vec<f64>,
    pub edad_tt_ini_frac: Vec<f64>,
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
    /// continente (adenda v7 §6.5): depende del grosor `h` de la que entra,
    /// `η(h) = η_min + (1 − η_min)·clamp((h − h_1)/(h_2 − h_1), 0, 1)` ([`TectonicaCfg::eta_choque`]):
    /// se hunde sobre todo el margen adelgazado (de menos de `h_1`) y la corteza de grosor normal (de
    /// más de `h_2`) se queda entera. `η_min` se calibra.
    pub eta_min: f64,
    pub h1_choque_km: f64,
    pub h2_choque_km: f64,
    /// La misma fracción en un choque arco–continente (Luzón–Taiwán).
    pub eta_arco: f64,
    /// Delaminación: por encima de `h_del` la base de la corteza pasa a eclogita y se desprende con
    /// tiempo característico `tau_del` (calibrado).
    pub h_del_km: f64,
    pub tau_del_myr: f64,
    /// Flujo lateral de la corteza engrosada (adenda v6 §6.6b): por encima de `h_flujo` la corteza
    /// inferior se desparrama hacia los lados con difusividad `D_0·(c/c_ref)³` (km²/Myr), con
    /// `c = h_c − h_flujo` el espesor del canal débil (Clark y Royden 2000). `D_0` se calibra; `c_ref`
    /// es una escala de normalización.
    pub h_flujo_km: f64,
    pub d0_flujo_km2_myr: f64,
    pub c_ref_km: f64,
    pub w_min_orog_km: f64,
    /// Rifts (adenda v8 §6.10): antes de que nazca el océano, la corteza de los bordes se estira,
    /// conservando su volumen, hasta una rampa de anchura `W_m` (`w_margen_km`) a cada lado, de su
    /// espesor de partida a `h_rot` junto al eje; entonces se rompe y empieza el fondo oceánico.
    pub w_margen_km: f64,
    pub h_rot_km: f64,
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
    // --- Bordes de placa (nota_bordes_placa.md §5) ---
    /// Ángulo (grados) entre la velocidad relativa y el borde suavizado por debajo del cual un tramo
    /// es transformante (medido: separa las clases de Bird 2003).
    pub theta_tr_grados: f64,
    /// Radio del suavizado del borde (km): pertenencia, normal y tipo de tramo; también la distancia
    /// máxima para recolocar un diente (calibrado contra la longitud de los bordes de Bird).
    pub l_borde_km: f64,
    // --- Ciclo de vida de las fosas (nota_ciclo_fosas.md v2) ---
    /// Convergencia acumulada para que nazca una fosa en un tramo difuso (km; modelos numéricos:
    /// 100–150 km en una zona de fractura, Gurnis et al. 2004; ~200 km, Li y Gurnis 2023).
    pub x_ini_km: f64,
    /// Tiempo seguido sin convergencia tras el que muere una fosa (Myr; supuesto).
    pub tau_muerte_myr: f64,
    /// Nacimiento espontáneo: probabilidad por km de tramo candidato y Myr, con fondo de 100 Myr
    /// (1/(km·Myr); se calibra con la edad del fondo oceánico, Seton et al. 2020).
    pub lambda_esp: f64,
    /// Longitud de losa con la que el tirón es pleno (km; losa que llega a los 660 km con 45–60°).
    pub l_losa_km: f64,
    /// Edad mínima del fondo para un nacimiento espontáneo en una transformante (`a_min` de la nota
    /// de fosas, Myr; supuesto físico: solo el fondo viejo y denso se hunde solo; rango 100–180).
    pub a_min_myr: f64,
    /// Velocidad media con la que una fosa activa se alarga desde su extremo al invadir una
    /// transformante o un margen pasivo (`v_inv` de la nota de fosas v4 §6.4, km/Myr). Resume
    /// también la fase lenta inicial (Gibraltar, ~20 Myr). Calibrado con la vida de los márgenes
    /// pasivos (Bradley 2008) y la edad del fondo.
    pub v_inv_km_myr: f64,
    /// Edad mínima del fondo que una fosa puede invadir (`a_inv`, Myr; supuesto: el fondo que se
    /// hunde bajo las Antillas Menores es del Cretácico; rango 40–100).
    pub a_inv_myr: f64,
    /// Tiempo entre el inicio de un choque y la rotura de la losa, con losa joven y con losa de
    /// 100 Myr o más (`t_rot`, nota de fosas v4 §6.6, Myr; modelos de van Hunen y Allen 2011).
    pub t_rot_joven_myr: f64,
    pub t_rot_viejo_myr: f64,
    // --- Eventos de placas (§5.6) ---
    pub n_p0: u32,
    pub n_cratones: u32,
    pub n_min: u32,
    pub a_min_frac: f64,
    /// Rifts por bloques (nota de rifts v2 §6): cada bloque continental se rompe en el paso con
    /// probabilidad `1 − exp(−λ_A·A_k·min(1, t_k/τ_ais)·Δt)` y el trazado sigue su sutura más larga.
    /// `λ_A` es la probabilidad de ruptura por Mkm² de continente y por Myr (1/(Mkm²·Myr)) de un
    /// bloque que lleva unido más de `τ_ais`; `t_k`, el tiempo que lleva el bloque sin romperse ni
    /// soldarse a otro continente. Se calibran con el número de placas grandes (~13, Bird 2003) y
    /// la duración de los supercontinentes.
    pub lambda_a_rift: f64,
    /// Tiempo que un bloque tiene que llevar unido para alcanzar su probabilidad plena de ruptura
    /// (`τ_ais`, Myr; nota de rifts v2 §6.1): el calor del manto se acumula bajo él.
    pub tau_ais_myr: f64,
    /// Área mínima de continente de cada trozo tras la ruptura (Mkm²; Madagascar, ~0,6, es de los
    /// trozos más pequeños separados por un rift).
    pub a_trozo_min_mkm2: f64,
    /// Longitud mínima de una sutura para guiar la ruptura (km; 3·`L_borde`).
    pub l_sut_min_km: f64,
    /// Muerte de placas (nota de demografía de placas v2). A2: una microplaca (de menos de
    /// `A_micro`, Mkm²) que lleva `τ_micro` Myr sin fosa activa propia pasa a la vecina con la que
    /// comparte más borde.
    pub a_micro_mkm2: f64,
    pub tau_micro_myr: f64,
    /// Rama D (subducción de dorsales): cuando la franja de una placa entre una de sus dorsales y una
    /// fosa activa donde se hunde es más estrecha que `d_dc` (km), la dorsal muere, la franja pasa a
    /// la placa del otro lado de la dorsal y la fosa de enfrente se apaga. Con `d_dc` = 0, no actúa.
    pub d_dc_km: f64,
    /// La fosa está delante de la dorsal si su tramo más cercano cae a menos de `θ_dc` (grados) de la
    /// dirección de apertura hacia dentro de la placa: la franja entre las dos, no el costado de un
    /// punto triple (versión aceptada por el responsable el 2026-11-08).
    pub theta_dc_grados: f64,
    pub v_rift_cm_anio: f64,
    /// Soldadura tras un choque agotado (nota de succión de las losas §6.4; antes, las suturas del
    /// documento principal): un choque entre continentes, ya rota su losa, que converge a menos de
    /// `v_sold` (cm/año) durante `τ_sold` (Myr) funde sus dos placas.
    pub v_sutura_cm_anio: f64,
    pub t_sutura_myr: f64,
    /// Erosión de la génesis (adenda de balance v5 §6.7): `E = E_1·(z/1 km)^n_ero` km/Myr en la
    /// tierra emergida, con `z` la altura sobre el mar (sustituto del relieve local, que la malla no
    /// resuelve). `E_1`, la erosión a 1 km (calibrada con la relajación de un orógeno y la
    /// exportación al mar), y `n_ero` (rango 1,5–3: la erosión crece mucho más que proporcional con
    /// el relieve; Montgomery y Brandon 2002).
    pub e1_km_myr: f64,
    pub n_ero: f64,
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
            h_cont_min_km: 20.0,
            rho_ast: 3255.0,
            rho_c_sup: 2750.0,
            rho_c_inf: 3000.0,
            z_t_km: 15.0,
            drho_ma: 25.0,
            drho_ma_arcaico: 15.0,
            t_arc_myr: 2500.0,
            l0_lit_km: 93.6,
            l_pend_km_myr: 0.04,
            l_min_km: 50.0,
            tau_l_myr: 100.0,
            h0_lit_m: 3076.9,
            eps_del_km: 1.0,
            e0_mck_km: 3.2,
            tau_mck_myr: 62.8,
            n_hs: 12,
            t_hs_myr: 150.0,
            lip_cada_myr: 23.0,
            a_lip_min_mkm2: 0.2,
            a_lip_max_mkm2: 2.0,
            h_lip_km: 15.0,
            h_cfb_km: 3.0,
            dt_lip_myr: 2.0,
            q_traza_km3_myr: 1.25e5,
            r_traza_km: 100.0,
            h_sw_min_km: 0.5,
            h_sw_max_km: 1.2,
            w_sw_min_km: 1000.0,
            w_sw_max_km: 1500.0,
            d_off_km: 500.0,
            w_suc_km: 1500.0,
            eps_suc: 0.0,
            v_hund_cm_anio: 1.2,
            prof_hund_km: 2200.0,
            edad_tt_ini_bordes_myr: vec![
                0.0, 100.0, 200.0, 300.0, 400.0, 500.0, 750.0, 1000.0, 1250.0, 1500.0, 1750.0,
                2000.0, 2250.0, 2500.0, 2750.0, 3000.0, 3250.0,
            ],
            edad_tt_ini_frac: vec![
                0.040, 0.068, 0.068, 0.068, 0.068, 0.068, 0.068, 0.068, 0.068, 0.068, 0.068, 0.058,
                0.058, 0.069, 0.069, 0.028,
            ],
            d_r_m: 2500.0,
            c_hs: 350.0,
            d_inf_m: 5310.0,
            a_pl_m: 2373.0,
            tau_pl_myr: 31.0,
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
            f_acr: 0.2,
            eta_min: 0.5,
            h1_choque_km: 20.0,
            h2_choque_km: 35.0,
            eta_arco: 0.9,
            h_del_km: 50.0,
            tau_del_myr: 25.0,
            h_flujo_km: 50.0,
            d0_flujo_km2_myr: 400.0,
            c_ref_km: 10.0,
            w_min_orog_km: 150.0,
            w_margen_km: 150.0,
            h_rot_km: 10.0,
            f_sp: 1.0,
            f_rp: 0.2,
            t_sat_myr: 80.0,
            c_col: 90_000.0,
            kappa_cont: 3.0,
            v_obj_cm_anio: 4.39,
            recalibrar_myr: 50.0,
            v_inicial_min_cm_anio: 1.0,
            v_inicial_max_cm_anio: 10.0,
            theta_tr_grados: 20.0,
            l_borde_km: 150.0,
            x_ini_km: 150.0,
            tau_muerte_myr: 10.0,
            lambda_esp: 4.0e-6,
            l_losa_km: 800.0,
            a_min_myr: 140.0,
            v_inv_km_myr: 25.0,
            a_inv_myr: 60.0,
            t_rot_joven_myr: 10.0,
            t_rot_viejo_myr: 25.0,
            n_p0: 12,
            n_cratones: 8,
            n_min: 8,
            a_min_frac: 0.005,
            lambda_a_rift: 4.0e-4,
            tau_ais_myr: 100.0,
            a_trozo_min_mkm2: 0.5,
            l_sut_min_km: 450.0,
            a_micro_mkm2: 0.5,
            tau_micro_myr: 10.0,
            d_dc_km: 300.0,
            theta_dc_grados: 45.0,
            v_rift_cm_anio: 2.0,
            v_sutura_cm_anio: 0.5,
            t_sutura_myr: 20.0,
            e1_km_myr: 0.02,
            n_ero: 2.5,
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

    /// Fracción de la corteza continental de grosor `h_km` que se queda arriba al entrar en un
    /// choque (adenda v7 §6.5): `η_min` por debajo de `h_1`, 1 por encima de `h_2` y lineal entre
    /// los dos.
    pub fn eta_choque(&self, h_km: f64) -> f64 {
        let x = if self.h2_choque_km > self.h1_choque_km {
            ((h_km - self.h1_choque_km) / (self.h2_choque_km - self.h1_choque_km)).clamp(0.0, 1.0)
        } else if h_km > self.h1_choque_km {
            1.0
        } else {
            0.0
        };
        self.eta_min + (1.0 - self.eta_min) * x
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
        if self.mundo.modo == Modo::Tierra {
            let t = &self.tierra;
            if t.nivel_carga < self.lod.l_min || t.nivel_carga > 6 {
                return Err(invalid("tierra: se requiere lod.l_min <= nivel_carga <= 6"));
            }
            if t.l_bloque == 0
                || !(t.r_max >= 1.0 && t.r_max.is_finite())
                || t.hielo_min_m < 0.0
                || !(0.0..=1.0).contains(&t.cobertura_min)
            {
                return Err(invalid(
                    "tierra: se requiere l_bloque >= 1, r_max >= 1, hielo_min_m >= 0 y 0 <= cobertura_min <= 1",
                ));
            }
            if t.periodo_base[1] < t.periodo_base[0] {
                return Err(invalid(
                    "tierra: periodo_base debe ser [desde, hasta] con desde <= hasta",
                ));
            }
            // El calendario es el real: 365 días, con los meses de 31, 28, 31…
            if self.climate.t_year != 365.0 {
                return Err(invalid("tierra: se requiere climate.t_year = 365"));
            }
            if self.planet.sea_level != 0.0 || (self.planet.r_km - 6371.0).abs() > 1e-9 {
                return Err(invalid(
                    "tierra: se requiere planet.sea_level = 0 y planet.r_km = 6371 (los datos son de la Tierra actual)",
                ));
            }
        } else if self.worldgen.relieve == Relieve::Tectonico {
            self.tectonica.validate(self.planet.n0)?;
        }
        Ok(())
    }

    /// `true` en el modo Tierra real.
    pub fn es_tierra(&self) -> bool {
        self.mundo.modo == Modo::Tierra
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
        if ![self.f_cont, self.eta_min, self.eta_arco, self.f_acr]
            .iter()
            .all(|x| (0.0..=1.0).contains(x))
        {
            return Err(invalid(
                "se requiere que f_cont, eta_min, eta_arco y f_acr estén entre 0 y 1",
            ));
        }
        if !(self.h1_choque_km >= 0.0 && self.h2_choque_km >= self.h1_choque_km) {
            return Err(invalid("se requiere 0 <= h1_choque_km <= h2_choque_km"));
        }
        if !(self.p_arc_oc >= 0.0
            && self.p_arc_cont >= 0.0
            && self.p_se >= 0.0
            && self.tau_del_myr > 0.0
            && self.h_del_km > 0.0
            && self.s_ero_km >= 0.0
            && self.v_ero_cm_anio >= 0.0
            && self.h_flujo_km > 0.0
            && self.d0_flujo_km2_myr >= 0.0
            && self.c_ref_km > 0.0
            && self.l_borde_km > 0.0
            && self.theta_tr_grados > 0.0
            && self.theta_tr_grados < 90.0
            && self.x_ini_km > 0.0
            && self.tau_muerte_myr > 0.0
            && self.lambda_esp >= 0.0
            && self.l_losa_km > 0.0
            && self.a_min_myr >= 0.0
            && self.v_inv_km_myr >= 0.0
            && self.a_inv_myr >= 0.0
            && self.t_rot_joven_myr >= 0.0
            && self.t_rot_viejo_myr >= self.t_rot_joven_myr)
        {
            return Err(invalid(
                "se requiere p_arc_oc, p_arc_cont, p_se, s_ero_km, v_ero_cm_anio, d0_flujo_km2_myr >= 0, c_ref_km, tau_del_myr, h_del_km, h_flujo_km, l_borde_km, x_ini_km, tau_muerte_myr, l_losa_km > 0, lambda_esp, v_inv_km_myr, a_inv_myr >= 0, 0 <= t_rot_joven_myr <= t_rot_viejo_myr y 0 < theta_tr_grados < 90",
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
        if !(self.h_oc_km > 0.0 && self.h_cont_min_km > self.h_oc_km && self.h0_km > 0.0) {
            return Err(invalid("se requiere 0 < h_oc < h_cont_min y 0 < h0"));
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
        if !(self.e1_km_myr > 0.0 && self.n_ero > 0.0 && self.t_sat_myr > 0.0) {
            return Err(invalid("se requiere e1_km_myr, n_ero y t_sat > 0"));
        }
        if !(self.w_margen_km >= 0.0 && self.h_rot_km > 0.0) {
            return Err(invalid("se requiere w_margen_km >= 0 y h_rot_km > 0"));
        }
        if !(self.a_micro_mkm2 >= 0.0 && self.tau_micro_myr >= 0.0 && self.d_dc_km >= 0.0) {
            return Err(invalid(
                "se requiere a_micro_mkm2, tau_micro_myr y d_dc_km >= 0",
            ));
        }
        if !(self.rho_ast > self.rho_w
            && self.rho_c_sup < self.rho_ast
            && self.rho_c_inf < self.rho_ast
            && self.z_t_km >= 0.0
            && self.drho_ma >= 0.0
            && self.drho_ma_arcaico >= 0.0
            && self.l_min_km > 0.0
            && self.tau_l_myr > 0.0
            && self.eps_del_km >= 0.0
            && self.e0_mck_km >= 0.0
            && self.tau_mck_myr > 0.0)
        {
            return Err(invalid(
                "isostasia con litosfera: se requiere rho_c_sup, rho_c_inf < rho_ast, rho_w < rho_ast, z_t, drho_ma, eps_del, e0_mck >= 0 y l_min, tau_l, tau_mck > 0",
            ));
        }
        if !(self.t_hs_myr > 0.0
            && self.lip_cada_myr >= 0.0
            && self.a_lip_min_mkm2 > 0.0
            && self.a_lip_max_mkm2 >= self.a_lip_min_mkm2
            && self.h_lip_km >= 0.0
            && self.h_cfb_km >= 0.0
            && self.dt_lip_myr > 0.0
            && self.q_traza_km3_myr >= 0.0
            && self.r_traza_km > 0.0
            && self.h_sw_max_km >= self.h_sw_min_km
            && self.h_sw_min_km >= 0.0
            && self.w_sw_max_km >= self.w_sw_min_km
            && self.w_sw_min_km > 0.0
            && self.d_off_km >= 0.0
            && self.w_suc_km > 0.0
            && self.eps_suc >= 0.0
            && self.v_hund_cm_anio > 0.0
            && self.prof_hund_km > 0.0)
        {
            return Err(invalid(
                "puntos calientes y succión: se requiere t_hs, dt_lip, r_traza, w_sw_min, w_suc, v_hund, prof_hund > 0, rangos ordenados y el resto >= 0",
            ));
        }
        if self.edad_tt_ini_bordes_myr.len() != self.edad_tt_ini_frac.len() + 1
            || self.edad_tt_ini_bordes_myr.windows(2).any(|w| w[1] <= w[0])
            || self.edad_tt_ini_frac.iter().any(|&f| f < 0.0)
            || self.edad_tt_ini_frac.iter().sum::<f64>() <= 0.0
        {
            return Err(invalid(
                "edad_tt_ini_bordes_myr (creciente) debe tener una entrada más que edad_tt_ini_frac (>= 0, con suma > 0)",
            ));
        }
        if !(self.lambda_a_rift >= 0.0
            && self.tau_ais_myr >= 0.0
            && self.a_trozo_min_mkm2 > 0.0
            && self.l_sut_min_km >= 0.0)
        {
            return Err(invalid(
                "se requiere lambda_a_rift, tau_ais_myr >= 0, a_trozo_min_mkm2 > 0 y l_sut_min_km >= 0",
            ));
        }
        Ok(())
    }
}
