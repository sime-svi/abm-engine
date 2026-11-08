//! Generador de estáticos (incremento 2 §3): cada campo estático de una celda es **función pura**
//! de su dirección `(cara, nivel, i, j)`, la semilla y la config. No depende de la malla vecina ni
//! del orden en que se generan las celdas, así que subdividir o colapsar una celda recalcula su
//! valor "verdadero" a ese nivel, venga de donde venga (independencia del camino).
//!
//! - Elevación: ruido 3D con **normalización congelada** (min/max del nivel base, calculados una
//!   vez) y **una octava más por nivel** (con normalización fija, añadir octavas solo suma detalle).
//! - Pendiente: diferencias centrales de la propia función de altura, a un paso de celda.
//! - Suelo: mismas fórmulas del incremento 1, con esa pendiente.
//! - Recurso: RNG **por contador** (hash de semilla + clave del ancestro de nivel base).

use std::f64::consts::FRAC_PI_2;
use std::sync::Arc;

use crate::config::TerrenoConfig;
use crate::geometry::{V3, cell_area, center_dir, east_north, face_n, key, lat_lon};
use crate::noise::Perlin3;

/// Geometría de una celda (función pura de su dirección).
#[derive(Debug, Clone, Copy)]
pub struct CellGeom {
    pub dir: V3,
    pub lat: f64,
    pub lon: f64,
    pub area: f64,
    pub east: V3,
    pub north: V3,
}

/// Campos estáticos de una celda (función pura de su dirección).
#[derive(Debug, Clone, Copy)]
pub struct CellStatics {
    pub h: f64,
    pub slope: f64,
    pub aspect: f64,
    pub grad_e: f64,
    pub grad_n: f64,
    /// `∂h/∂este` del relieve que ve el aire, con el mar plano al nivel del mar (nota de sombra
    /// pluviométrica §6): las mismas diferencias centrales con `max(h, nivel_mar)`.
    pub grad_air_e: f64,
    pub w_cap: f64,
    pub k_perm: f64,
    pub fert: f64,
    pub resource: f64,
}

/// Mezclador SplitMix64 (bien distribuido, sin estado): base del RNG por contador.
#[inline]
pub(crate) fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Número uniforme en `[0, 1)` que depende solo de `(semilla, clave, flujo)`: un RNG "por contador".
/// Mismo valor se pida cuando se pida y en el orden que sea (CLAUDE §6: sigue derivando de la semilla).
#[inline]
pub fn hash01(seed: u64, key: u64, stream: u64) -> f64 {
    let h = splitmix64(seed ^ splitmix64(key ^ splitmix64(stream)));
    (h >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0) // 2^-53
}

/// Generador de estáticos de un mundo. Se construye una vez por `(semilla, config)`.
#[derive(Clone)]
pub struct StaticGen {
    perlin: Perlin3,
    tex: Perlin3,
    res_seed: u64,
    n0: u32,
    l_min: u32,
    r_planet: f64,
    base_freq: f64,
    persistence: f64,
    lacunarity: f64,
    oct_base: u32,
    oct_per_level: u32,
    /// Normalización fija: suma de amplitudes de las octavas del nivel base.
    norm0: f64,
    /// Normalización congelada de la elevación (min y rango de `raw` en el nivel base).
    raw_min: f64,
    raw_span: f64,
    sea: f64,
    h_min: f64,
    h_max: f64,
    land_exp: f64,
    w_cap_min: f64,
    w_cap_max: f64,
    resource_density: f64,
    /// Relieve de referencia de la lluvia orográfica: nivel `L_oro` (el de lado más cercano a
    /// `d_oro`) y paso angular `d_oro / R` de sus diferencias centrales. Paso 0 = desactivado.
    oro_level: u32,
    oro_delta: f64,
    /// Relieve tectónico (incremento 4): si está, la altura sale de él y del ruido de detalle.
    tect: Option<Arc<crate::tectonica::Tectonica>>,
    detalle: Detalle,
    /// Modo Tierra (`diseno_modo_tierra.md`): si está, la altura y el hielo son los de la Tierra.
    tierra: Option<Arc<crate::tierra::DatosTierra>>,
}

/// Ruido de detalle del relieve tectónico (§6.8 y decisión del responsable): empieza por debajo de
/// la malla tectónica (primera onda = 2 lados de celda tectónica) y suma una octava por cada nivel
/// del LOD más fino que ella, con amplitud `σ(z)·p^k`. Solo entran las ondas de al menos dos lados de
/// la celda que se evalúa: así la altura de una celda es la media de su zona, como en ETOPO.
#[derive(Clone)]
struct Detalle {
    ruido: Perlin3,
    /// Frecuencia (sobre la esfera unidad) de la primera octava.
    f0: f64,
    /// Nivel de la malla tectónica.
    nivel_tect: u32,
    persistencia: f64,
    /// Desviación típica de una octava del ruido (para normalizar a σ = 1).
    norma: f64,
}

impl Detalle {
    fn new(cfg: &TerrenoConfig) -> Self {
        let tc = &cfg.tectonica;
        let lado_km = FRAC_PI_2 * cfg.planet.r_km / face_n(cfg.planet.n0, tc.nivel_malla) as f64;
        let f0 = cfg.planet.r_km / (2.0 * lado_km);
        let ruido = Perlin3::new(cfg.run.seed ^ 0xDE7A);
        // Desviación típica de una octava, medida en una espiral de Fibonacci (determinista).
        let n = 8192;
        let (mut s, mut s2) = (0.0, 0.0);
        for i in 0..n {
            let z = 1.0 - (2.0 * i as f64 + 1.0) / n as f64;
            let r = (1.0 - z * z).max(0.0).sqrt();
            let phi = i as f64 * 2.399_963_229_728_653;
            let v = ruido.get(r * phi.cos() * f0, r * phi.sin() * f0, z * f0);
            s += v;
            s2 += v * v;
        }
        let m = s / n as f64;
        let norma = (s2 / n as f64 - m * m).max(1e-12).sqrt();
        Self {
            ruido,
            f0,
            nivel_tect: tc.nivel_malla,
            persistencia: tc.detalle_persistencia,
            norma,
        }
    }

    /// Ruido de detalle (σ = 1 en la primera octava) en `d` con el detalle del nivel `l`.
    #[inline]
    fn en(&self, d: V3, l: u32) -> f64 {
        if l < self.nivel_tect {
            return 0.0;
        }
        let (mut amp, mut f, mut suma) = (1.0, self.f0, 0.0);
        for _ in 0..=(l - self.nivel_tect) {
            suma += amp * self.ruido.get(d[0] * f, d[1] * f, d[2] * f);
            amp *= self.persistencia;
            f *= 2.0;
        }
        suma / self.norma
    }
}

/// Punto a un ángulo `delta` de `dir` en la dirección tangente `u` (`sign` = ±1): los extremos de
/// las diferencias centrales del relieve de referencia orográfico.
pub fn oro_offset(dir: V3, u: V3, sign: f64, delta: f64) -> V3 {
    let (cd, sd) = (delta.cos(), delta.sin());
    [
        cd * dir[0] + sign * sd * u[0],
        cd * dir[1] + sign * sd * u[1],
        cd * dir[2] + sign * sd * u[2],
    ]
}

/// Nivel del árbol cuyo lado de celda (en el centro de una cara) es el más cercano a `d_km`.
pub fn level_closest_to(n0: u32, r_km: f64, d_km: f64) -> u32 {
    let side = |l: u32| std::f64::consts::FRAC_PI_2 * r_km / face_n(n0, l) as f64;
    (0..=20u32)
        .min_by(|&a, &b| (side(a) - d_km).abs().total_cmp(&(side(b) - d_km).abs()))
        .unwrap_or(0)
}

impl StaticGen {
    /// Construye el generador y **congela** la normalización de la elevación con los centros de la
    /// malla uniforme del nivel base `l_min` (en `L = l_min` la elevación es la del incremento 1).
    pub fn new(cfg: &TerrenoConfig) -> Self {
        let wg = &cfg.worldgen;
        let mut g = Self {
            perlin: Perlin3::new(cfg.run.seed),
            tex: Perlin3::new(cfg.run.seed ^ 0x5015),
            res_seed: cfg.run.seed ^ 0xACE1,
            n0: cfg.planet.n0,
            l_min: cfg.lod.l_min,
            r_planet: cfg.planet.r_km * 1000.0,
            base_freq: wg.base_freq,
            persistence: wg.persistence,
            lacunarity: wg.lacunarity,
            oct_base: wg.noise_octaves,
            oct_per_level: wg.octaves_per_level,
            norm0: Perlin3::fbm_norm(wg.noise_octaves, wg.persistence),
            raw_min: 0.0,
            raw_span: 1.0,
            sea: cfg.planet.sea_level,
            h_min: cfg.planet.h_min,
            h_max: cfg.planet.h_max,
            land_exp: wg.land_exp,
            w_cap_min: cfg.soil.w_cap_min,
            w_cap_max: cfg.soil.w_cap_max,
            resource_density: wg.resource_density,
            oro_level: level_closest_to(cfg.planet.n0, cfg.planet.r_km, cfg.climate.d_oro_km),
            oro_delta: cfg.climate.d_oro_km / cfg.planet.r_km,
            tect: None,
            detalle: Detalle::new(cfg),
            tierra: None,
        };
        let n = face_n(g.n0, g.l_min);
        let (mut mn, mut mx) = (f64::INFINITY, f64::NEG_INFINITY);
        for c in 0..6u8 {
            for j in 0..n {
                for i in 0..n {
                    let r = g.raw(center_dir(c, i, j, n), g.l_min);
                    mn = mn.min(r);
                    mx = mx.max(r);
                }
            }
        }
        g.raw_min = mn;
        g.raw_span = (mx - mn).max(1e-9);
        g
    }

    /// Generador con el relieve tectónico (incremento 4): la altura de nacimiento es la base
    /// tectónica interpolada (con su fosa) más el ruido de detalle según el contexto (§6.8).
    pub fn con_tectonica(cfg: &TerrenoConfig, t: Arc<crate::tectonica::Tectonica>) -> Self {
        let mut g = Self::new(cfg);
        g.tect = Some(t);
        g
    }

    /// Estado tectónico final, si el relieve es tectónico.
    pub fn tectonica(&self) -> Option<&Arc<crate::tectonica::Tectonica>> {
        self.tect.as_ref()
    }

    /// Generador del modo Tierra: la altura de una celda es la media de ETOPO 2022 en ella, a su
    /// nivel (o a la del nivel de carga, si la celda es más fina). La textura del suelo y el recurso
    /// siguen siendo los del modelo (provisional hasta SoilGrids).
    pub fn con_tierra(cfg: &TerrenoConfig, t: Arc<crate::tierra::DatosTierra>) -> Self {
        let mut g = Self::new(cfg);
        g.tierra = Some(t);
        g
    }

    /// Datos de la Tierra, en el modo Tierra.
    pub fn tierra(&self) -> Option<&Arc<crate::tierra::DatosTierra>> {
        self.tierra.as_ref()
    }

    /// Fracción con hielo de la celda `(c, l, i, j)`: la de la Tierra, o 0 en el planeta inventado.
    pub fn hielo(&self, c: u8, l: u32, i: u32, j: u32) -> f64 {
        self.tierra.as_ref().map_or(0.0, |t| t.hielo(c, l, i, j))
    }

    /// Fracción de lago de la celda `(c, l, i, j)`: la de HydroLAKES, o 0 en el planeta inventado.
    pub fn lago(&self, c: u8, l: u32, i: u32, j: u32) -> f64 {
        self.tierra.as_ref().map_or(0.0, |t| t.lago(c, l, i, j))
    }

    /// Altura de la base tectónica en `d` (con la fosa y sin el ruido de detalle); `None` sin
    /// tectónica. Sirve para distinguir las cuencas de la tectónica de los hoyos del ruido (G6).
    pub fn h_base_at(&self, d: V3) -> Option<f64> {
        self.tect.as_ref().map(|t| t.base_en(d).z_m)
    }

    /// Levantamiento tectónico (m/año) en `d` para el prerrodaje (§6.9a); 0 sin tectónica.
    pub fn levantamiento_m_anio(&self, d: V3) -> f64 {
        self.tect
            .as_ref()
            .map_or(0.0, |t| t.levantamiento_m_anio(d))
    }

    /// Nivel base uniforme.
    pub fn l_min(&self) -> u32 {
        self.l_min
    }

    /// Octavas de ruido al nivel `l`: las del nivel base + `octaves_per_level` por nivel extra.
    #[inline]
    pub fn octaves(&self, l: u32) -> u32 {
        self.oct_base + self.oct_per_level * l.saturating_sub(self.l_min)
    }

    /// Ruido de elevación sin hipsometría en la dirección `d` al nivel `l` (normalización fija).
    #[inline]
    pub fn raw(&self, d: V3, l: u32) -> f64 {
        let f = self.base_freq;
        let sum = self.perlin.fbm_sum(
            d[0] * f,
            d[1] * f,
            d[2] * f,
            self.octaves(l),
            self.persistence,
            self.lacunarity,
        );
        if self.norm0 > 0.0 {
            sum / self.norm0
        } else {
            0.0
        }
    }

    /// Curva hipsométrica del incremento 1 sobre la fracción normalizada (recortada a `[0, 1]`).
    #[inline]
    pub fn h_from_raw(&self, raw: f64) -> f64 {
        let frac = ((raw - self.raw_min) / self.raw_span).clamp(0.0, 1.0);
        let sea_frac = ((self.sea - self.h_min) / (self.h_max - self.h_min)).clamp(0.0, 1.0);
        if frac < sea_frac {
            // Océano: profundidad lineal.
            self.h_min + frac * (self.h_max - self.h_min)
        } else {
            // Tierra: curva hipsométrica (potencia) => la mayoría cerca del nivel del mar.
            let lf = (frac - sea_frac) / (1.0 - sea_frac).max(1e-9);
            self.sea + lf.powf(self.land_exp) * (self.h_max - self.sea)
        }
    }

    /// Elevación (m) en la dirección `d` con el detalle del nivel `l`.
    #[inline]
    pub fn h_at(&self, d: V3, l: u32) -> f64 {
        if let Some(t) = &self.tierra {
            return t.h_en(d, l);
        }
        match &self.tect {
            Some(t) => {
                let b = t.base_en(d);
                b.z_m + b.sigma_m * self.detalle.en(d, l)
            }
            None => self.h_from_raw(self.raw(d, l)),
        }
    }

    /// Geometría de la celda `(c, l, i, j)`.
    pub fn geom(&self, c: u8, l: u32, i: u32, j: u32) -> CellGeom {
        let n = face_n(self.n0, l);
        let dir = center_dir(c, i, j, n);
        let (lat, lon) = lat_lon(dir);
        let (east, north) = east_north(dir);
        CellGeom {
            dir,
            lat,
            lon,
            area: cell_area(c, i, j, n, self.r_planet),
            east,
            north,
        }
    }

    /// Estáticos de la celda `(c, l, i, j)` (con su geometría ya calculada).
    pub fn statics(&self, c: u8, l: u32, i: u32, j: u32, g: &CellGeom) -> CellStatics {
        let h = self.h_at(g.dir, l);

        // Pendiente: diferencias centrales de h(·, l) a un paso de celda sobre la esfera (§3.2).
        let delta = FRAC_PI_2 / face_n(self.n0, l) as f64;
        let (cd, sd) = (delta.cos(), delta.sin());
        let at = |u: V3, s: f64| -> f64 {
            self.h_at(
                [
                    cd * g.dir[0] + s * sd * u[0],
                    cd * g.dir[1] + s * sd * u[1],
                    cd * g.dir[2] + s * sd * u[2],
                ],
                l,
            )
        };
        let denom = 2.0 * delta * self.r_planet;
        let (h_este, h_oeste) = (at(g.east, 1.0), at(g.east, -1.0));
        let gx = (h_este - h_oeste) / denom;
        let gx_air = (h_este.max(self.sea) - h_oeste.max(self.sea)) / denom;
        let gy = (at(g.north, 1.0) - at(g.north, -1.0)) / denom;
        let slope = (gx * gx + gy * gy).sqrt().atan();
        let aspect = gy.atan2(gx);

        // Textura de suelo y derivados (mismas fórmulas del incremento 1).
        let d = g.dir;
        let ns = 0.5 * (self.tex.get(d[0] * 6.0, d[1] * 6.0, d[2] * 6.0) + 1.0); // [0,1]
        // Laderas empinadas => más arenoso; llanuras => más limo/arcilla.
        let sand = (0.35 + 0.4 * ns + 0.4 * (slope.tan()).min(1.0)).clamp(0.1, 0.9);
        let fine = 1.0 - sand;
        let silt = fine * 0.6;
        let clay = fine * 0.4;
        let w_cap = self.w_cap_min + (self.w_cap_max - self.w_cap_min) * (clay * 0.7 + silt * 0.5);
        let k_perm = sand.clamp(0.05, 1.0);
        let fert = (silt * 0.5 + clay * 0.6).clamp(0.05, 0.95);

        // Recurso: densidad del ancestro de nivel base, sorteada por contador; nula en el mar.
        let resource = if h < self.sea {
            0.0
        } else {
            let shift = l.saturating_sub(self.l_min);
            let base = key(c, self.l_min, i >> shift, j >> shift);
            if hash01(self.res_seed, base, 0) < self.resource_density {
                100.0 * hash01(self.res_seed, base, 1)
            } else {
                0.0
            }
        };

        CellStatics {
            h,
            slope,
            aspect,
            grad_e: gx,
            grad_n: gy,
            grad_air_e: gx_air,
            w_cap,
            k_perm,
            fert,
            resource,
        }
    }

    /// Paso angular (rad) del relieve de referencia orográfico (0 = desactivado).
    pub fn oro_delta(&self) -> f64 {
        self.oro_delta
    }

    /// Nivel de referencia `L_oro` del relieve orográfico (el de lado más cercano a `d_oro`).
    pub fn oro_level(&self) -> u32 {
        self.oro_level
    }

    /// `true` si la lluvia de una celda de este nivel usa el relieve de referencia: es tan fina
    /// como `d_oro` o más. Las más gruesas usan su propia pendiente, que ya está suavizada a una
    /// escala mayor (nota de lluvia orográfica §3).
    pub fn uses_oro_reference(&self, level: u32) -> bool {
        self.oro_delta > 0.0 && level >= self.oro_level
    }

    /// Parte pura del gradiente orográfico de una celda (nota de lluvia orográfica): diferencias
    /// centrales de la elevación de nacimiento **al nivel de referencia `L_oro`**, con paso `d_oro`,
    /// en su base este/norte. Es función pura de la posición: en un punto del planeta da lo mismo
    /// que la zona esté en malla fina o gruesa. `(0, 0)` si la celda no usa la referencia.
    pub fn oro_gradient0(&self, g: &CellGeom, level: u32) -> (f64, f64) {
        let (ge, gn, _) = self.oro_gradient0_air(g, level);
        (ge, gn)
    }

    /// Como [`StaticGen::oro_gradient0`], y además la componente este del relieve que ve el aire,
    /// con el mar plano (`max(h, nivel_mar)`): `(este, norte, este_aire)`.
    pub fn oro_gradient0_air(&self, g: &CellGeom, level: u32) -> (f64, f64, f64) {
        if !self.uses_oro_reference(level) {
            return (0.0, 0.0, 0.0);
        }
        self.oro_gradient0_full_at(g.dir, g.east, g.north)
    }

    /// Lo mismo en un punto cualquiera (`dir`, con su base `east`/`north`), sin mirar el nivel.
    pub fn oro_gradient0_at(&self, dir: V3, east: V3, north: V3) -> (f64, f64) {
        let (ge, gn, _) = self.oro_gradient0_full_at(dir, east, north);
        (ge, gn)
    }

    /// Gradiente puro del relieve de referencia en un punto y su componente este con el mar plano.
    fn oro_gradient0_full_at(&self, dir: V3, east: V3, north: V3) -> (f64, f64, f64) {
        if self.oro_delta <= 0.0 {
            return (0.0, 0.0, 0.0);
        }
        let at = |u: V3, s: f64| -> f64 {
            self.h_at(oro_offset(dir, u, s, self.oro_delta), self.oro_level)
        };
        let denom = 2.0 * self.oro_delta * self.r_planet;
        let (h_este, h_oeste) = (at(east, 1.0), at(east, -1.0));
        (
            (h_este - h_oeste) / denom,
            (at(north, 1.0) - at(north, -1.0)) / denom,
            (h_este.max(self.sea) - h_oeste.max(self.sea)) / denom,
        )
    }

    /// Nivel del mar (m).
    pub fn sea_level(&self) -> f64 {
        self.sea
    }

    /// `true` si una elevación queda bajo el nivel del mar (clasificación base, §4).
    #[inline]
    pub fn is_ocean_h(&self, h: f64) -> bool {
        h < self.sea
    }

    /// `true` si la celda de clave `clave` y altura `h` es mar: en el planeta inventado, si está bajo
    /// el nivel del mar; en el modo Tierra, si la mitad de su área es mar conectado con el océano
    /// mundial (adenda 1 §1.4: las depresiones interiores son tierra).
    #[inline]
    pub fn es_mar(&self, clave: u64, h: f64) -> bool {
        match &self.tierra {
            Some(t) => {
                let (lv, k) = t.celda_de_clave(clave);
                t.mar_frac[lv][k] >= 0.5
            }
            None => self.is_ocean_h(h),
        }
    }

    /// `true` si la celda de clave `clave` mezcla cuencas cerradas y abiertas (modo Tierra): el LOD la
    /// afina para que ninguna cuenca cerrada rebose por una celda gruesa.
    #[inline]
    pub fn mezcla_cuencas(&self, clave: u64) -> bool {
        self.tierra.as_ref().is_some_and(|t| {
            let (lv, k) = t.celda_de_clave(clave);
            t.mezcla[lv][k]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_por_contador_es_uniforme_y_estable() {
        let mut suma = 0.0;
        for k in 0..10_000u64 {
            let u = hash01(42, k, 0);
            assert!((0.0..1.0).contains(&u));
            assert_eq!(u, hash01(42, k, 0));
            suma += u;
        }
        let media = suma / 10_000.0;
        assert!((media - 0.5).abs() < 0.02, "media sesgada: {media}");
        assert_ne!(
            hash01(42, 7, 0),
            hash01(42, 7, 1),
            "flujos distintos deben diferir"
        );
    }

    #[test]
    fn la_elevacion_no_depende_de_la_historia() {
        // Función pura: el mismo generador da la misma altura en la misma celda, se pida cuando se
        // pida; y dos generadores con la misma config también (normalización congelada).
        let cfg = TerrenoConfig::default();
        let a = StaticGen::new(&cfg);
        let b = StaticGen::new(&cfg);
        for &(c, l, i, j) in &[(0u8, 0u32, 3u32, 5u32), (4, 2, 100, 37), (5, 3, 383, 0)] {
            let ga = a.geom(c, l, i, j);
            let sa = a.statics(c, l, i, j, &ga);
            let sb = b.statics(c, l, i, j, &b.geom(c, l, i, j));
            assert_eq!(sa.h.to_bits(), sb.h.to_bits());
            assert_eq!(sa.w_cap.to_bits(), sb.w_cap.to_bits());
            assert_eq!(sa.resource.to_bits(), sb.resource.to_bits());
            assert_eq!(
                a.statics(c, l, i, j, &ga).slope.to_bits(),
                sa.slope.to_bits()
            );
        }
    }

    #[test]
    fn mas_niveles_mas_octavas() {
        let cfg = TerrenoConfig::default();
        let g = StaticGen::new(&cfg);
        assert_eq!(g.octaves(cfg.lod.l_min), cfg.worldgen.noise_octaves);
        assert_eq!(g.octaves(cfg.lod.l_min + 2), cfg.worldgen.noise_octaves + 2);
    }
}
