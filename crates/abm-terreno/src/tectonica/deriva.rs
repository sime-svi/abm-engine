//! Deriva de placas (§6.1–§6.6 y §9, G1–G2, del diseño del relieve tectónico).
//!
//! Cada placa guarda su corteza en su propio sistema de referencia, en una copia de la malla
//! tectónica (`mapa`: celda local → muestra). Cada paso:
//! 1. velocidades por equilibrio de fuerzas (tirón de la placa, empuje de dorsal, rozamiento basal y
//!    resistencia de colisión), marco sin rotación neta y calibración de `c_b`;
//! 2. las placas giran (cuaterniones);
//! 3. el mundo se «rasteriza»: para cada celda del mundo se mira qué placas tienen corteza en ella;
//! 4. huecos → corteza oceánica nueva; solapes → subducción o choque;
//! 5. los mapas de las placas se ponen al día con el mundo (sin remuestrear lo que no cambia);
//! 6. arcos, engrosamiento de los choques, erosión tectónica, envejecimiento y sedimento;
//! 7. eventos de placas: suturas, rifts, absorción de placas pequeñas y saltos de dorsal;
//! 8. bordes para las fuerzas del paso siguiente, métricas y eventos.
//!
//! Todo el azar sale de un generador por contador `H(semilla, paso, id de placa, propósito, índice)`
//! y todas las sumas van en un orden fijo: dos deribas con la misma semilla dan lo mismo bit a bit.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::historia::{CuentasCorteza, EventoTectonico, MetricaMyr};
use super::isostasia;
use super::malla::{Malla, Mat3, NADA, Quat, aplicar, aplicar_t, velocidad};
use crate::config::TectonicaCfg;
use crate::geometry::{V3, cross, dot, east_north, lat_lon, normalize};
use crate::noise::Perlin3;
use crate::statics::{hash01, splitmix64};

/// Máximo de placas que se apuntan en una celda del mundo (más es rarísimo: se ignoran).
const MAXC: usize = 4;

// Propósitos del generador por contador.
const P_SEMILLA: u64 = 1;
const P_EJE: u64 = 2;
const P_VEL: u64 = 3;
const P_RIFT: u64 = 5;
const P_ORIENT: u64 = 6;
const P_CRATON: u64 = 7;
const P_SALTO: u64 = 8;

/// 1 cm/año en km/Myr.
pub const CM_ANIO: f64 = 10.0;

/// Número uniforme en `[0, 1)` del generador por contador.
fn azar(seed: u64, paso: u64, id: u64, prop: u64, i: u64) -> f64 {
    hash01(
        seed ^ splitmix64(prop.wrapping_mul(0x9E37_79B9_7F4A_7C15)),
        splitmix64(paso ^ splitmix64(id)),
        i,
    )
}

/// Id determinista de una placa hija: `H(id de la madre, paso, índice)`.
fn id_hija(madre: u64, paso: u64, i: u64) -> u64 {
    splitmix64(madre ^ splitmix64(paso.wrapping_add(0x5151) ^ splitmix64(i)))
}

/// Dirección uniforme sobre la esfera a partir de dos números de `[0, 1)`.
fn dir_uniforme(u1: f64, u2: f64) -> V3 {
    let z = 2.0 * u1 - 1.0;
    let r = (1.0 - z * z).max(0.0).sqrt();
    let f = 2.0 * std::f64::consts::PI * u2;
    [r * f.cos(), r * f.sin(), z]
}

#[inline]
fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
fn esc(a: V3, s: f64) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}

/// Una columna de corteza (§3, «propiedades de cada celda de corteza»).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Corteza {
    pub cont: bool,
    /// Espesor de la corteza cristalina (km).
    pub h: f64,
    /// Espesor del sedimento de los márgenes (km), con su densidad (corrección C2).
    pub sed: f64,
    /// Edad de la corteza oceánica (Myr).
    pub edad_oc: f64,
    /// Tiempo desde el último engrosamiento tectónico (Myr).
    pub edad_orog: f64,
    /// Ritmo de engrosamiento del paso (km/Myr): arcos y choques.
    pub dh_dt: f64,
}

impl Corteza {
    fn oceanica(h_oc: f64) -> Self {
        Self {
            cont: false,
            h: h_oc,
            sed: 0.0,
            edad_oc: 0.0,
            edad_orog: 0.0,
            dh_dt: 0.0,
        }
    }

    /// Espesor equivalente en roca (km): el sedimento cuenta con su masa.
    #[inline]
    pub fn h_eq(&self, rho_s: f64, rho_c: f64) -> f64 {
        self.h + self.sed * rho_s / rho_c
    }
}

/// Una placa rígida con su corteza en su propio sistema de referencia.
#[derive(Clone)]
pub struct Placa {
    pub id: u64,
    pub viva: bool,
    pub q: Quat,
    /// Rotación local → mundo (de `q`).
    pub rot: Mat3,
    /// Velocidad angular (rad/Myr) en el sistema del mundo.
    pub omega: V3,
    /// Velocidad extra solo para el movimiento siguiente (apertura de un rift).
    pub empuje: V3,
    /// Celda local → muestra (o [`NADA`]).
    pub mapa: Vec<u32>,
    /// Muestra → celda local.
    pub celda: Vec<u32>,
    pub cort: Vec<Corteza>,
    /// Muestra → celda del mundo que contiene su centro (se recalcula tras mover).
    pub mundo: Vec<u32>,
    /// Casquete que contiene la placa, en su sistema: centro y coseno del radio.
    cap_c: V3,
    cap_cos: f64,
    /// Marca de trabajo por celda local (se limpia en cada paso).
    marca: Vec<u8>,
    pub nacida_myr: f64,
}

impl Placa {
    fn nueva(id: u64, num: usize, t: f64) -> Self {
        Self {
            id,
            viva: true,
            q: Quat::IDENTIDAD,
            rot: Quat::IDENTIDAD.matriz(),
            omega: [0.0; 3],
            empuje: [0.0; 3],
            mapa: vec![NADA; num],
            celda: Vec::new(),
            cort: Vec::new(),
            mundo: Vec::new(),
            cap_c: [1.0, 0.0, 0.0],
            cap_cos: -2.0,
            marca: vec![0; num],
            nacida_myr: t,
        }
    }

    /// Placa vacía con la misma orientación y velocidad que `madre` (hijas de rift).
    fn hermana(madre: &Placa, id: u64, t: f64) -> Self {
        let mut p = Placa::nueva(id, madre.mapa.len(), t);
        p.q = madre.q;
        p.rot = madre.rot;
        p.omega = madre.omega;
        p
    }

    fn insertar(&mut self, l: u32, c: Corteza, mundo: u32) {
        debug_assert_eq!(self.mapa[l as usize], NADA);
        self.mapa[l as usize] = self.cort.len() as u32;
        self.celda.push(l);
        self.cort.push(c);
        self.mundo.push(mundo);
    }

    /// Quita la muestra de la celda local `l` y la devuelve.
    fn quitar(&mut self, l: u32) -> Option<Corteza> {
        let s = self.mapa[l as usize];
        if s == NADA {
            return None;
        }
        let s = s as usize;
        let c = self.cort.swap_remove(s);
        self.celda.swap_remove(s);
        self.mundo.swap_remove(s);
        self.mapa[l as usize] = NADA;
        if s < self.cort.len() {
            let movida = self.celda[s];
            self.mapa[movida as usize] = s as u32;
        }
        Some(c)
    }

    #[inline]
    pub fn get(&self, l: u32) -> Option<&Corteza> {
        let s = self.mapa[l as usize];
        (s != NADA).then(|| &self.cort[s as usize])
    }

    #[inline]
    fn get_mut(&mut self, l: u32) -> Option<&mut Corteza> {
        let s = self.mapa[l as usize];
        if s == NADA {
            None
        } else {
            Some(&mut self.cort[s as usize])
        }
    }

    /// Recalcula el casquete que contiene todas sus muestras (con un margen de dos celdas).
    fn actualizar_cap(&mut self, malla: &Malla) {
        if self.celda.is_empty() {
            self.cap_c = [1.0, 0.0, 0.0];
            self.cap_cos = 2.0;
            return;
        }
        let mut c = [0.0; 3];
        for &l in &self.celda {
            c = add(c, malla.dir[l as usize]);
        }
        let n = dot(c, c).sqrt();
        if n < 1e-9 {
            self.cap_c = [1.0, 0.0, 0.0];
            self.cap_cos = -2.0;
            return;
        }
        let c = esc(c, 1.0 / n);
        let mut cos_min = 1.0f64;
        for &l in &self.celda {
            cos_min = cos_min.min(dot(c, malla.dir[l as usize]));
        }
        let margen = 2.5 * malla.lado_km / malla.r_km;
        let ang = cos_min.clamp(-1.0, 1.0).acos() + margen;
        self.cap_c = c;
        self.cap_cos = if ang >= std::f64::consts::PI {
            -2.0
        } else {
            ang.cos()
        };
    }

    fn morir(&mut self) {
        self.viva = false;
        self.mapa = Vec::new();
        self.marca = Vec::new();
        self.celda = Vec::new();
        self.cort = Vec::new();
        self.mundo = Vec::new();
    }
}

/// Tipo de un borde de placa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoBorde {
    Divergente,
    Subduccion,
    Colision,
    Transformante,
}

impl TipoBorde {
    pub fn codigo(self) -> u8 {
        match self {
            TipoBorde::Divergente => 1,
            TipoBorde::Subduccion => 2,
            TipoBorde::Colision => 3,
            TipoBorde::Transformante => 4,
        }
    }
}

/// Una celda del mundo en el borde de su placa `p`, frente a la placa `q`.
#[derive(Debug, Clone, Copy)]
pub struct Borde {
    pub k: u32,
    pub p: u32,
    pub q: u32,
    /// Celda de `q` al otro lado.
    pub k_q: u32,
    /// Normal tangente de `p` hacia `q`.
    pub normal: V3,
    pub largo_km: f64,
    /// Convergencia (km/Myr): `> 0` si `p` y `q` se acercan.
    pub v_n: f64,
    pub tipo: TipoBorde,
    /// En una subducción: `p` es la placa que se hunde.
    pub p_subduce: bool,
}

/// Historia de un par de placas (ids `a < b`).
#[derive(Debug, Clone)]
struct Par {
    a: u64,
    b: u64,
    /// Última subducción entre las dos: (id de la que se hundía, Myr).
    subduccion: Option<(u64, f64)>,
    /// Choque en curso: (id de la de abajo, id de la de arriba).
    choque: Option<(u64, u64)>,
    /// Subducción entre cortezas oceánicas en curso: id de la que se hunde (polaridad de todo el
    /// tramo, decidida en el primer contacto por la edad media de cada lado).
    polaridad_oo: Option<u64>,
    /// Último Myr con contacto convergente entre las dos.
    contacto_myr: f64,
    choque_visto_myr: f64,
    sutura_myr: f64,
    habia_subduccion: bool,
    habia_colision: bool,
}

/// Una celda del mundo que una placa perdió en un solape.
#[derive(Debug, Clone, Copy)]
struct Perdida {
    k: u32,
    p: u32,
    l: u32,
    choque: bool,
}

/// Campo de distancias por la superficie (Dijkstra) con la fuente más cercana de cada celda.
#[derive(Default)]
struct Campo {
    dist: Vec<f64>,
    fuente: Vec<u32>,
    /// Celdas alcanzadas, en orden creciente de distancia.
    orden: Vec<u32>,
}

#[derive(Clone, Copy, PartialEq)]
struct Item {
    d: f64,
    k: u32,
}
impl Eq for Item {}
impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        o.d.total_cmp(&self.d).then_with(|| o.k.cmp(&self.k))
    }
}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Campo {
    fn nuevo(n: usize) -> Self {
        Self {
            dist: vec![f64::INFINITY; n],
            fuente: vec![NADA; n],
            orden: Vec::new(),
        }
    }

    fn limpiar(&mut self) {
        for &k in &self.orden {
            self.dist[k as usize] = f64::INFINITY;
            self.fuente[k as usize] = NADA;
        }
        self.orden.clear();
    }

    /// Dijkstra desde `fuentes` (celda, distancia inicial, etiqueta) por las celdas `permitida`,
    /// hasta `max_d` km.
    fn calcular(
        &mut self,
        malla: &Malla,
        fuentes: &[(u32, f64, u32)],
        permitida: impl Fn(usize, usize) -> bool,
        max_d: f64,
    ) {
        self.limpiar();
        let mut heap = BinaryHeap::new();
        for &(k, d0, et) in fuentes {
            let ku = k as usize;
            if d0 < self.dist[ku] {
                self.dist[ku] = d0;
                self.fuente[ku] = et;
                heap.push(Item { d: d0, k });
            }
        }
        // Cada celda alcanzada sale del montón una sola vez con su distancia final (los empujes
        // solo ocurren con distancia estrictamente menor): `orden` las tiene todas, en orden.
        while let Some(Item { d, k }) = heap.pop() {
            let ku = k as usize;
            if d > self.dist[ku] {
                continue;
            }
            self.orden.push(k);
            for (j, &v) in malla.vec[ku].iter().enumerate() {
                if v == NADA || !permitida(ku, v as usize) {
                    continue;
                }
                let nd = d + malla.dvec[ku][j];
                if nd <= max_d && nd < self.dist[v as usize] {
                    self.dist[v as usize] = nd;
                    self.fuente[v as usize] = self.fuente[ku];
                    heap.push(Item { d: nd, k: v });
                }
            }
        }
    }
}

/// Estado de la deriva de placas.
pub struct Deriva {
    pub cfg: TectonicaCfg,
    pub seed: u64,
    pub malla: Malla,
    pub placas: Vec<Placa>,
    /// Dueño de cada celda del mundo (índice en `placas`).
    pub dueno: Vec<u32>,
    /// Celda local del dueño que representa la celda del mundo.
    pub rep: Vec<u32>,
    pub paso: u64,
    /// Tiempo actual (Myr; negativo, sube hasta 0).
    pub t_myr: f64,
    t0_myr: f64,
    pub c_b: f64,
    prox_calibracion: f64,
    pub bordes: Vec<Borde>,
    pares: Vec<Par>,
    pub eventos: Vec<EventoTectonico>,
    pub metricas: Vec<MetricaMyr>,
    pub cuentas: CuentasCorteza,
    /// Nivel del mar del paso respecto al dato (m): la costa del sedimento.
    pub zeta_m: f64,
    super_formado: bool,
    /// Correlaciones de Forsyth y Uyeda acumuladas tras el rodaje: (suma, nº de pasos).
    pub fu_fosa: (f64, u64),
    pub fu_cont: (f64, u64),
    pub v_rms_acum: (f64, u64),
    pub ultima_v_rms: f64,
    /// Área de corteza oceánica nueva del último paso (km²).
    pub area_nueva_km2: f64,
    // Trabajo (se reserva una vez).
    cov: Vec<[(u32, u32); MAXC]>,
    ncov: Vec<u8>,
    hueco: Vec<bool>,
    perdidas: Vec<Perdida>,
    campo: Campo,
    campo2: Campo,
    ruido_def: [Perlin3; 3],
}

impl Deriva {
    /// Estado inicial aleatorio (§6.1): `N_p0` placas de Voronoi deformadas, `N_cratones` cratones
    /// con área total `f_cont`, y fondo oceánico de edad aleatoria en [0, 150] Myr.
    pub fn nueva(cfg: &TectonicaCfg, seed: u64, malla: Malla) -> Self {
        let n = cfg.n_p0 as usize;
        let semillas: Vec<V3> = (0..n as u64)
            .map(|i| {
                dir_uniforme(
                    azar(seed, 0, i, P_SEMILLA, 0),
                    azar(seed, 0, i, P_SEMILLA, 1),
                )
            })
            .collect();
        let ruido = [
            Perlin3::new(seed ^ 0x7EC1),
            Perlin3::new(seed ^ 0x7EC2),
            Perlin3::new(seed ^ 0x7EC3),
        ];
        let deformar = |d: V3, amp: f64, f: f64| -> V3 {
            normalize(add(
                d,
                [
                    amp * ruido[0].fbm(d[0] * f, d[1] * f, d[2] * f, 3, 0.5, 2.0),
                    amp * ruido[1].fbm(d[0] * f, d[1] * f, d[2] * f, 3, 0.5, 2.0),
                    amp * ruido[2].fbm(d[0] * f, d[1] * f, d[2] * f, 3, 0.5, 2.0),
                ],
            ))
        };
        // Placas: celda más cercana a una semilla, con la posición deformada (bordes no rectos).
        let placa_de: Vec<usize> = (0..malla.num)
            .map(|k| {
                let d = deformar(malla.dir[k], 3.0 * cfg.deformacion, 2.0);
                (0..n)
                    .max_by(|&a, &b| {
                        dot(d, semillas[a])
                            .total_cmp(&dot(d, semillas[b]))
                            .then(b.cmp(&a))
                    })
                    .unwrap_or(0)
            })
            .collect();
        // Cratones: centrados en las semillas de `N_cratones` placas, con tamaños distintos.
        let nc = cfg.n_cratones as usize;
        let mut orden: Vec<usize> = (0..n).collect();
        orden.sort_by(|&a, &b| {
            azar(seed, 0, a as u64, P_CRATON, 0).total_cmp(&azar(seed, 0, b as u64, P_CRATON, 0))
        });
        let cratones: Vec<(V3, f64)> = orden[..nc]
            .iter()
            .map(|&i| {
                (
                    semillas[i],
                    0.6 + 0.8 * azar(seed, 0, i as u64, P_CRATON, 1),
                )
            })
            .collect();
        let rf = Perlin3::new(seed ^ 0xC7A7);
        let puntos: Vec<f64> = (0..malla.num)
            .map(|k| {
                if cratones.is_empty() {
                    return f64::NEG_INFINITY;
                }
                let d = deformar(malla.dir[k], 4.0 * cfg.deformacion, 3.0);
                cratones
                    .iter()
                    .map(|&(c, s)| -crate::geometry::angle_between(d, c) / s)
                    .fold(f64::NEG_INFINITY, f64::max)
            })
            .collect();
        let mut por_puntos: Vec<usize> = (0..malla.num).collect();
        por_puntos.sort_by(|&a, &b| puntos[b].total_cmp(&puntos[a]).then(a.cmp(&b)));
        let objetivo = cfg.f_cont * malla.area_total();
        let mut es_cont = vec![false; malla.num];
        let mut acum = 0.0;
        for &k in &por_puntos {
            if acum >= objetivo || cratones.is_empty() {
                break;
            }
            es_cont[k] = true;
            acum += malla.area[k];
        }
        // Ruido suave del espesor continental (σ_H0) y de la edad inicial del fondo.
        let rh = Perlin3::new(seed ^ 0x4E0);
        let std_h = 0.25;
        let corteza = |k: usize| -> Corteza {
            let d = malla.dir[k];
            if es_cont[k] {
                let x = rh.fbm(d[0] * 6.0, d[1] * 6.0, d[2] * 6.0, 3, 0.5, 2.0) / std_h;
                Corteza {
                    cont: true,
                    h: cfg.h0_km + cfg.sigma_h0_km * x.clamp(-3.0, 3.0),
                    sed: 0.0,
                    edad_oc: 0.0,
                    edad_orog: 1000.0,
                    dh_dt: 0.0,
                }
            } else {
                let u = 0.5 + 0.5 * rf.fbm(d[0] * 4.0, d[1] * 4.0, d[2] * 4.0, 2, 0.5, 2.0) / 0.35;
                Corteza {
                    edad_oc: 150.0 * u.clamp(0.0, 1.0),
                    ..Corteza::oceanica(cfg.h_oc_km)
                }
            }
        };
        let cortezas: Vec<Corteza> = (0..malla.num).map(corteza).collect();
        let t0 = -cfg.t_deriva_myr;
        let ids: Vec<u64> = (0..n as u64)
            .map(|i| splitmix64(seed ^ splitmix64(i + 1)))
            .collect();
        Self::desde(cfg, seed, malla, t0, &ids, |k| (placa_de[k], cortezas[k]))
    }

    /// Estado inicial dado: cada celda del mundo pertenece a la placa `asignar(k).0` (índice en
    /// `ids`) con la corteza `asignar(k).1`. Para las pruebas de comportamiento (§10).
    pub fn desde(
        cfg: &TectonicaCfg,
        seed: u64,
        malla: Malla,
        t0_myr: f64,
        ids: &[u64],
        asignar: impl Fn(usize) -> (usize, Corteza),
    ) -> Self {
        let num = malla.num;
        let mut placas: Vec<Placa> = ids
            .iter()
            .map(|&id| Placa::nueva(id, num, t0_myr))
            .collect();
        let mut dueno = vec![NADA; num];
        let mut rep = vec![NADA; num];
        for k in 0..num {
            let (p, c) = asignar(k);
            placas[p].insertar(k as u32, c, k as u32);
            dueno[k] = p as u32;
            rep[k] = k as u32;
        }
        for p in &mut placas {
            p.actualizar_cap(&malla);
        }
        let ruido_def = [
            Perlin3::new(seed ^ 0x51F1),
            Perlin3::new(seed ^ 0x51F2),
            Perlin3::new(seed ^ 0x51F3),
        ];
        let mut d = Self {
            cfg: cfg.clone(),
            seed,
            cov: vec![[(NADA, NADA); MAXC]; num],
            ncov: vec![0; num],
            hueco: vec![false; num],
            perdidas: Vec::new(),
            campo: Campo::nuevo(num),
            campo2: Campo::nuevo(num),
            malla,
            placas,
            dueno,
            rep,
            paso: 0,
            t_myr: t0_myr,
            t0_myr,
            c_b: 1.0,
            prox_calibracion: f64::NEG_INFINITY,
            bordes: Vec::new(),
            pares: Vec::new(),
            eventos: Vec::new(),
            metricas: Vec::new(),
            cuentas: CuentasCorteza::default(),
            zeta_m: 0.0,
            super_formado: false,
            fu_fosa: (0.0, 0),
            fu_cont: (0.0, 0),
            v_rms_acum: (0.0, 0),
            ultima_v_rms: 0.0,
            area_nueva_km2: 0.0,
            ruido_def,
        };
        d.cuentas.volumen_inicial_km3 = d.volumen_continental();
        d.cuentas.inventario_inicial_km3 = d.inventario_continental();
        d.zeta_m = d.nivel_del_mar();
        d.super_formado = d.indice_supercontinente() >= 0.7;
        d
    }

    /// Corre la deriva entera (`T_deriva / Δt` pasos).
    pub fn correr(&mut self) {
        let pasos = (self.cfg.t_deriva_myr / self.cfg.dt_myr).round() as u64;
        for _ in 0..pasos {
            self.dar_paso();
        }
        self.cuentas.volumen_final_km3 = self.volumen_continental();
        self.cuentas.inventario_final_km3 = self.inventario_continental();
        self.cuentas.remuestreo_km3 = self.cuentas.inventario_final_km3
            - self.cuentas.inventario_inicial_km3
            - self.cuentas.explicado();
    }

    /// Un paso de la deriva (§9, G2 a–g).
    pub fn dar_paso(&mut self) {
        let dt = self.cfg.dt_myr;
        self.velocidades();
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.q = Quat::de_rotacion(add(p.omega, p.empuje), dt)
                .componer(p.q)
                .normalizado();
            p.rot = p.q.matriz();
            p.empuje = [0.0; 3];
        }
        self.recalcular_mundo();
        self.rasterizar();
        self.resolver_solapes();
        self.rellenar_huecos();
        let choques = self.reconstruir_mapas();
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.cort.iter_mut().for_each(|c| c.dh_dt = 0.0);
        }
        self.clasificar_bordes();
        self.crecer_arcos();
        self.engrosar_choques(&choques);
        let erosion = self.erosion_y_envejecimiento();
        self.zeta_m = self.nivel_del_mar();
        self.sedimento(&erosion);
        let mut cambio = self.suturas();
        cambio |= self.rifts();
        cambio |= self.absorciones();
        cambio |= self.saltos_de_dorsal();
        if cambio {
            for p in self.placas.iter_mut().filter(|p| p.viva) {
                p.actualizar_cap(&self.malla);
            }
            self.recalcular_mundo();
            self.clasificar_bordes();
        }
        self.paso += 1;
        self.t_myr = self.t0_myr + self.paso as f64 * dt;
        self.metricas_del_paso();
    }

    /// Arranca con velocidades dadas en lugar del empujón al azar (pruebas de comportamiento): fija
    /// las velocidades angulares (rad/Myr) y clasifica los bordes, de modo que desde el primer paso
    /// mandan las fuerzas.
    pub fn arrancar_con(&mut self, omegas: &[V3]) {
        for (p, &w) in self.placas.iter_mut().zip(omegas) {
            p.omega = w;
        }
        self.paso = 1;
        self.recalcular_mundo();
        self.clasificar_bordes();
    }

    /// Longitud (km) del frente de colisión entre dos placas (por su índice).
    pub fn frente_colision(&self, a: usize, b: usize) -> f64 {
        self.bordes
            .iter()
            .filter(|x| {
                x.tipo == TipoBorde::Colision
                    && ((x.p as usize, x.q as usize) == (a, b)
                        || (x.p as usize, x.q as usize) == (b, a))
            })
            .map(|x| 0.5 * x.largo_km)
            .sum()
    }

    /// Solo la cinemática de un paso, con las velocidades que ya tengan las placas: girar, rasterizar,
    /// resolver huecos y solapes, y poner al día los mapas (para las pruebas de la malla).
    pub fn paso_cinematico(&mut self) {
        let dt = self.cfg.dt_myr;
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.q = Quat::de_rotacion(add(p.omega, p.empuje), dt)
                .componer(p.q)
                .normalizado();
            p.rot = p.q.matriz();
        }
        self.recalcular_mundo();
        self.rasterizar();
        self.resolver_solapes();
        self.rellenar_huecos();
        let _ = self.reconstruir_mapas();
        self.paso += 1;
        self.t_myr = self.t0_myr + self.paso as f64 * dt;
    }

    // --- Consultas ----------------------------------------------------------------------------

    /// Corteza de la celda del mundo `k` (la de su dueño).
    #[inline]
    pub fn corteza(&self, k: usize) -> Option<&Corteza> {
        let p = self.dueno[k];
        if p == NADA {
            return None;
        }
        self.placas[p as usize].get(self.rep[k])
    }

    /// Inventario de corteza continental de las placas (km³): todas sus muestras, también las que el
    /// mundo no ve este paso; con el sedimento por su masa.
    pub fn inventario_continental(&self) -> f64 {
        let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
        self.placas
            .iter()
            .filter(|p| p.viva)
            .map(|p| {
                p.cort
                    .iter()
                    .zip(&p.celda)
                    .filter(|(c, _)| c.cont)
                    .map(|(c, &l)| c.h_eq(rs, rc) * self.malla.area[l as usize])
                    .sum::<f64>()
            })
            .sum()
    }

    /// Volumen de corteza continental (km³) en el mundo, con el sedimento por su masa.
    pub fn volumen_continental(&self) -> f64 {
        let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
        (0..self.malla.num)
            .filter_map(|k| {
                self.corteza(k)
                    .filter(|c| c.cont)
                    .map(|c| c.h_eq(rs, rc) * self.malla.area[k])
            })
            .sum()
    }

    /// Altura isostática (m, respecto al dato) de cada celda del mundo.
    pub fn alturas(&self) -> Vec<f64> {
        (0..self.malla.num)
            .map(|k| {
                self.corteza(k)
                    .map_or(-self.cfg.d_r_m, |c| isostasia::altura(&self.cfg, c))
            })
            .collect()
    }

    /// Nivel del mar (m, respecto al dato) con el volumen de agua de `gel_m`.
    pub fn nivel_del_mar(&self) -> f64 {
        let z = self.alturas();
        let vol = self.cfg.gel_m / 1000.0 * self.malla.area_total();
        isostasia::nivel_del_mar(&z, &self.malla.area, vol)
    }

    /// Fracción de la corteza continental en la mayor masa conectada.
    pub fn indice_supercontinente(&self) -> f64 {
        let num = self.malla.num;
        let cont: Vec<bool> = (0..num)
            .map(|k| self.corteza(k).is_some_and(|c| c.cont))
            .collect();
        let total: f64 = (0..num)
            .filter(|&k| cont[k])
            .map(|k| self.malla.area[k])
            .sum();
        if total <= 0.0 {
            return 0.0;
        }
        let mut visto = vec![false; num];
        let mut mayor = 0.0f64;
        let mut pila = Vec::new();
        for s in 0..num {
            if !cont[s] || visto[s] {
                continue;
            }
            let mut a = 0.0;
            visto[s] = true;
            pila.push(s as u32);
            while let Some(k) = pila.pop() {
                a += self.malla.area[k as usize];
                for &v in &self.malla.vec[k as usize] {
                    if v != NADA && cont[v as usize] && !visto[v as usize] {
                        visto[v as usize] = true;
                        pila.push(v);
                    }
                }
            }
            mayor = mayor.max(a);
        }
        mayor / total
    }

    fn par_idx(&mut self, x: u64, y: u64) -> usize {
        let (a, b) = if x < y { (x, y) } else { (y, x) };
        match self.pares.binary_search_by(|p| (p.a, p.b).cmp(&(a, b))) {
            Ok(i) => i,
            Err(i) => {
                self.pares.insert(
                    i,
                    Par {
                        a,
                        b,
                        subduccion: None,
                        choque: None,
                        polaridad_oo: None,
                        contacto_myr: f64::NEG_INFINITY,
                        choque_visto_myr: f64::NEG_INFINITY,
                        sutura_myr: 0.0,
                        habia_subduccion: false,
                        habia_colision: false,
                    },
                );
                i
            }
        }
    }

    // --- a. Velocidades (§6.2) ----------------------------------------------------------------

    fn velocidades(&mut self) {
        let n = self.placas.len();
        let r = self.malla.r_km;
        if self.paso == 0 {
            self.empujon_inicial();
            return;
        }
        // Rozamiento basal: M_p = Σ (1 + κ·[cont]) (I − x xᵀ) A; y sin pesar, para el marco NNR.
        let mut m = vec![[[0.0; 3]; 3]; n];
        let mut m0 = vec![[[0.0; 3]; 3]; n];
        for k in 0..self.malla.num {
            let p = self.dueno[k];
            if p == NADA {
                continue;
            }
            let x = self.malla.dir[k];
            let a = self.malla.area[k];
            let w = 1.0
                + if self.corteza(k).is_some_and(|c| c.cont) {
                    self.cfg.kappa_cont
                } else {
                    0.0
                };
            for i in 0..3 {
                for j in 0..3 {
                    let pr = if i == j { 1.0 } else { 0.0 } - x[i] * x[j];
                    m[p as usize][i][j] += pr * a * w;
                    m0[p as usize][i][j] += pr * a;
                }
            }
        }
        // Fuerzas de borde (par respecto al centro) y acoplamiento implícito de las colisiones.
        let mut t = vec![[0.0; 3]; n];
        let mut acop: Vec<(usize, usize, Mat3)> = Vec::new();
        for b in &self.bordes {
            let x = self.malla.dir[b.k as usize];
            let p = b.p as usize;
            let f = match b.tipo {
                TipoBorde::Subduccion if b.p_subduce => {
                    let edad = self.corteza(b.k as usize).map_or(0.0, |c| c.edad_oc);
                    esc(
                        b.normal,
                        self.cfg.f_sp * (edad / self.cfg.t_sat_myr).sqrt().min(1.0),
                    )
                }
                TipoBorde::Divergente => esc(b.normal, -self.cfg.f_rp),
                TipoBorde::Colision => {
                    // −c_col·c_b·v_n·n̂ con v_n = R (ω_p − ω_q)·(x × n̂): término implícito.
                    let g = cross(x, b.normal);
                    let c = self.cfg.c_col * 0.5 * b.largo_km * r * r;
                    let mut gg = [[0.0; 3]; 3];
                    for i in 0..3 {
                        for j in 0..3 {
                            gg[i][j] = c * g[i] * g[j];
                        }
                    }
                    acop.push((p, b.q as usize, gg));
                    [0.0; 3]
                }
                _ => [0.0; 3],
            };
            let par = esc(cross(x, f), r * b.largo_km);
            t[p] = add(t[p], par);
        }
        // Sistema K ω = T / c_b, con K = R²·M (diagonal por placas) + acoplamientos.
        let vivas: Vec<usize> = (0..n).filter(|&i| self.placas[i].viva).collect();
        let mut pos = vec![usize::MAX; n];
        for (i, &p) in vivas.iter().enumerate() {
            pos[p] = i;
        }
        let dim = 3 * vivas.len();
        let mut kmat = vec![0.0; dim * dim];
        let mut rhs = vec![0.0; dim];
        for (i, &p) in vivas.iter().enumerate() {
            for a in 0..3 {
                for b in 0..3 {
                    kmat[(3 * i + a) * dim + 3 * i + b] += r * r * m[p][a][b];
                }
                rhs[3 * i + a] = t[p][a];
            }
        }
        for (p, q, gg) in &acop {
            let (ip, iq) = (pos[*p], pos[*q]);
            if ip == usize::MAX || iq == usize::MAX {
                continue;
            }
            for a in 0..3 {
                for b in 0..3 {
                    kmat[(3 * ip + a) * dim + 3 * ip + b] += gg[a][b];
                    kmat[(3 * iq + a) * dim + 3 * iq + b] += gg[a][b];
                    kmat[(3 * ip + a) * dim + 3 * iq + b] -= gg[a][b];
                    kmat[(3 * iq + a) * dim + 3 * ip + b] -= gg[a][b];
                }
            }
        }
        let sol = resolver(&mut kmat, &mut rhs, dim);
        let mut omega = vec![[0.0; 3]; n];
        for (i, &p) in vivas.iter().enumerate() {
            omega[p] = [sol[3 * i], sol[3 * i + 1], sol[3 * i + 2]];
        }
        // Marco sin rotación neta: ω_net = (Σ M'_p)⁻¹ Σ M'_p ω_p.
        let mut mt = [[0.0; 3]; 3];
        let mut mw = [0.0; 3];
        for &p in &vivas {
            for a in 0..3 {
                for b in 0..3 {
                    mt[a][b] += m0[p][a][b];
                    mw[a] += m0[p][a][b] * omega[p][b];
                }
            }
        }
        if let Some(inv) = super::malla::inversa_simetrica(&mt) {
            let wnet = aplicar(&inv, mw);
            for &p in &vivas {
                omega[p] = sub(omega[p], wnet);
            }
        }
        // Calibración de c_b a la velocidad real de las placas (todo es lineal en 1/c_b).
        let v1 = self.v_rms_de(&omega);
        if self.t_myr >= self.prox_calibracion && v1 > 0.0 {
            self.c_b = v1 / (self.cfg.v_obj_cm_anio * CM_ANIO);
            self.prox_calibracion = self.t_myr + self.cfg.recalibrar_myr;
        }
        for &p in &vivas {
            self.placas[p].omega = esc(omega[p], 1.0 / self.c_b);
        }
    }

    /// Velocidad media cuadrática de la superficie (km/Myr) con las velocidades angulares dadas.
    fn v_rms_de(&self, omega: &[V3]) -> f64 {
        let (mut s, mut a) = (0.0, 0.0);
        for k in 0..self.malla.num {
            let p = self.dueno[k];
            if p == NADA {
                continue;
            }
            let v = velocidad(omega[p as usize], self.malla.dir[k], self.malla.r_km);
            s += dot(v, v) * self.malla.area[k];
            a += self.malla.area[k];
        }
        if a > 0.0 { (s / a).sqrt() } else { 0.0 }
    }

    /// Empujón inicial (decisión del responsable): eje sorteado y velocidad media de cada placa en
    /// el rango real (1–10 cm/año). Solo decide los primeros bordes: desde el paso siguiente mandan
    /// las fuerzas, y el modelo no tiene inercia.
    fn empujon_inicial(&mut self) {
        let (vmin, vmax) = (
            self.cfg.v_inicial_min_cm_anio,
            self.cfg.v_inicial_max_cm_anio,
        );
        let n = self.placas.len();
        let mut s2 = vec![0.0; n];
        let mut a = vec![0.0; n];
        let ejes: Vec<V3> = self
            .placas
            .iter()
            .map(|p| {
                dir_uniforme(
                    azar(self.seed, 0, p.id, P_EJE, 0),
                    azar(self.seed, 0, p.id, P_EJE, 1),
                )
            })
            .collect();
        for k in 0..self.malla.num {
            let p = self.dueno[k];
            if p == NADA {
                continue;
            }
            let v = velocidad(ejes[p as usize], self.malla.dir[k], self.malla.r_km);
            s2[p as usize] += dot(v, v) * self.malla.area[k];
            a[p as usize] += self.malla.area[k];
        }
        for (i, p) in self.placas.iter_mut().enumerate() {
            if !p.viva || a[i] <= 0.0 {
                continue;
            }
            let v_unit = (s2[i] / a[i]).sqrt();
            let v = vmin + (vmax - vmin) * azar(self.seed, 0, p.id, P_VEL, 0);
            p.omega = esc(ejes[i], v * CM_ANIO / v_unit.max(1e-12));
        }
    }

    // --- b–c. Movimiento y raster del mundo (§6.3) --------------------------------------------

    /// Celda del mundo que contiene el centro de cada muestra de cada placa.
    fn recalcular_mundo(&mut self) {
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            for s in 0..p.celda.len() {
                let x = aplicar(&p.rot, self.malla.dir[p.celda[s] as usize]);
                p.mundo[s] = self.malla.localizar(x) as u32;
            }
        }
    }

    /// Para cada celda del mundo, qué placas tienen corteza en su posición.
    fn rasterizar(&mut self) {
        let caps: Vec<(V3, f64)> = self
            .placas
            .iter()
            .map(|p| (aplicar(&p.rot, p.cap_c), p.cap_cos))
            .collect();
        for k in 0..self.malla.num {
            let x = self.malla.dir[k];
            let mut c = 0usize;
            for (pi, p) in self.placas.iter().enumerate() {
                if !p.viva || dot(x, caps[pi].0) < caps[pi].1 {
                    continue;
                }
                let l = self.malla.localizar(aplicar_t(&p.rot, x)) as u32;
                if p.mapa[l as usize] != NADA && c < MAXC {
                    self.cov[k][c] = (pi as u32, l);
                    c += 1;
                }
            }
            self.ncov[k] = c as u8;
        }
    }

    /// Solapes (§6.3–§6.4): quién se queda la celda y quién la pierde.
    ///
    /// La polaridad de una subducción entre cortezas oceánicas y los papeles de un choque son de
    /// todo el par de placas, no de cada celda (`incremento-4-diseno.md`, I7): se deciden en el
    /// primer contacto con la media de cada lado en los solapes del paso (más vieja abajo; en un
    /// choque sin subducción previa, la de corteza más fina abajo).
    fn resolver_solapes(&mut self) {
        self.perdidas.clear();
        // (id menor, id mayor) → (Σ edad·A y Σ A de cada lado en solapes O+O; Σ H·A y Σ A en C+C).
        let mut medias: Vec<((u64, u64), [f64; 8])> = Vec::new();
        for k in 0..self.malla.num {
            let n = self.ncov[k] as usize;
            if n < 2 {
                continue;
            }
            for i in 0..n {
                for j in i + 1..n {
                    let (a, b) = (self.cov[k][i], self.cov[k][j]);
                    let pa = &self.placas[a.0 as usize];
                    let pb = &self.placas[b.0 as usize];
                    let (Some(ca), Some(cb)) = (pa.get(a.1), pb.get(b.1)) else {
                        continue;
                    };
                    if ca.cont != cb.cont {
                        continue;
                    }
                    let (ida, idb) = (pa.id, pb.id);
                    let (x, y, cx, cy) = if ida < idb {
                        (ida, idb, ca, cb)
                    } else {
                        (idb, ida, cb, ca)
                    };
                    let a_k = self.malla.area[k];
                    let e = match medias.iter_mut().find(|m| m.0 == (x, y)) {
                        Some(m) => &mut m.1,
                        None => {
                            medias.push(((x, y), [0.0; 8]));
                            &mut medias.last_mut().expect("recién puesto").1
                        }
                    };
                    let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
                    if cx.cont {
                        e[4] += cx.h_eq(rs, rc) * a_k;
                        e[5] += a_k;
                        e[6] += cy.h_eq(rs, rc) * a_k;
                        e[7] += a_k;
                    } else {
                        e[0] += cx.edad_oc * a_k;
                        e[1] += a_k;
                        e[2] += cy.edad_oc * a_k;
                        e[3] += a_k;
                    }
                }
            }
        }
        let t = self.t_myr;
        for ((x, y), e) in medias {
            let i = self.par_idx(x, y);
            self.pares[i].contacto_myr = t;
            if e[1] > 0.0 && self.pares[i].polaridad_oo.is_none() {
                let (ex, ey) = (e[0] / e[1], e[2] / e[3]);
                self.pares[i].polaridad_oo = Some(if ex >= ey { x } else { y });
            }
            if e[5] > 0.0 && self.pares[i].choque.is_none() {
                let abajo = match self.pares[i].subduccion {
                    Some((s, _)) => s,
                    None => {
                        let (hx, hy) = (e[4] / e[5], e[6] / e[7]);
                        if hx < hy { x } else { y }
                    }
                };
                let arriba = if abajo == x { y } else { x };
                self.pares[i].choque = Some((abajo, arriba));
            }
        }
        for k in 0..self.malla.num {
            let n = self.ncov[k] as usize;
            self.hueco[k] = n == 0;
            if n == 0 {
                self.dueno[k] = NADA;
                self.rep[k] = NADA;
                continue;
            }
            let mut gana = self.cov[k][0];
            for i in 1..n {
                let otra = self.cov[k][i];
                let (g, pierde, choque) = self.torneo(gana, otra);
                self.perdidas.push(Perdida {
                    k: k as u32,
                    p: pierde.0,
                    l: pierde.1,
                    choque,
                });
                gana = g;
            }
            self.dueno[k] = gana.0;
            self.rep[k] = gana.1;
        }
    }

    /// Quién se queda una celda solapada por dos placas: `(ganadora, perdedora, ¿choque?)`.
    fn torneo(&mut self, a: (u32, u32), b: (u32, u32)) -> ((u32, u32), (u32, u32), bool) {
        let ca = *self.placas[a.0 as usize].get(a.1).expect("muestra");
        let cb = *self.placas[b.0 as usize].get(b.1).expect("muestra");
        let (ida, idb) = (self.placas[a.0 as usize].id, self.placas[b.0 as usize].id);
        let t = self.t_myr;
        let subduce = |s: (u32, u32), o: (u32, u32), d: &mut Deriva| {
            let (ids, ido) = (d.placas[s.0 as usize].id, d.placas[o.0 as usize].id);
            let i = d.par_idx(ids, ido);
            d.pares[i].subduccion = Some((ids, t));
            (o, s, false)
        };
        match (ca.cont, cb.cont) {
            (true, true) => {
                let i = self.par_idx(ida, idb);
                let abajo = match self.pares[i].choque {
                    Some((ab, _)) => ab,
                    None => {
                        // Abajo va la que subducía antes; si ninguna, la de corteza más fina en el
                        // contacto (decisión del responsable); empate: la de id mayor.
                        let ab = match self.pares[i].subduccion {
                            Some((s, _)) => s,
                            None => {
                                let (ha, hb) = (
                                    ca.h_eq(self.cfg.rho_s, self.cfg.rho_c),
                                    cb.h_eq(self.cfg.rho_s, self.cfg.rho_c),
                                );
                                if ha < hb || (ha == hb && ida > idb) {
                                    ida
                                } else {
                                    idb
                                }
                            }
                        };
                        let ar = if ab == ida { idb } else { ida };
                        self.pares[i].choque = Some((ab, ar));
                        ab
                    }
                };
                self.pares[i].choque_visto_myr = t;
                if abajo == ida {
                    (b, a, true)
                } else {
                    (a, b, true)
                }
            }
            (false, true) => subduce(a, b, self),
            (true, false) => subduce(b, a, self),
            (false, false) => {
                let i = self.par_idx(ida, idb);
                let hunde = self.pares[i]
                    .polaridad_oo
                    .unwrap_or(if ca.edad_oc >= cb.edad_oc { ida } else { idb });
                if hunde == ida {
                    subduce(a, b, self)
                } else {
                    subduce(b, a, self)
                }
            }
        }
    }

    /// Huecos (§6.3): cada celda sin placa pasa a la placa cubierta más cercana (empate: id menor);
    /// ahí nacerá corteza oceánica nueva.
    fn rellenar_huecos(&mut self) {
        let num = self.malla.num;
        let mut frente: Vec<u32> = (0..num as u32)
            .filter(|&k| self.dueno[k as usize] == NADA)
            .collect();
        while !frente.is_empty() {
            let mut asignar: Vec<(u32, u32, u32)> = Vec::new();
            let mut resto = Vec::new();
            for &k in &frente {
                let ku = k as usize;
                let mut mejor: Option<(f64, u64, u32, u32)> = None;
                for (j, &v) in self.malla.vec[ku].iter().enumerate() {
                    if v == NADA {
                        continue;
                    }
                    let p = self.dueno[v as usize];
                    if p == NADA {
                        continue;
                    }
                    // Empate de distancias (lo normal en una dorsal de una celda de ancho): sorteo
                    // por contador, sin sesgo, para que la expansión sea simétrica (I6).
                    let d = (self.malla.dvec[ku][j] * 1e6).round() / 1e6;
                    let id = self.placas[p as usize].id;
                    let sorteo =
                        splitmix64(self.seed ^ splitmix64(k as u64 ^ splitmix64(id ^ self.paso)));
                    let cand = (d, sorteo, p, v);
                    if mejor.is_none_or(|m| (cand.0, cand.1) < (m.0, m.1)) {
                        mejor = Some(cand);
                    }
                }
                match mejor {
                    Some((_, _, p, _)) => asignar.push((k, p, 0)),
                    None => resto.push(k),
                }
            }
            if asignar.is_empty() {
                break; // sin ninguna placa alrededor (no pasa en una deriva normal)
            }
            for (k, p, _) in asignar {
                self.dueno[k as usize] = p;
            }
            frente = resto;
        }
    }

    /// Pone al día los mapas de las placas con el mundo resuelto, sin remuestrear lo que no cambia
    /// (`incremento-4-diseno.md`, I2):
    /// - una celda local sigue si todavía representa alguna celda del mundo de su placa, o si
    ///   ninguna la representa (está «oculta» entre las del mundo) y su centro cae en su placa;
    /// - las que solo representan celdas perdidas en un solape se consumen (subducción o choque);
    /// - las celdas del mundo que eran hueco reciben corteza oceánica nueva, y se tapan los agujeros
    ///   del mapa local alrededor.
    ///
    /// Devuelve la corteza que la placa de abajo de un choque mete bajo la de arriba: (celda del
    /// frente, volumen en km³).
    fn reconstruir_mapas(&mut self) -> Vec<(u32, f64)> {
        let num = self.malla.num;
        let cfg = self.cfg.clone();
        let (rs, rc) = (cfg.rho_s, cfg.rho_c);
        // Marcas: 1 = representa una celda propia; 2 = solo perdidas por subducción; 3 = por choque.
        for k in 0..num {
            for i in 0..self.ncov[k] as usize {
                let (p, l) = self.cov[k][i];
                let propia = self.dueno[k] == p;
                let pl = &mut self.placas[p as usize];
                let m = &mut pl.marca[l as usize];
                if propia {
                    *m = 1;
                } else if *m != 1 {
                    *m = (*m).max(2);
                }
            }
        }
        for per in &self.perdidas {
            if per.choque {
                let m = &mut self.placas[per.p as usize].marca[per.l as usize];
                if *m == 2 {
                    *m = 3;
                }
            }
        }
        // Frente de cada celda perdida en un choque: la celda del mundo donde se perdió.
        let mut frente_de: Vec<(u32, u32, u32)> = self
            .perdidas
            .iter()
            .filter(|x| x.choque)
            .map(|x| (x.p, x.l, x.k))
            .collect();
        frente_de.sort_unstable();
        let mut choques: Vec<(u32, f64)> = Vec::new();
        let n = self.placas.len();
        for pi in 0..n {
            if !self.placas[pi].viva {
                continue;
            }
            // Quitar las consumidas y las ocultas que ya caen en otra placa.
            let mut quitar: Vec<(u32, u8)> = Vec::new();
            {
                let p = &self.placas[pi];
                for s in 0..p.celda.len() {
                    let l = p.celda[s];
                    // Las ocultas (ninguna celda del mundo las ve este paso) se conservan: cuando
                    // una celda del mundo las vuelva a ver, el solape decide (I2).
                    match p.marca[l as usize] {
                        0 | 1 => {}
                        m => quitar.push((l, m)),
                    }
                }
            }
            quitar.sort_unstable();
            for (l, m) in quitar {
                let Some(c) = self.placas[pi].quitar(l) else {
                    continue;
                };
                if !c.cont {
                    continue;
                }
                let v = c.h_eq(rs, rc) * self.malla.area[l as usize];
                let frente = if m == 3 {
                    frente_de
                        .binary_search_by(|x| (x.0, x.1).cmp(&(pi as u32, l)))
                        .ok()
                        .map(|i| frente_de[i].2)
                } else {
                    None
                };
                match frente {
                    Some(k) => {
                        choques.push((k, cfg.eta_col * v));
                        self.cuentas.choque_perdida_km3 += (1.0 - cfg.eta_col) * v;
                    }
                    None => self.cuentas.subducida_km3 += v,
                }
            }
        }
        for k in 0..num {
            for i in 0..self.ncov[k] as usize {
                let (p, l) = self.cov[k][i];
                if let Some(m) = self.placas[p as usize].marca.get_mut(l as usize) {
                    *m = 0;
                }
            }
        }
        // Corteza nueva en los huecos (su celda local por el giro inverso) y agujeros alrededor.
        let mut nuevas: Vec<(u32, u32)> = Vec::new();
        self.area_nueva_km2 = 0.0;
        for k in 0..num {
            if !self.hueco[k] {
                continue;
            }
            self.area_nueva_km2 += self.malla.area[k];
            let pi = self.dueno[k];
            if pi == NADA {
                continue;
            }
            let p = &mut self.placas[pi as usize];
            let l = self.malla.localizar(aplicar_t(&p.rot, self.malla.dir[k])) as u32;
            if p.mapa[l as usize] == NADA {
                p.insertar(l, Corteza::oceanica(cfg.h_oc_km), k as u32);
                nuevas.push((pi, l));
            }
            self.rep[k] = l;
        }
        for (pi, l) in nuevas {
            for j in 0..8 {
                let l2 = self.malla.vec[l as usize][j];
                if l2 == NADA || self.placas[pi as usize].mapa[l2 as usize] != NADA {
                    continue;
                }
                let x = aplicar(&self.placas[pi as usize].rot, self.malla.dir[l2 as usize]);
                let k2 = self.malla.localizar(x);
                if self.dueno[k2] != pi {
                    continue;
                }
                let c = if self.hueco[k2] {
                    Corteza::oceanica(cfg.h_oc_km)
                } else {
                    match self.placas[pi as usize].get(self.rep[k2]) {
                        Some(c) => *c,
                        None => continue,
                    }
                };
                self.placas[pi as usize].insertar(l2, c, k2 as u32);
            }
        }
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.actualizar_cap(&self.malla);
        }
        choques
    }

    /// Muestras (placa, muestra) del dueño cuyo centro cae en cada celda del mundo: así un cambio
    /// calculado en el mundo llega a todas las celdas locales que la cubren (I2).
    fn indice_inverso(&self) -> (Vec<u32>, Vec<u32>) {
        let num = self.malla.num;
        let mut off = vec![0u32; num + 1];
        for (pi, p) in self.placas.iter().enumerate() {
            if !p.viva {
                continue;
            }
            for &k in &p.mundo {
                if self.dueno[k as usize] == pi as u32 {
                    off[k as usize + 1] += 1;
                }
            }
        }
        for k in 0..num {
            off[k + 1] += off[k];
        }
        let mut pos = off.clone();
        let mut lista = vec![0u32; off[num] as usize];
        for (pi, p) in self.placas.iter().enumerate() {
            if !p.viva {
                continue;
            }
            for (s, &k) in p.mundo.iter().enumerate() {
                if self.dueno[k as usize] == pi as u32 {
                    lista[pos[k as usize] as usize] = s as u32;
                    pos[k as usize] += 1;
                }
            }
        }
        (off, lista)
    }

    /// Aplica `f` a todas las muestras del dueño de la celda del mundo `k` (y devuelve el área local
    /// tocada). Si ninguna muestra cae en `k`, se aplica a su representante.
    fn en_celda(
        &mut self,
        inv: &(Vec<u32>, Vec<u32>),
        k: usize,
        mut f: impl FnMut(&mut Corteza, f64),
    ) {
        let pi = self.dueno[k];
        if pi == NADA {
            return;
        }
        let (off, lista) = inv;
        let (a, b) = (off[k] as usize, off[k + 1] as usize);
        let p = &mut self.placas[pi as usize];
        if a == b {
            let l = self.rep[k];
            if let Some(c) = p.get_mut(l) {
                f(c, self.malla.area[l as usize]);
            }
            return;
        }
        for &s in &lista[a..b] {
            let l = p.celda[s as usize];
            f(&mut p.cort[s as usize], self.malla.area[l as usize]);
        }
    }

    // --- Bordes (para las fuerzas y los arcos) -------------------------------------------------

    /// Bordes de placa: normal y longitud por mínimos cuadrados del indicador de la otra placa,
    /// velocidad de convergencia y tipo (`incremento-4-diseno.md`, I3–I4).
    fn clasificar_bordes(&mut self) {
        self.bordes.clear();
        let r = self.malla.r_km;
        for k in 0..self.malla.num {
            let p = self.dueno[k];
            if p == NADA {
                continue;
            }
            let vec = self.malla.vec[k];
            // Placa vecina dominante.
            let mut cuenta: [(u32, u8); 8] = [(NADA, 0); 8];
            let mut nq = 0;
            for &v in &vec {
                if v == NADA {
                    continue;
                }
                let q = self.dueno[v as usize];
                if q == NADA || q == p {
                    continue;
                }
                match cuenta[..nq].iter_mut().find(|c| c.0 == q) {
                    Some(c) => c.1 += 1,
                    None => {
                        cuenta[nq] = (q, 1);
                        nq += 1;
                    }
                }
            }
            if nq == 0 {
                continue;
            }
            let (q, _) = *cuenta[..nq]
                .iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
                .expect("hay vecina");
            // Gradiente del indicador de q en el plano tangente.
            let x = self.malla.dir[k];
            let (e, nn) = east_north(x);
            let (mut sxx, mut sxy, mut syy, mut bx, mut by) = (0.0, 0.0, 0.0, 0.0, 0.0);
            for &v in &vec {
                if v == NADA {
                    continue;
                }
                let dv = sub(self.malla.dir[v as usize], x);
                let (de, dn) = (dot(dv, e) * r, dot(dv, nn) * r);
                let chi = if self.dueno[v as usize] == q {
                    1.0
                } else {
                    0.0
                };
                sxx += de * de;
                sxy += de * dn;
                syy += dn * dn;
                bx += de * chi;
                by += dn * chi;
            }
            let det = sxx * syy - sxy * sxy;
            if det.abs() < 1e-12 {
                continue;
            }
            let (ge, gn) = ((syy * bx - sxy * by) / det, (sxx * by - sxy * bx) / det);
            let gmod = (ge * ge + gn * gn).sqrt();
            if gmod <= 0.0 {
                continue;
            }
            let normal = normalize(add(esc(e, ge / gmod), esc(nn, gn / gmod)));
            let largo = 2.0 * self.malla.area[k] * gmod;
            // Celda de q más alineada con la normal.
            let k_q = vec
                .iter()
                .filter(|&&v| v != NADA && self.dueno[v as usize] == q)
                .max_by(|&&a, &&b| {
                    dot(sub(self.malla.dir[a as usize], x), normal)
                        .total_cmp(&dot(sub(self.malla.dir[b as usize], x), normal))
                        .then(b.cmp(&a))
                })
                .copied()
                .expect("hay celda de q");
            // Con el empuje de apertura de un rift recién hecho: así la dorsal empuja desde ya.
            let pp = &self.placas[p as usize];
            let pq = &self.placas[q as usize];
            let vp = velocidad(add(pp.omega, pp.empuje), x, r);
            let vq = velocidad(add(pq.omega, pq.empuje), x, r);
            let vrel = sub(vp, vq);
            let v_n = dot(vrel, normal);
            let vmod = dot(vrel, vrel).sqrt();
            let cp = self.corteza(k).copied();
            let cq = self.corteza(k_q as usize).copied();
            let (Some(cp), Some(cq)) = (cp, cq) else {
                continue;
            };
            let (tipo, p_subduce) = if vmod < 0.05 * CM_ANIO
                || v_n.abs() < self.cfg.f_transformante * vmod
            {
                (TipoBorde::Transformante, false)
            } else if v_n < 0.0 {
                (TipoBorde::Divergente, false)
            } else {
                match (cp.cont, cq.cont) {
                    (true, true) => (TipoBorde::Colision, false),
                    (false, true) => (TipoBorde::Subduccion, true),
                    (true, false) => (TipoBorde::Subduccion, false),
                    (false, false) => {
                        let idp = self.placas[p as usize].id;
                        let idq = self.placas[q as usize].id;
                        let (a, b) = if idp < idq { (idp, idq) } else { (idq, idp) };
                        let pol = self
                            .pares
                            .binary_search_by(|x| (x.a, x.b).cmp(&(a, b)))
                            .ok()
                            .and_then(|i| self.pares[i].polaridad_oo);
                        let hunde_p = match pol {
                            Some(h) => h == idp,
                            None => {
                                cp.edad_oc > cq.edad_oc || (cp.edad_oc == cq.edad_oc && idp > idq)
                            }
                        };
                        (TipoBorde::Subduccion, hunde_p)
                    }
                }
            };
            self.bordes.push(Borde {
                k: k as u32,
                p,
                q,
                k_q,
                normal,
                largo_km: largo,
                v_n,
                tipo,
                p_subduce,
            });
        }
    }

    // --- d. Arcos (§6.3) -----------------------------------------------------------------------

    /// En la placa que cabalga, a `d_arco ± w_arco/2` de la fosa, la corteza crece `q_arco` km² por
    /// km de fosa y Myr; la oceánica que supera `H_cont_min` pasa a continental.
    fn crecer_arcos(&mut self) {
        let cfg = self.cfg.clone();
        let fuentes: Vec<(u32, f64, u32)> = self
            .bordes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.tipo == TipoBorde::Subduccion && !b.p_subduce)
            .map(|(i, b)| {
                (
                    b.k,
                    0.5 * self.malla.dist_km(b.k as usize, b.k_q as usize),
                    i as u32,
                )
            })
            .collect();
        if fuentes.is_empty() {
            return;
        }
        let max_d = cfg.d_arco_km + 0.5 * cfg.w_arco_km;
        let dueno = &self.dueno;
        // Cada celda de la placa que cabalga se mide desde su fosa, sin salir de la placa.
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(&self.malla, &fuentes, |u, v| dueno[u] == dueno[v], max_d);
        let (lo, hi) = (
            cfg.d_arco_km - 0.5 * cfg.w_arco_km,
            cfg.d_arco_km + 0.5 * cfg.w_arco_km,
        );
        // Área de la franja de cada tramo de fosa (solo celdas de su misma placa).
        let mut area_franja = vec![0.0; self.bordes.len()];
        let mut celdas: Vec<(u32, u32)> = Vec::new();
        for &k in &campo.orden {
            let ku = k as usize;
            let d = campo.dist[ku];
            let f = campo.fuente[ku];
            if f == NADA || d < lo || d > hi {
                continue;
            }
            let b = &self.bordes[f as usize];
            if self.dueno[ku] != b.p {
                continue;
            }
            area_franja[f as usize] += self.malla.area[ku];
            celdas.push((k, f));
        }
        self.campo = campo;
        let inv = self.indice_inverso();
        let (rs, rc) = (cfg.rho_s, cfg.rho_c);
        let mut acrecion_area = 0.0;
        for (k, f) in celdas {
            let b = self.bordes[f as usize];
            let a_f = area_franja[f as usize];
            if a_f <= 0.0 {
                continue;
            }
            let dh = cfg.q_arco * b.largo_km * cfg.dt_myr / a_f;
            let mut arcos = 0.0;
            let mut acr = 0.0;
            let mut acr_a = 0.0;
            self.en_celda(&inv, k as usize, |c, a| {
                if c.sed > 0.0 {
                    c.h += c.sed * rs / rc;
                    c.sed = 0.0;
                }
                let era_cont = c.cont;
                c.h += dh;
                c.dh_dt += dh / cfg.dt_myr;
                c.edad_orog = 0.0;
                if era_cont {
                    arcos += dh * a;
                } else if c.h > cfg.h_cont_min_km {
                    c.cont = true;
                    acr += c.h * a;
                    acr_a += a;
                }
            });
            self.cuentas.arcos_km3 += arcos;
            self.cuentas.acrecion_km3 += acr;
            acrecion_area += acr_a;
        }
        if acrecion_area > 0.0 {
            self.evento("acrecion_arco", vec![], 0.0, acrecion_area, None);
        }
    }

    // --- d. Choques y mesetas (§6.4) -----------------------------------------------------------

    /// La corteza que la placa de abajo mete bajo la de arriba engruesa a esta desde el frente hacia
    /// dentro: uniforme en `W_min_orog`, sin pasar de `H_max` (el exceso sigue hacia el interior).
    fn engrosar_choques(&mut self, choques: &[(u32, f64)]) {
        if choques.is_empty() {
            return;
        }
        let cfg = self.cfg.clone();
        // Volumen por celda de frente.
        let mut por_frente: Vec<(u32, f64)> = choques.to_vec();
        por_frente.sort_unstable_by_key(|x| x.0);
        let mut agrupado: Vec<(u32, f64)> = Vec::new();
        for (k, v) in por_frente {
            match agrupado.last_mut() {
                Some(u) if u.0 == k => u.1 += v,
                _ => agrupado.push((k, v)),
            }
        }
        let fuentes: Vec<(u32, f64, u32)> = agrupado
            .iter()
            .enumerate()
            .map(|(i, &(k, _))| (k, 0.0, i as u32))
            .collect();
        let dueno = self.dueno.clone();
        let fuente_placa: Vec<u32> = agrupado.iter().map(|&(k, _)| dueno[k as usize]).collect();
        // Solo la corteza continental de la placa de arriba se apila (no su fondo oceánico).
        let cont: Vec<bool> = (0..self.malla.num)
            .map(|k| self.corteza(k).is_some_and(|c| c.cont))
            .collect();
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(
            &self.malla,
            &fuentes,
            |u, v| dueno[u] == dueno[v] && cont[v],
            f64::INFINITY,
        );
        // Cuenca de cada frente, en orden de distancia, solo en la placa de arriba.
        let mut cuenca: Vec<Vec<u32>> = vec![Vec::new(); agrupado.len()];
        for &k in &campo.orden {
            let f = campo.fuente[k as usize];
            if f != NADA && dueno[k as usize] == fuente_placa[f as usize] {
                cuenca[f as usize].push(k);
            }
        }
        let dist = campo.dist.clone();
        self.campo = campo;
        let (rs, rc) = (cfg.rho_s, cfg.rho_c);
        // Espesor actual (equivalente) de cada celda del mundo tocada.
        let h_de = |d: &Deriva, k: usize| d.corteza(k).map_or(0.0, |c| c.h_eq(rs, rc));
        let mut dh_mundo: Vec<(u32, f64)> = Vec::new();
        let mut sin_sitio = 0.0;
        for (f, celdas) in cuenca.iter().enumerate() {
            let mut v = agrupado[f].1;
            if celdas.is_empty() {
                sin_sitio += v;
                continue;
            }
            let banda: Vec<u32> = celdas
                .iter()
                .copied()
                .take_while(|&k| dist[k as usize] < cfg.w_min_orog_km)
                .collect();
            let banda = if banda.is_empty() {
                vec![celdas[0]]
            } else {
                banda
            };
            let a_banda: f64 = banda.iter().map(|&k| self.malla.area[k as usize]).sum();
            let dh0 = v / a_banda;
            for &k in &banda {
                let room = (cfg.h_max_km - h_de(self, k as usize)).max(0.0);
                let dh = dh0.min(room);
                if dh > 0.0 {
                    dh_mundo.push((k, dh));
                    v -= dh * self.malla.area[k as usize];
                }
            }
            for &k in celdas.iter().skip(banda.len()) {
                if v <= 1e-12 {
                    break;
                }
                let a = self.malla.area[k as usize];
                let room = (cfg.h_max_km - h_de(self, k as usize)).max(0.0);
                let dh = (v / a).min(room);
                if dh > 0.0 {
                    dh_mundo.push((k, dh));
                    v -= dh * a;
                }
            }
            sin_sitio += v.max(0.0);
        }
        self.cuentas.choque_sin_sitio_km3 += sin_sitio;
        dh_mundo.sort_unstable_by_key(|x| x.0);
        let inv = self.indice_inverso();
        for (k, dh) in dh_mundo {
            let mut puesto = 0.0;
            self.en_celda(&inv, k as usize, |c, a| {
                if c.sed > 0.0 {
                    c.h += c.sed * rs / rc;
                    c.sed = 0.0;
                }
                c.h += dh;
                c.dh_dt += dh / cfg.dt_myr;
                c.edad_orog = 0.0;
                puesto += dh * a;
            });
            self.cuentas.choque_engrosamiento_km3 += puesto;
        }
    }

    // --- e. Erosión tectónica, envejecimiento y sedimento (§6.6) ------------------------------

    /// Envejece la corteza y gasta las cordilleras: el exceso de espesor sobre `H_0` decae con
    /// `τ_orog`. Devuelve la roca erosionada en cada celda del mundo (km³).
    fn erosion_y_envejecimiento(&mut self) -> Vec<(u32, f64)> {
        let cfg = &self.cfg;
        let f = 1.0 - (-cfg.dt_myr / cfg.tau_orog_myr).exp();
        let mut ero: Vec<(u32, f64)> = Vec::new();
        let mut total = 0.0;
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            for s in 0..p.cort.len() {
                let c = &mut p.cort[s];
                if c.cont {
                    if c.h > cfg.h0_km {
                        let dh = (c.h - cfg.h0_km) * f;
                        c.h -= dh;
                        let v = dh * self.malla.area[p.celda[s] as usize];
                        ero.push((p.mundo[s], v));
                        total += v;
                    }
                } else {
                    c.edad_oc += cfg.dt_myr;
                }
                c.edad_orog += cfg.dt_myr;
            }
        }
        self.cuentas.erosion_km3 += total;
        ero.sort_unstable_by_key(|x| x.0);
        ero
    }

    /// El sedimento va a los márgenes continentales de la misma placa más cercanos (dentro de
    /// `w_margen` de la costa) hasta la profundidad de plataforma (−130 m); lo que no cabe sale al
    /// fondo oceánico y se pierde.
    fn sedimento(&mut self, ero: &[(u32, f64)]) {
        if ero.is_empty() {
            return;
        }
        let cfg = self.cfg.clone();
        let num = self.malla.num;
        let z = self.alturas();
        let zeta = self.zeta_m;
        let z_plat = zeta - 130.0;
        // Distancia a la costa (desde la tierra) por el mar.
        let mut costa: Vec<(u32, f64, u32)> = Vec::new();
        for k in 0..num {
            if z[k] >= zeta
                && self.malla.vec[k]
                    .iter()
                    .any(|&v| v != NADA && z[v as usize] < zeta)
            {
                costa.push((k as u32, 0.0, k as u32));
            }
        }
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(&self.malla, &costa, |_, v| z[v] < zeta, cfg.w_margen_km);
        let margen: Vec<bool> = (0..num)
            .map(|k| {
                campo.dist[k].is_finite()
                    && z[k] < z_plat
                    && self.corteza(k).is_some_and(|c| c.cont)
            })
            .collect();
        self.campo = campo;
        // Margen más cercano de la misma placa para cada celda (Dijkstra por placa).
        let fuentes: Vec<(u32, f64, u32)> = (0..num)
            .filter(|&k| margen[k])
            .map(|k| (k as u32, 0.0, k as u32))
            .collect();
        let dueno = self.dueno.clone();
        let mut campo2 = std::mem::take(&mut self.campo2);
        // Sin salir de la placa: cada celda recibe el margen más cercano de la suya.
        campo2.calcular(
            &self.malla,
            &fuentes,
            |u, v| dueno[u] == dueno[v],
            f64::INFINITY,
        );
        // Capacidad de cada margen (km³ de roca) y recibido.
        let mut recibido: Vec<(u32, f64)> = Vec::new();
        let mut perdido = 0.0;
        for &(k, v) in ero {
            let m = campo2.fuente[k as usize];
            if m == NADA || dueno[m as usize] != dueno[k as usize] {
                perdido += v;
                continue;
            }
            recibido.push((m, v));
        }
        self.campo2 = campo2;
        recibido.sort_unstable_by_key(|x| x.0);
        let mut por_margen: Vec<(u32, f64)> = Vec::new();
        for (k, v) in recibido {
            match por_margen.last_mut() {
                Some(u) if u.0 == k => u.1 += v,
                _ => por_margen.push((k, v)),
            }
        }
        let area = &self.malla.area;
        let cap_roca = |k: usize| -> f64 {
            isostasia::sedimento_hasta(&cfg, z[k], z_plat) * cfg.rho_s / cfg.rho_c * area[k]
        };
        // Primera pasada: cada margen, lo suyo; el sobrante, a los demás márgenes de su placa.
        let mut deposito: Vec<(u32, f64)> = Vec::new();
        let mut sobra_placa: Vec<f64> = vec![0.0; self.placas.len()];
        let mut libre: Vec<(u32, f64)> = Vec::new();
        for (k, v) in &por_margen {
            let cap = cap_roca(*k as usize);
            let d = v.min(cap);
            deposito.push((*k, d));
            sobra_placa[dueno[*k as usize] as usize] += v - d;
        }
        for (k, _) in margen.iter().enumerate().filter(|x| *x.1) {
            {
                let usado = por_margen
                    .binary_search_by_key(&(k as u32), |x| x.0)
                    .map_or(0.0, |i| deposito[i].1);
                let cap = cap_roca(k) - usado;
                if cap > 0.0 {
                    libre.push((k as u32, cap));
                }
            }
        }
        let mut libre_placa: Vec<f64> = vec![0.0; self.placas.len()];
        for &(k, c) in &libre {
            libre_placa[dueno[k as usize] as usize] += c;
        }
        for &(k, c) in &libre {
            let pi = dueno[k as usize] as usize;
            if sobra_placa[pi] > 0.0 && libre_placa[pi] > 0.0 {
                let d = c * (sobra_placa[pi] / libre_placa[pi]).min(1.0);
                deposito.push((k, d));
            }
        }
        for pi in 0..self.placas.len() {
            if sobra_placa[pi] > libre_placa[pi] {
                perdido += sobra_placa[pi] - libre_placa[pi];
            }
        }
        self.cuentas.sedimento_perdido_km3 += perdido;
        deposito.sort_unstable_by_key(|x| x.0);
        let inv = self.indice_inverso();
        for (k, v) in deposito {
            let a_k = self.malla.area[k as usize];
            let ds = v / a_k * cfg.rho_c / cfg.rho_s;
            let mut puesto = 0.0;
            self.en_celda(&inv, k as usize, |c, a| {
                c.sed += ds;
                puesto += ds * a * cfg.rho_s / cfg.rho_c;
            });
            self.cuentas.sedimento_km3 += puesto;
        }
    }

    // --- f. Eventos de placas (§6.4–§6.5) ------------------------------------------------------

    fn evento(&mut self, tipo: &str, placas: Vec<u64>, largo: f64, area: f64, punto: Option<V3>) {
        let (lat, lon) = punto.map_or((0.0, 0.0), |d| {
            let (a, b) = lat_lon(d);
            (a.to_degrees(), b.to_degrees())
        });
        self.eventos.push(EventoTectonico {
            myr: self.t_myr,
            tipo: tipo.to_string(),
            placas,
            longitud_km: largo,
            area_km2: area,
            lat_deg: lat,
            lon_deg: lon,
        });
    }

    fn area_placa(&self) -> Vec<f64> {
        let mut a = vec![0.0; self.placas.len()];
        for k in 0..self.malla.num {
            if self.dueno[k] != NADA {
                a[self.dueno[k] as usize] += self.malla.area[k];
            }
        }
        a
    }

    /// Funde la placa `b` en la `a`: su corteza pasa al sistema de `a` (una sola vez) y se tapan los
    /// agujeros del mapa de `a`.
    fn fundir(&mut self, a: usize, b: usize) {
        let num = self.malla.num;
        let mut nuevas: Vec<u32> = Vec::new();
        for k in 0..num {
            if self.dueno[k] != b as u32 {
                continue;
            }
            let c = match self.placas[b].get(self.rep[k]) {
                Some(c) => *c,
                None => continue,
            };
            let pa = &mut self.placas[a];
            let l = self.malla.localizar(aplicar_t(&pa.rot, self.malla.dir[k])) as u32;
            if pa.mapa[l as usize] == NADA {
                pa.insertar(l, c, k as u32);
                nuevas.push(l);
            }
            self.dueno[k] = a as u32;
            self.rep[k] = l;
        }
        // Agujeros: celdas locales de a cuyo centro cae en lo que era de b.
        for l in nuevas {
            for j in 0..8 {
                let l2 = self.malla.vec[l as usize][j];
                if l2 == NADA || self.placas[a].mapa[l2 as usize] != NADA {
                    continue;
                }
                let x = aplicar(&self.placas[a].rot, self.malla.dir[l2 as usize]);
                let k2 = self.malla.localizar(x);
                if self.dueno[k2] != a as u32 {
                    continue;
                }
                if let Some(c) = self.placas[a].get(self.rep[k2]).copied() {
                    self.placas[a].insertar(l2, c, k2 as u32);
                }
            }
        }
        self.placas[b].morir();
        self.placas[a].actualizar_cap(&self.malla);
    }

    /// Suturas (§6.4): dos placas en colisión que convergen a menos de `v_sutura` durante más de
    /// `t_sutura` se funden (la mayor absorbe a la menor).
    fn suturas(&mut self) -> bool {
        // Convergencia media y longitud del frente de cada par en colisión.
        let mut frentes: Vec<(u32, u32, f64, f64, V3)> = Vec::new();
        for b in &self.bordes {
            if b.tipo != TipoBorde::Colision {
                continue;
            }
            let (x, y) = if b.p < b.q { (b.p, b.q) } else { (b.q, b.p) };
            match frentes.iter_mut().find(|f| f.0 == x && f.1 == y) {
                Some(f) => {
                    f.2 += b.v_n * b.largo_km;
                    f.3 += b.largo_km;
                }
                None => frentes.push((
                    x,
                    y,
                    b.v_n * b.largo_km,
                    b.largo_km,
                    self.malla.dir[b.k as usize],
                )),
            }
        }
        let dt = self.cfg.dt_myr;
        let mut fundir: Vec<(usize, usize, f64, V3)> = Vec::new();
        for &(x, y, vl, l, punto) in &frentes {
            let (ix, iy) = (self.placas[x as usize].id, self.placas[y as usize].id);
            let i = self.par_idx(ix, iy);
            let v_media = vl / l.max(1e-9);
            if v_media < self.cfg.v_sutura_cm_anio * CM_ANIO {
                self.pares[i].sutura_myr += dt;
            } else {
                self.pares[i].sutura_myr = 0.0;
            }
            if self.pares[i].sutura_myr >= self.cfg.t_sutura_myr {
                fundir.push((x as usize, y as usize, 0.5 * l, punto));
            }
        }
        if fundir.is_empty() {
            return false;
        }
        let areas = self.area_placa();
        let mut cambio = false;
        for (x, y, l, punto) in fundir {
            if !self.placas[x].viva || !self.placas[y].viva {
                continue;
            }
            let (a, b) = if areas[x] >= areas[y] { (x, y) } else { (y, x) };
            let ids = vec![self.placas[a].id, self.placas[b].id];
            self.fundir(a, b);
            self.evento("sutura", ids, l, 0.0, Some(punto));
            cambio = true;
        }
        cambio
    }

    /// Parte la placa `pi` por un círculo máximo de polo `polo` deformado por ruido. Devuelve los
    /// índices de las dos hijas (la del lado `+polo` primero), o `None` si una quedaría vacía o
    /// demasiado pequeña.
    fn partir(
        &mut self,
        pi: usize,
        polo: V3,
        centro: V3,
        adelgazar: bool,
    ) -> Option<(usize, usize)> {
        let cfg = self.cfg.clone();
        let amp = cfg.deformacion;
        let lado = |d: &Deriva, x: V3| -> f64 {
            let f = 2.5;
            dot(x, polo) + amp * d.ruido_def[0].fbm(x[0] * f, x[1] * f, x[2] * f, 3, 0.5, 2.0)
        };
        let madre = &self.placas[pi];
        let mut a_mas = 0.0;
        let mut a_menos = 0.0;
        let mut lados = Vec::with_capacity(madre.celda.len());
        for s in 0..madre.celda.len() {
            let x = aplicar(&madre.rot, self.malla.dir[madre.celda[s] as usize]);
            let sv = lado(self, x);
            lados.push(sv);
            let a = self.malla.area[madre.celda[s] as usize];
            if sv >= 0.0 {
                a_mas += a;
            } else {
                a_menos += a;
            }
        }
        let a_min = cfg.a_min_frac * self.malla.area_total();
        if a_mas < a_min || a_menos < a_min {
            return None;
        }
        let paso = self.paso;
        let id_m = madre.id;
        let t = self.t_myr;
        let mut h1 = Placa::hermana(madre, id_hija(id_m, paso, 0), t);
        let mut h2 = Placa::hermana(madre, id_hija(id_m, paso, 1), t);
        let w_rad = cfg.w_margen_km / self.malla.r_km;
        let mut adelg = 0.0;
        for (s, &lado_s) in lados.iter().enumerate() {
            let l = madre.celda[s];
            let mut c = madre.cort[s];
            if adelgazar && c.cont && lado_s.abs() < w_rad {
                let antes = c.h;
                c.h *= cfg.f_rift;
                adelg += (antes - c.h) * self.malla.area[l as usize];
            }
            if lado_s >= 0.0 {
                h1.insertar(l, c, madre.mundo[s]);
            } else {
                h2.insertar(l, c, madre.mundo[s]);
            }
        }
        self.cuentas.rift_km3 += adelg;
        // Apertura: giro alrededor del eje del círculo a 90° del centro (en el centro, v_rift).
        let t_eje = normalize(cross(polo, centro));
        let w = cfg.v_rift_cm_anio * CM_ANIO / (2.0 * self.malla.r_km);
        h1.empuje = esc(t_eje, -w);
        h2.empuje = esc(t_eje, w);
        h1.actualizar_cap(&self.malla);
        h2.actualizar_cap(&self.malla);
        self.placas[pi].morir();
        let i1 = self.placas.len();
        self.placas.push(h1);
        self.placas.push(h2);
        for k in 0..self.malla.num {
            if self.dueno[k] == pi as u32 {
                let l = self.rep[k];
                self.dueno[k] = if self.placas[i1].mapa[l as usize] != NADA {
                    i1 as u32
                } else {
                    i1 as u32 + 1
                };
            }
        }
        Some((i1, i1 + 1))
    }

    /// Rifts (§6.5): probabilidad por paso `(Δt/τ_rift)·(A_cont,p/A_cont,total)`.
    fn rifts(&mut self) -> bool {
        let num = self.malla.num;
        let n = self.placas.len();
        let mut a_cont = vec![0.0; n];
        let mut centro = vec![[0.0; 3]; n];
        for k in 0..num {
            if let Some(c) = self.corteza(k)
                && c.cont
            {
                let p = self.dueno[k] as usize;
                a_cont[p] += self.malla.area[k];
                centro[p] = add(centro[p], esc(self.malla.dir[k], self.malla.area[k]));
            }
        }
        let total: f64 = a_cont.iter().sum();
        if total <= 0.0 {
            return false;
        }
        let mut cambio = false;
        for pi in 0..n {
            if !self.placas[pi].viva || a_cont[pi] <= 0.0 {
                continue;
            }
            let prob = self.cfg.dt_myr / self.cfg.tau_rift_myr * a_cont[pi] / total;
            let id = self.placas[pi].id;
            if azar(self.seed, self.paso, id, P_RIFT, 0) >= prob {
                continue;
            }
            let c = normalize(centro[pi]);
            let (e, nn) = east_north(c);
            let th = std::f64::consts::PI * azar(self.seed, self.paso, id, P_ORIENT, 0);
            let polo = normalize(add(esc(e, th.cos()), esc(nn, th.sin())));
            if let Some((h1, h2)) = self.partir(pi, polo, c, true) {
                let ids = vec![id, self.placas[h1].id, self.placas[h2].id];
                // Longitud de la línea dentro del continente: celdas continentales junto a ella.
                let largo = self.largo_rift(h1, h2);
                self.evento("rift", ids, largo, a_cont[pi], Some(c));
                cambio = true;
            }
        }
        cambio
    }

    fn largo_rift(&self, h1: usize, h2: usize) -> f64 {
        let mut l = 0.0;
        for k in 0..self.malla.num {
            if self.dueno[k] != h1 as u32 || !self.corteza(k).is_some_and(|c| c.cont) {
                continue;
            }
            if self.malla.vec[k]
                .iter()
                .any(|&v| v != NADA && self.dueno[v as usize] == h2 as u32)
            {
                l += self.malla.lado_km;
            }
        }
        l
    }

    /// Placas pequeñas (§6.5): menos de `A_min` → se absorben en la vecina con más borde común.
    fn absorciones(&mut self) -> bool {
        let areas = self.area_placa();
        let a_min = self.cfg.a_min_frac * self.malla.area_total();
        let mut cambio = false;
        for (pi, &area_p) in areas.iter().enumerate() {
            if !self.placas[pi].viva || area_p >= a_min {
                continue;
            }
            // Vecina con más borde común.
            let mut borde: Vec<(u32, f64)> = Vec::new();
            for b in &self.bordes {
                if b.p as usize == pi {
                    match borde.iter_mut().find(|x| x.0 == b.q) {
                        Some(x) => x.1 += b.largo_km,
                        None => borde.push((b.q, b.largo_km)),
                    }
                }
            }
            let Some(&(q, _)) = borde
                .iter()
                .filter(|x| self.placas[x.0 as usize].viva)
                .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
            else {
                continue;
            };
            let ids = vec![self.placas[pi].id, self.placas[q as usize].id];
            self.fundir(q as usize, pi);
            self.evento("absorcion", ids, 0.0, area_p, None);
            cambio = true;
        }
        cambio
    }

    /// Mínimo de placas (§6.5): con menos de `N_min`, se abre una dorsal nueva en la placa con más
    /// fondo oceánico.
    fn saltos_de_dorsal(&mut self) -> bool {
        let vivas = self.placas.iter().filter(|p| p.viva).count();
        if vivas >= self.cfg.n_min as usize {
            return false;
        }
        let n = self.placas.len();
        let mut a_oc = vec![0.0; n];
        let mut centro = vec![[0.0; 3]; n];
        for k in 0..self.malla.num {
            if let Some(c) = self.corteza(k)
                && !c.cont
            {
                let p = self.dueno[k] as usize;
                a_oc[p] += self.malla.area[k];
                centro[p] = add(centro[p], esc(self.malla.dir[k], self.malla.area[k]));
            }
        }
        let Some(pi) = (0..n)
            .filter(|&i| self.placas[i].viva && a_oc[i] > 0.0)
            .max_by(|&a, &b| a_oc[a].total_cmp(&a_oc[b]).then(b.cmp(&a)))
        else {
            return false;
        };
        let c = normalize(centro[pi]);
        let (e, nn) = east_north(c);
        let id = self.placas[pi].id;
        let th = std::f64::consts::PI * azar(self.seed, self.paso, id, P_SALTO, 0);
        let polo = normalize(add(esc(e, th.cos()), esc(nn, th.sin())));
        match self.partir(pi, polo, c, false) {
            Some((h1, h2)) => {
                let ids = vec![id, self.placas[h1].id, self.placas[h2].id];
                self.evento("salto_dorsal", ids, 0.0, a_oc[pi], Some(c));
                true
            }
            None => false,
        }
    }

    // --- g. Métricas y eventos de inicio (§7–§8) -----------------------------------------------

    fn metricas_del_paso(&mut self) {
        let num = self.malla.num;
        let rodaje = self.t_myr - self.t0_myr <= self.cfg.t_rodaje_myr;
        let omegas: Vec<V3> = self.placas.iter().map(|p| p.omega).collect();
        let v_rms = self.v_rms_de(&omegas) / CM_ANIO;
        self.ultima_v_rms = v_rms;
        let (mut dors, mut fosas, mut col) = (0.0, 0.0, 0.0);
        for b in &self.bordes {
            match b.tipo {
                TipoBorde::Divergente => dors += 0.5 * b.largo_km,
                TipoBorde::Subduccion if b.p_subduce => fosas += b.largo_km,
                TipoBorde::Colision => col += 0.5 * b.largo_km,
                _ => {}
            }
        }
        let (mut a_oc, mut edad_a, mut edad_max, mut a_cont) = (0.0, 0.0, 0.0f64, 0.0);
        for k in 0..num {
            if let Some(c) = self.corteza(k) {
                if c.cont {
                    a_cont += self.malla.area[k];
                } else {
                    a_oc += self.malla.area[k];
                    edad_a += c.edad_oc * self.malla.area[k];
                    edad_max = edad_max.max(c.edad_oc);
                }
            }
        }
        let sc = self.indice_supercontinente();
        if sc >= 0.7 && !self.super_formado {
            self.super_formado = true;
            self.evento("supercontinente_formado", vec![], 0.0, a_cont, None);
        } else if sc < 0.7 && self.super_formado {
            self.super_formado = false;
            self.evento("supercontinente_roto", vec![], 0.0, a_cont, None);
        }
        self.eventos_de_inicio();
        if !rodaje {
            self.v_rms_acum.0 += v_rms;
            self.v_rms_acum.1 += 1;
            if let Some((rf, rc)) = self.forsyth_uyeda() {
                self.fu_fosa.0 += rf;
                self.fu_fosa.1 += 1;
                self.fu_cont.0 += rc;
                self.fu_cont.1 += 1;
            }
        }
        let a_tot = self.malla.area_total();
        self.metricas.push(MetricaMyr {
            myr: self.t_myr,
            rodaje,
            n_placas: self.placas.iter().filter(|p| p.viva).count() as u32,
            v_rms_cm_anio: v_rms,
            dorsales_km: dors,
            fosas_km: fosas,
            colisiones_km: col,
            edad_media_oceano_myr: if a_oc > 0.0 { edad_a / a_oc } else { 0.0 },
            edad_max_oceano_myr: edad_max,
            volumen_continental_km3: self.volumen_continental(),
            area_continental_frac: a_cont / a_tot,
            supercontinente: sc,
            nivel_mar_m: self.zeta_m,
        });
    }

    /// Eventos de inicio de subducción y de colisión entre pares de placas.
    fn eventos_de_inicio(&mut self) {
        let mut subd: Vec<(u64, u64, f64, V3)> = Vec::new();
        let mut colis: Vec<(u64, u64, f64, V3)> = Vec::new();
        for b in &self.bordes {
            let (ip, iq) = (self.placas[b.p as usize].id, self.placas[b.q as usize].id);
            let (x, y) = if ip < iq { (ip, iq) } else { (iq, ip) };
            let lista = match b.tipo {
                TipoBorde::Subduccion if b.p_subduce => &mut subd,
                TipoBorde::Colision => &mut colis,
                _ => continue,
            };
            match lista.iter_mut().find(|e| e.0 == x && e.1 == y) {
                Some(e) => e.2 += b.largo_km,
                None => lista.push((x, y, b.largo_km, self.malla.dir[b.k as usize])),
            }
        }
        for i in 0..self.pares.len() {
            let (a, b) = (self.pares[i].a, self.pares[i].b);
            let s = subd.iter().find(|e| e.0 == a && e.1 == b).copied();
            let c = colis.iter().find(|e| e.0 == a && e.1 == b).copied();
            if let Some((_, _, l, d)) = s
                && !self.pares[i].habia_subduccion
            {
                self.evento("inicio_subduccion", vec![a, b], l, 0.0, Some(d));
            }
            if let Some((_, _, l, d)) = c
                && !self.pares[i].habia_colision
            {
                self.evento("inicio_colision", vec![a, b], 0.5 * l, 0.0, Some(d));
            }
            self.pares[i].habia_subduccion = s.is_some();
            self.pares[i].habia_colision = c.is_some();
            if c.is_none() && self.t_myr - self.pares[i].choque_visto_myr > 20.0 {
                self.pares[i].choque = None;
            }
            if s.is_none() && self.t_myr - self.pares[i].contacto_myr > 20.0 {
                self.pares[i].polaridad_oo = None;
            }
        }
        // Pares nuevos que aún no tienen registro.
        for (a, b, l, d) in subd {
            let i = self.par_idx(a, b);
            if !self.pares[i].habia_subduccion {
                self.pares[i].habia_subduccion = true;
                self.evento("inicio_subduccion", vec![a, b], l, 0.0, Some(d));
            }
        }
        for (a, b, l, d) in colis {
            let i = self.par_idx(a, b);
            if !self.pares[i].habia_colision {
                self.pares[i].habia_colision = true;
                self.evento("inicio_colision", vec![a, b], 0.5 * l, 0.0, Some(d));
            }
        }
        // Olvidar los pares de placas que ya no existen.
        let vivas: Vec<u64> = {
            let mut v: Vec<u64> = self
                .placas
                .iter()
                .filter(|p| p.viva)
                .map(|p| p.id)
                .collect();
            v.sort_unstable();
            v
        };
        self.pares
            .retain(|p| vivas.binary_search(&p.a).is_ok() && vivas.binary_search(&p.b).is_ok());
    }

    /// Correlaciones de Spearman de la velocidad de cada placa con su fracción de borde en fosa y
    /// con su fracción continental (Forsyth y Uyeda, 1975).
    fn forsyth_uyeda(&self) -> Option<(f64, f64)> {
        let n = self.placas.len();
        let mut s2 = vec![0.0; n];
        let mut a = vec![0.0; n];
        let mut ac = vec![0.0; n];
        for k in 0..self.malla.num {
            let p = self.dueno[k];
            if p == NADA {
                continue;
            }
            let v = velocidad(
                self.placas[p as usize].omega,
                self.malla.dir[k],
                self.malla.r_km,
            );
            s2[p as usize] += dot(v, v) * self.malla.area[k];
            a[p as usize] += self.malla.area[k];
            if self.corteza(k).is_some_and(|c| c.cont) {
                ac[p as usize] += self.malla.area[k];
            }
        }
        let mut fosa = vec![0.0; n];
        let mut borde = vec![0.0; n];
        for b in &self.bordes {
            borde[b.p as usize] += b.largo_km;
            if b.tipo == TipoBorde::Subduccion && b.p_subduce {
                fosa[b.p as usize] += b.largo_km;
            }
        }
        let idx: Vec<usize> = (0..n)
            .filter(|&i| self.placas[i].viva && a[i] > 0.0 && borde[i] > 0.0)
            .collect();
        if idx.len() < 4 {
            return None;
        }
        let v: Vec<f64> = idx.iter().map(|&i| (s2[i] / a[i]).sqrt()).collect();
        let f: Vec<f64> = idx.iter().map(|&i| fosa[i] / borde[i]).collect();
        let c: Vec<f64> = idx.iter().map(|&i| ac[i] / a[i]).collect();
        Some((spearman(&v, &f), spearman(&v, &c)))
    }
}

/// Correlación de Spearman (rangos con empates promediados).
pub fn spearman(x: &[f64], y: &[f64]) -> f64 {
    fn rangos(v: &[f64]) -> Vec<f64> {
        let mut idx: Vec<usize> = (0..v.len()).collect();
        idx.sort_by(|&a, &b| v[a].total_cmp(&v[b]).then(a.cmp(&b)));
        let mut r = vec![0.0; v.len()];
        let mut i = 0;
        while i < idx.len() {
            let mut j = i;
            while j + 1 < idx.len() && v[idx[j + 1]] == v[idx[i]] {
                j += 1;
            }
            let medio = 0.5 * (i + j) as f64;
            for &k in &idx[i..=j] {
                r[k] = medio;
            }
            i = j + 1;
        }
        r
    }
    let (rx, ry) = (rangos(x), rangos(y));
    let n = x.len() as f64;
    let (mx, my) = (rx.iter().sum::<f64>() / n, ry.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for i in 0..x.len() {
        let (dx, dy) = (rx[i] - mx, ry[i] - my);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx > 0.0 && syy > 0.0 {
        sxy / (sxx * syy).sqrt()
    } else {
        0.0
    }
}

/// Resuelve `K x = b` (denso, eliminación gaussiana con pivote parcial). Las filas sin pivote
/// (placa sin área) dan 0.
fn resolver(k: &mut [f64], b: &mut [f64], n: usize) -> Vec<f64> {
    for col in 0..n {
        let piv = (col..n)
            .max_by(|&a, &c| k[a * n + col].abs().total_cmp(&k[c * n + col].abs()))
            .unwrap_or(col);
        if k[piv * n + col].abs() < 1e-300 {
            continue;
        }
        if piv != col {
            for j in 0..n {
                k.swap(col * n + j, piv * n + j);
            }
            b.swap(col, piv);
        }
        let d = k[col * n + col];
        for r in col + 1..n {
            let f = k[r * n + col] / d;
            if f == 0.0 {
                continue;
            }
            for j in col..n {
                k[r * n + j] -= f * k[col * n + j];
            }
            b[r] -= f * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for r in (0..n).rev() {
        let d = k[r * n + r];
        if d.abs() < 1e-300 {
            continue;
        }
        let mut s = b[r];
        for j in r + 1..n {
            s -= k[r * n + j] * x[j];
        }
        x[r] = s / d;
    }
    x
}
