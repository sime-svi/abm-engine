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
use std::collections::{BinaryHeap, HashMap};

use super::historia::{
    CuentasCorteza, EventoTectonico, FotoSupercontinente, MargenRift, MetricaMyr,
};

mod diagnostico;
mod fosas;
mod rifts;
mod suavizado;
use super::isostasia;
use super::malla::{LADOS, Malla, Mat3, NADA, Quat, aplicar, aplicar_t, velocidad};
use crate::config::TectonicaCfg;
use crate::geometry::{V3, cross, dot, east_north, lat_lon, normalize};
use crate::noise::Perlin3;
use crate::statics::{hash01, splitmix64};
use fosas::Difusa;
pub use fosas::{ConteoFosas, EstadoFosa};
use suavizado::{TipoTramo, Vecindad};

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
    /// Espesor de la corteza cristalina (km): la base basáltica (si la hay) más la de tipo
    /// continental.
    pub h: f64,
    /// Espesor de la corteza de tipo continental (km, `h_c` de la adenda de balance): toda la de un
    /// continente, y en el fondo oceánico solo lo que han añadido los arcos y la acreción, sin los
    /// 7 km basálticos de base. Siempre `0 ≤ hc ≤ h`.
    pub hc: f64,
    /// Origen de arco (`a_i`, adenda v9): fracción de la corteza de tipo continental de la columna
    /// que hizo un arco que aún no se ha unido a un continente (0–1). Se mezcla por volumen.
    pub arco: f64,
    /// Espesor del sedimento (km), con su densidad (corrección C2): en los márgenes y, desde la
    /// adenda de balance, también sobre el fondo oceánico.
    pub sed: f64,
    /// Edad de la corteza oceánica (Myr).
    pub edad_oc: f64,
    /// Tiempo desde el último engrosamiento tectónico (Myr).
    pub edad_orog: f64,
    /// Ritmo de engrosamiento del paso (km/Myr): arcos y choques.
    pub dh_dt: f64,
    /// Sutura que bordea la muestra (nota de rifts §3): `2·id + lado`, con `id ≥ 1` la sutura
    /// ([`Deriva::suturas_hechas`]) y `lado` (0 o 1) la placa de la que venía; 0 si ninguna. Se
    /// pone al fundirse dos placas en las muestras de los dos lados del frente de choque y viaja con
    /// la muestra: las suturas son líneas dentro de las placas que un rift puede seguir.
    pub sutura: u32,
    /// Reloj del bloque continental (nota de rifts v2 §6.1): Myr en que el bloque de la muestra se
    /// rompió en un rift o se soldó a otro continente por última vez; `−∞` si ninguno (la corteza
    /// nueva no cambia el reloj de su bloque). Viaja con la muestra: el reloj de un bloque es el
    /// tiempo desde el reinicio más reciente de sus muestras (al juntarse dos bloques, el menor).
    pub reinicio_myr: f64,
    /// Edad tectonotérmica (Myr; nota de isostasia con litosfera §3): tiempo desde el último evento
    /// que calentó la columna (rift, arco, choque o delaminación). De ella sale el grosor de su
    /// litosfera. Se mezcla por volumen cuando se junta corteza de varias procedencias.
    pub edad_tt: f64,
    /// Margen de rift al que pertenece la muestra (diagnóstico, el modelo no lo lee): `2·id + lado`,
    /// con `id ≥ 1` el rift en el orden en que empezaron; 0 si ninguno.
    pub rift_id: u32,
    /// De dónde viene la muestra (diagnóstico; el modelo nunca la lee).
    pub proc: Procedencia,
}

/// Cómo nació una muestra ([`Procedencia::nace`]).
pub mod nace {
    /// Estado inicial del planeta.
    pub const INICIAL: u8 = 0;
    /// Fondo nuevo en un hueco de un borde divergente (dorsal).
    pub const DORSAL: u8 = 1;
    /// Fondo nuevo en un hueco de un tramo transformante o convergente (creación residual).
    pub const HUECO: u8 = 2;
    /// Corteza continental estirada en la abertura de un rift (adenda v8 §6.10).
    pub const RIFT: u8 = 3;
    /// Fondo que dejó una columna continental al perder toda su corteza continental.
    pub const SIN_CONTINENTE: u8 = 4;
}

/// Tipo de un lote ([`Procedencia::lote`], [`Deriva::lotes`]): el evento que juntó la corteza de una
/// placa con la de otra o la separó de su madre.
pub mod lote {
    pub const NINGUNO: u8 = 0;
    pub const SUTURA: u8 = 1;
    pub const ABSORCION: u8 = 2;
    pub const SALTO_DORSAL: u8 = 3;
    pub const RIFT: u8 = 4;
    /// Movimientos de una sola muestra a otro sitio u otra placa: un diente recolocado, una isla
    /// absorbida por su vecina y el suavizado de la pertenencia de los bordes oceánicos.
    pub const DIENTE: u8 = 5;
    pub const ISLA: u8 = 6;
    pub const SUAVIZADO: u8 = 7;
    /// Separación de un océano por la invasión de un margen pasivo (v4 §6.4).
    pub const INVASION: u8 = 8;
    /// Captura de una microplaca (nota de demografía de placas, A2) y franja de una placa que pasa a
    /// la del otro lado de su dorsal cuando la dorsal llega a la fosa (rama D).
    pub const CAPTURA: u8 = 9;
    pub const DORSAL_FOSA: u8 = 10;
    pub const NOMBRES: [&str; 11] = [
        "ninguno",
        "sutura",
        "absorción",
        "salto de dorsal",
        "rift",
        "diente recolocado",
        "isla absorbida",
        "suavizado de la pertenencia",
        "invasión",
        "captura de microplaca",
        "subducción de dorsal",
    ];
}

/// Vía por la que llegó a una columna su corteza de tipo continental ([`Procedencia::vias`]).
pub mod via {
    pub const ARCO: usize = 0;
    pub const ACRECION: usize = 1;
    /// Sedimento que pasa a corteza al sumarle corteza encima.
    pub const SEDIMENTO: usize = 2;
    pub const CHOQUE: usize = 3;
    pub const FLUJO: usize = 4;
    pub const DIFUSA: usize = 5;
    /// Dientes de los bordes recolocados en otra placa.
    pub const DIENTES: usize = 6;
    /// Islas de una placa absorbidas por su vecina al suavizar los bordes.
    pub const ISLAS: usize = 7;
    /// Muestras de una placa fundida que no encuentran sitio en la otra.
    pub const FUSION: usize = 8;
    pub const INICIAL: usize = 9;
    pub const RIFT: usize = 10;
    pub const N: usize = 11;
    pub const NOMBRES: [&str; N] = [
        "arco",
        "acreción",
        "sedimento",
        "choque",
        "flujo lateral",
        "difusa",
        "dientes",
        "islas",
        "fusión",
        "inicial",
        "rift",
    ];
}

/// Procedencia de una muestra: diagnóstico de la nota de fosas v6 y de la adenda v8. Viaja con la
/// muestra, pero ninguna regla del modelo la lee.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Procedencia {
    /// Cómo nació ([`nace`]).
    pub nace: u8,
    /// Último evento que juntó su corteza con la de otra placa o la separó de su placa madre:
    /// índice en [`Deriva::lotes`] (0, ninguno).
    pub lote: u32,
    /// Corteza de tipo continental (km) por la vía que la puso en la columna ([`via`]); suma `hc`.
    pub vias: [f32; via::N],
}

impl Procedencia {
    /// La misma procedencia con su corteza de tipo continental multiplicada por `f`.
    pub fn escalada(mut self, f: f64) -> Self {
        for v in &mut self.vias {
            *v = (*v as f64 * f) as f32;
        }
        self
    }
}

impl Corteza {
    pub fn oceanica(h_oc: f64) -> Self {
        Self {
            cont: false,
            h: h_oc,
            hc: 0.0,
            arco: 0.0,
            sed: 0.0,
            edad_oc: 0.0,
            edad_orog: 0.0,
            dh_dt: 0.0,
            sutura: 0,
            reinicio_myr: f64::NEG_INFINITY,
            edad_tt: 0.0,
            rift_id: 0,
            proc: Procedencia::default(),
        }
    }

    /// Corteza continental de espesor `h` (km), sin sedimento.
    pub fn continental(h: f64) -> Self {
        let mut proc = Procedencia::default();
        proc.vias[via::INICIAL] = h as f32;
        Self {
            cont: true,
            h,
            hc: h,
            arco: 0.0,
            sed: 0.0,
            edad_oc: 0.0,
            edad_orog: 1000.0,
            dh_dt: 0.0,
            sutura: 0,
            reinicio_myr: f64::NEG_INFINITY,
            edad_tt: 0.0,
            rift_id: 0,
            proc,
        }
    }

    /// Diagnóstico: cambia `hc` en `dh` km por la vía `v` (si suma) o quita en proporción (si
    /// resta). Solo lleva la cuenta: quien llama cambia `hc`.
    #[inline]
    fn anotar_hc(&mut self, hc_antes: f64, dh: f64, v: usize) {
        if dh >= 0.0 {
            self.proc.vias[v] += dh as f32;
        } else if hc_antes > 0.0 {
            self.proc = self.proc.escalada(((hc_antes + dh) / hc_antes).max(0.0));
        }
    }

    /// Espesor equivalente en roca (km): el sedimento cuenta con su masa.
    #[inline]
    pub fn h_eq(&self, rho_s: f64, rho_c: f64) -> f64 {
        self.h + self.sed * rho_s / rho_c
    }

    /// Material continental de la columna por unidad de área (km de roca): `h_c + S·ρ_s/ρ_c`.
    #[inline]
    pub fn v_cont(&self, rho_s: f64, rho_c: f64) -> f64 {
        self.hc + self.sed * rho_s / rho_c
    }

    /// La columna es sobre todo de arco: más de la mitad de su corteza de tipo continental la hizo
    /// un arco que aún no se ha unido a un continente.
    #[inline]
    pub fn de_arco(&self) -> bool {
        self.arco >= 0.5
    }

    /// Corteza continental que no es sobre todo de arco: la de un continente.
    #[inline]
    pub fn continente(&self) -> bool {
        self.cont && !self.de_arco()
    }

    /// Añade `dh` km de corteza de tipo continental con una fracción de arco `f_arco` (adenda v9):
    /// 1 lo que hace un arco; 0 la acreción de sedimento y lo que deja un choque sobre un
    /// continente; la de la corteza que llega cuando solo se traslada (dientes, islas, fusiones,
    /// convergencia difusa) o cuando un choque la deja fuera de un continente. El sedimento de
    /// encima se incorpora a la corteza (se conserva su masa), con fracción de arco 0. La fracción
    /// de la columna se mezcla por volumen, `a ← (a·h_c + f_arco·dh)/(h_c + roca + dh)`. La
    /// oceánica que supera `h_cont_min` pasa a continental. Devuelve `true` si la columna acaba de
    /// pasar a continental.
    ///
    /// La edad tectonotérmica se mezcla por volumen con la del aporte, `edad_aporte` (nota de
    /// isostasia con litosfera §6.3): la de la corteza que llega, o 0 si es de un arco o de un choque.
    /// Una columna oceánica que pasa a continental se queda con la edad de su corteza de tipo
    /// continental.
    #[allow(clippy::too_many_arguments)]
    fn sumar_hc(
        &mut self,
        dh: f64,
        f_arco: f64,
        edad_aporte: f64,
        rs: f64,
        rc: f64,
        h_cont_min: f64,
        v: usize,
    ) -> bool {
        if dh <= 0.0 {
            return false;
        }
        let v_arco = self.arco * self.hc + f_arco * dh;
        if self.sed > 0.0 {
            let roca = self.sed * rs / rc;
            self.h += roca;
            self.hc += roca;
            self.sed = 0.0;
            self.proc.vias[via::SEDIMENTO] += roca as f32;
        }
        let antes = self.hc;
        self.edad_tt = if antes > 0.0 {
            (self.edad_tt * antes + edad_aporte * dh) / (antes + dh)
        } else {
            edad_aporte
        };
        self.h += dh;
        self.hc += dh;
        self.arco = (v_arco / self.hc).clamp(0.0, 1.0);
        self.proc.vias[v] += dh as f32;
        if !self.cont && self.h > h_cont_min {
            self.cont = true;
            return true;
        }
        false
    }

    /// Quita `dh ≤ hc` km de corteza de tipo continental. Si se queda sin ella, la columna pasa a
    /// fondo oceánico de edad `edad` (como mínimo la base basáltica `h_oc`). Devuelve `true` si
    /// perdió así su corteza continental.
    fn quitar_hc(&mut self, dh: f64, h_oc: f64, edad: f64) -> bool {
        let dh = dh.min(self.hc);
        let antes = self.hc;
        self.h -= dh;
        self.hc -= dh;
        self.anotar_hc(antes, -dh, 0);
        if self.hc > 1e-9 {
            return false;
        }
        self.hc = 0.0;
        self.arco = 0.0;
        self.proc.vias = [0.0; via::N];
        let era_cont = self.cont;
        if era_cont {
            self.cont = false;
            self.h = self.h.max(h_oc);
            self.edad_oc = edad;
            self.proc.nace = nace::SIN_CONTINENTE;
        }
        era_cont
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
    /// Estado de los tramos convergentes en las celdas de borde de la placa (nota de fosas §3):
    /// celda local → estado. Viaja con la placa.
    pub fosas: HashMap<u32, EstadoFosa>,
    /// Ids de las placas de las que viene (por rifts, saltos de dorsal o suturas): el estado de un
    /// tramo de otra placa que mire a una de ellas mira a esta.
    pub ancestros: Vec<u64>,
    /// Frente de invasión de sus márgenes pasivos (nota de fosas v4 §6.4): celda local continental
    /// del margen → lo que hereda de la fosa que la invade. La placa aún no se ha roto ahí.
    pub invadidas: HashMap<u32, Invadida>,
    /// Longitud invadida (km) en el último intento de separar el océano de delante.
    pub inv_intento_km: f64,
    /// Tiempo seguido sin ninguna fosa activa propia, ni como placa que baja ni como placa de arriba
    /// (`T_sf`, nota de demografía de placas A2, Myr).
    pub sin_fosa_myr: f64,
}

/// Una celda de margen pasivo por la que ya ha pasado el frente de una fosa que invade (v4 §6.4):
/// la losa y la edad de esa fosa, y el último paso en que el frente la tocó.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Invadida {
    pub l_km: f64,
    pub edad_myr: f64,
    pub paso: u64,
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
            fosas: HashMap::new(),
            ancestros: Vec::new(),
            invadidas: HashMap::new(),
            inv_intento_km: 0.0,
            sin_fosa_myr: 0.0,
        }
    }

    /// Placa vacía con la misma orientación y velocidad que `madre` (hijas de rift).
    fn hermana(madre: &Placa, id: u64, t: f64) -> Self {
        let mut p = Placa::nueva(id, madre.mapa.len(), t);
        p.q = madre.q;
        p.rot = madre.rot;
        p.omega = madre.omega;
        p.ancestros = madre.ancestros.clone();
        p.ancestros.push(madre.id);
        p.recortar_ancestros();
        p
    }

    /// Solo hacen falta los antepasados recientes: el estado de un tramo que mira a una placa ya
    /// partida o fundida se reescribe con el id nuevo en el paso siguiente.
    fn recortar_ancestros(&mut self) {
        const MAX: usize = 16;
        if self.ancestros.len() > MAX {
            let quitar = self.ancestros.len() - MAX;
            self.ancestros.drain(..quitar);
        }
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

/// Régimen de un margen (adenda de balance §6.2): erosivo si converge a más de `v_ero` y le llega
/// menos de `S_ero` de sedimento; acrecionario en cualquier otro caso.
pub fn margen_erosivo(cfg: &TectonicaCfg, v_cm_anio: f64, s_fosa_km: f64) -> bool {
    v_cm_anio > cfg.v_ero_cm_anio && s_fosa_km < cfg.s_ero_km
}

/// Delaminación de una columna (adenda de balance §6.6): espesor de tipo continental tras `dt` Myr,
/// `h_del + (h_c − h_del)·e^(−Δt/τ_del)` por encima de `h_del`. La forma exponencial exacta es
/// estable con cualquier paso y nunca baja de `h_del`.
pub fn tras_delaminar(hc: f64, h_del: f64, dt: f64, tau: f64) -> f64 {
    if hc <= h_del {
        hc
    } else {
        h_del + (hc - h_del) * (-dt / tau).exp()
    }
}

/// Tipo de un borde de placa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoBorde {
    Divergente,
    Subduccion,
    Colision,
    Transformante,
    /// Convergencia de corteza oceánica sin fosa (nota de fosas §6.2): se absorbe engrosando.
    Difusa,
}

impl TipoBorde {
    pub fn codigo(self) -> u8 {
        match self {
            TipoBorde::Divergente => 1,
            TipoBorde::Subduccion => 2,
            TipoBorde::Colision => 3,
            TipoBorde::Transformante => 4,
            TipoBorde::Difusa => 5,
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
    /// Módulo de la velocidad relativa de `p` respecto a `q` en la celda (km/Myr).
    pub v_rel: f64,
    pub tipo: TipoBorde,
    /// En una subducción: `p` es la placa que se hunde.
    pub p_subduce: bool,
    /// Fracción del tirón de la losa (`min(1, L_j/L_losa)`, nota de fosas §6.5) en el lado que se
    /// hunde de una fosa activa; 0 en el resto.
    pub losa: f64,
    /// En una fosa que ya choca, la edad del fondo que se hundía (Myr), para el tirón de la losa
    /// que aún cuelga; 0 en el resto (vale la del fondo de la celda).
    pub edad_losa: f64,
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
    /// Último Myr con contacto convergente entre las dos.
    contacto_myr: f64,
    choque_visto_myr: f64,
    sutura_myr: f64,
    habia_subduccion: bool,
    habia_colision: bool,
    /// Choque en curso (adenda de balance §7): corteza continental solapada, la de arco y la
    /// perdida al manto (km³), último Myr con solape y si ya se avisó de su inicio.
    choque_solapado: f64,
    choque_arco: f64,
    choque_perdido: f64,
    choque_ultimo_myr: f64,
    choque_avisado: bool,
    /// Myr del primer solape del choque en curso (su duración, nota de fosas v4 §8).
    choque_inicio_myr: f64,
    /// Régimen del margen en el último paso con subducción: (id de la placa de arriba, ¿erosivo?).
    regimen: Option<(u64, bool)>,
}

/// Un tramo de fosa del paso (adenda de balance §3): la celda del mundo donde la placa de abajo
/// desaparece bajo la de arriba, con lo que se ha hundido allí. El área la da exacta el remuestreo.
#[derive(Debug, Clone, Copy)]
struct Tramo {
    k: u32,
    /// Fondo oceánico consumido (km², `A_sub,j`).
    a_sub: f64,
    /// Sedimento que llega (km³ de roca), `Σ S·A` (km³ de sedimento) y `Σ edad·A`.
    v_llega: f64,
    s_area: f64,
    edad_area: f64,
    /// Placa que se hunde (índice; la de más área si hay varias) y su área.
    placa: u32,
    a_placa: f64,
}

/// Corteza continental de la placa de abajo que engruesa a la de arriba en un choque (§6.5).
#[derive(Debug, Clone, Copy)]
struct Choque {
    /// Celda del frente (de la placa de arriba).
    k: u32,
    /// Lo que se queda arriba (`η·V_sol`, km³).
    vol: f64,
    /// La fracción `η` con que se quedó (adenda v7 §6.5).
    eta: f64,
    /// Lo que se queda arriba de la parte de arco (`a_i·η_arco·V_sol`, km³, adenda v9).
    vol_arco: f64,
}

/// Detalle opcional de un evento (adenda de balance §7).
#[derive(Debug, Clone, Default)]
struct Detalle {
    clase: Option<String>,
    eta: Option<f64>,
    solapado_km3: Option<f64>,
    perdido_km3: Option<f64>,
    edad_fondo_myr: Option<f64>,
    edad_fosa_myr: Option<f64>,
    losa_km: Option<f64>,
    tiempo_choque_myr: Option<f64>,
    trozos_km2: Option<[f64; 2]>,
    sutura_km: Option<f64>,
    extension_km: Option<f64>,
    fosa_km: Option<f64>,
}

/// Fracción suavizada por debajo de la cual una muestra que reclama una celda es una muestra suelta
/// de su placa dentro de otra (criterio numérico de la representación: una celda de unas 26 a menos
/// de L_borde da ~0,04; un borde real, 0,3–0,7).
const PHI_SUELTA: f64 = 0.15;

/// Clases de edad tectonotérmica de las métricas (Myr; nota de isostasia con litosfera §8): 0–100,
/// 100–500, 500–1 600, 1 600–2 500 y más de 2 500.
pub const CLASES_EDAD_TT_MYR: [f64; 5] = [0.0, 100.0, 500.0, 1600.0, 2500.0];

/// Clases del tiempo desde la rotura de los márgenes estirados (Myr): 0–20, 20–50, 50–80, 80–120,
/// 120–200 y más de 200.
pub const CLASES_MARGEN_MYR: [f64; 6] = [0.0, 20.0, 50.0, 80.0, 120.0, 200.0];

/// Límites (cm/año) de las clases de la convergencia perpendicular en las fosas (diagnóstico de la
/// nota de fosas §0); la última clase no tiene tope.
pub const CLASES_CONVERGENCIA_CM_ANIO: [f64; 10] =
    [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0, 10.0, 15.0];

/// Una etapa de la traza de diagnóstico de [`Deriva::dar_paso`]: espesor continental máximo de las
/// muestras de las placas al acabarla (km) y su tiempo de pared (s; solo rendimiento, CLAUDE §6).
#[derive(Debug, Clone, Copy)]
pub struct EtapaTraza {
    pub etapa: &'static str,
    pub h_max_km: f64,
    /// La muestra continental más gruesa (índice de placa, celda local).
    pub max_en: (u32, u32),
    /// La muestra vigilada (`Deriva::vigilada`) al acabar la etapa, si sigue viva.
    pub vigilada: Option<Corteza>,
    /// Celdas del mundo sin corteza (sin dueño o sin muestra) al acabar la etapa.
    pub huecos: u32,
    pub segundos: f64,
}

/// Red del flujo lateral de un paso ([`Deriva::flujo_lateral`]): las muestras que intervienen (su
/// índice no cambia en el paso), su espesor de tipo continental y lo que reciben en el subpaso; el
/// orden de (placa, celda local) en que se aplican los cambios; los lados de cada celda fuente hacia
/// sus vecinas (los grupos, en el orden de las celdas fuente, y sus lados en el de `LADOS`) y las
/// parejas de muestra visible y oculta de una celda.
///
/// Dentro del paso no cambian los dueños, los representantes ni qué muestras existen, así que los
/// lados de una celda fuente son siempre los mismos: cuando entran fuentes nuevas, la red se amplía
/// con ellas en vez de rehacerse entera, y queda exactamente igual que rehecha.
#[derive(Default)]
struct RedFlujo {
    muestras: Vec<(u32, u32)>,
    /// Posición de cada muestra en su placa (`Placa::cort`): no cambia dentro del flujo.
    pos: Vec<u32>,
    indice: HashMap<(u32, u32), u32>,
    orden: Vec<u32>,
    hc: Vec<f64>,
    delta: Vec<f64>,
    /// Fracción de arco de cada muestra y cambio de su volumen de arco en el subpaso (adenda v9: el
    /// flujo transporta la fracción con el volumen).
    arco: Vec<f64>,
    delta_arco: Vec<f64>,
    /// Edad tectonotérmica de cada muestra y cambio de su volumen por edad en el subpaso (nota de
    /// isostasia con litosfera §6.3: la edad viaja con el volumen y se mezcla por él).
    edad: Vec<f64>,
    delta_edad: Vec<f64>,
    grupos: Vec<GrupoFlujo>,
    lados: Vec<LadoFlujo>,
    parejas: Vec<(u32, u32)>,
    /// Celdas del mundo ya vistas como fuente y parejas ya vistas de la lista de ocultas.
    en_red: Vec<bool>,
    ocultas_vistas: usize,
}

/// Una celda fuente (la celda del mundo `k`): su muestra (índice en la red) y el tramo de sus lados.
struct GrupoFlujo {
    k: u32,
    i: u32,
    desde: u32,
    hasta: u32,
}

/// Un lado de una celda fuente: la muestra de la vecina, la distancia entre centros, la longitud del
/// lado común y la celda vecina (si recibe, puede pasar a fuente).
struct LadoFlujo {
    j: u32,
    d: f64,
    l: f64,
    k2: u32,
}

/// Una muestra: placa y celda local.
type Muestra = (u32, u32);

impl RedFlujo {
    /// Índice de la muestra `m` en la red; si no está, se añade con su espesor de ahora.
    fn muestra(&mut self, d: &Deriva, m: Muestra) -> u32 {
        if let Some(&i) = self.indice.get(&m) {
            return i;
        }
        let i = self.muestras.len() as u32;
        let c = d.placas[m.0 as usize].get(m.1);
        self.muestras.push(m);
        self.pos.push(d.placas[m.0 as usize].mapa[m.1 as usize]);
        self.indice.insert(m, i);
        self.hc.push(c.map_or(0.0, |c| c.hc));
        self.arco.push(c.map_or(0.0, |c| c.arco));
        self.edad.push(c.map_or(0.0, |c| c.edad_tt));
        self.delta.push(0.0);
        self.delta_arco.push(0.0);
        self.delta_edad.push(0.0);
        let muestras = &self.muestras;
        let pos = self.orden.partition_point(|&x| muestras[x as usize] < m);
        self.orden.insert(pos, i);
        i
    }

    /// Añade a la red las celdas fuente y las parejas de ocultas que aún no tiene.
    fn ampliar(
        &mut self,
        d: &Deriva,
        fuente: &[u32],
        ocultas: &[(u32, u32, u32)],
        choque_con: &HashMap<u32, u32>,
    ) {
        let malla = &d.malla;
        if self.en_red.len() != malla.num {
            self.en_red = vec![false; malla.num];
        }
        let existe = |p: u32, l: u32| d.placas[p as usize].get(l).is_some();
        for &k in fuente {
            let ku = k as usize;
            if self.en_red[ku] {
                continue;
            }
            self.en_red[ku] = true;
            let (p, si) = (d.dueno[ku], d.rep[ku]);
            if p == NADA || si == NADA || !existe(p, si) {
                continue;
            }
            let i = self.muestra(d, (p, si));
            let desde = self.lados.len() as u32;
            for (o, &j) in LADOS.iter().enumerate() {
                let k2 = malla.vec[ku][j];
                if k2 == NADA {
                    continue;
                }
                let p2 = d.dueno[k2 as usize];
                let sutura = p2 != p
                    && p2 != NADA
                    && (choque_con.get(&k) == Some(&p2) || choque_con.get(&k2) == Some(&p));
                if (p2 != p && !sutura) || (p2 == p && d.rep[k2 as usize] == si) {
                    continue;
                }
                let s2 = d.rep[k2 as usize];
                if p2 == NADA || s2 == NADA || !existe(p2, s2) {
                    continue;
                }
                let jx = self.muestra(d, (p2, s2));
                self.lados.push(LadoFlujo {
                    j: jx,
                    d: malla.dvec[ku][j],
                    l: malla.lado_comun[ku][o],
                    k2,
                });
            }
            let g = GrupoFlujo {
                k,
                i,
                desde,
                hasta: self.lados.len() as u32,
            };
            let pos = self.grupos.partition_point(|x| x.k < k);
            self.grupos.insert(pos, g);
        }
        for &(p, lr, lh) in &ocultas[self.ocultas_vistas..] {
            if existe(p, lr) && existe(p, lh) {
                let a = self.muestra(d, (p, lr));
                let b = self.muestra(d, (p, lh));
                self.parejas.push((a, b));
            }
        }
        self.ocultas_vistas = ocultas.len();
    }
}

/// Una celda del mundo que una placa perdió en un solape.
#[derive(Debug, Clone, Copy)]
struct Perdida {
    k: u32,
    p: u32,
    l: u32,
    choque: bool,
    /// En un tramo convergente (se consume); si no, es un diente que se recoloca (nota de bordes
    /// §6.4).
    convergente: bool,
    /// En un tramo convergente difuso, sin fosa activa: se absorbe engrosando (nota de fosas §6.2).
    difusa: bool,
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

/// Un encuentro entre una dorsal y una fosa (nota de demografía de placas v2, D6): un tramo de
/// dorsal activa entre las placas `p` y `r` (ids) a menos de `d_dc` de una fosa activa donde se
/// hunde `p`, medido por dentro de `p`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EncuentroDorsalFosa {
    pub p: u64,
    pub r: u64,
    /// Longitud de la dorsal a menos de `d_dc` de la fosa (km) y su distancia mínima a ella (km).
    pub dorsal_km: f64,
    pub dist_min_km: f64,
}

/// Edad (Myr) a la que es más vieja la fracción `q` del área de la distribución (`bordes`, `frac`):
/// uniforme dentro de cada clase.
fn edad_de_cuantil(bordes: &[f64], frac: &[f64], q: f64) -> f64 {
    let total: f64 = frac.iter().sum();
    let mut acum = 0.0;
    for b in (0..frac.len()).rev() {
        let f = frac[b] / total;
        if f > 0.0 && acum + f >= q {
            let (lo, hi) = (bordes[b], bordes[b + 1]);
            return hi - (q - acum) / f * (hi - lo);
        }
        acum += f;
    }
    bordes[0]
}

/// Edades tectonotérmicas de la corteza continental de partida (nota de isostasia con litosfera
/// §6.5): dentro de cada continente (componente conexa de `es_cont`), las celdas se ordenan de la más
/// interior a la más costera por su distancia a la costa (por dentro del continente; desempate por
/// índice), y la fracción `q` de área que queda por dentro de cada una le da la edad de ese cuantil
/// de la distribución de la config (`edad_tt_ini_*`): las más viejas, en el interior, como crecen
/// los continentes, añadiendo corteza por los bordes. Las celdas que no son continentales, 0.
pub fn edades_tt_iniciales(cfg: &TectonicaCfg, malla: &Malla, es_cont: &[bool]) -> Vec<f64> {
    use std::cmp::Reverse;
    let num = malla.num;
    let mut edad = vec![0.0; num];
    // Distancia a la costa (km): Dijkstra desde las celdas continentales con alguna vecina que no lo
    // es, por las continentales.
    let mut dist = vec![f64::INFINITY; num];
    let mut heap: BinaryHeap<Reverse<(u64, u32)>> = BinaryHeap::new();
    let clave = |d: f64| (d * 1e6) as u64;
    for k in 0..num {
        if es_cont[k]
            && malla.vec[k]
                .iter()
                .any(|&v| v != NADA && !es_cont[v as usize])
        {
            dist[k] = 0.5 * malla.lado_km;
            heap.push(Reverse((clave(dist[k]), k as u32)));
        }
    }
    while let Some(Reverse((dk, k))) = heap.pop() {
        let ku = k as usize;
        if dk > clave(dist[ku]) {
            continue;
        }
        for (j, &v) in malla.vec[ku].iter().enumerate() {
            if v == NADA || !es_cont[v as usize] {
                continue;
            }
            let nd = dist[ku] + malla.dvec[ku][j];
            if nd < dist[v as usize] {
                dist[v as usize] = nd;
                heap.push(Reverse((clave(nd), v)));
            }
        }
    }
    // Componentes conexas y reparto por cuantiles de área desde el interior.
    let mut visto = vec![false; num];
    let mut pila: Vec<u32> = Vec::new();
    for k0 in 0..num {
        if !es_cont[k0] || visto[k0] {
            continue;
        }
        visto[k0] = true;
        pila.push(k0 as u32);
        let mut comp: Vec<u32> = Vec::new();
        while let Some(k) = pila.pop() {
            comp.push(k);
            for &v in &malla.vec[k as usize] {
                if v != NADA && es_cont[v as usize] && !visto[v as usize] {
                    visto[v as usize] = true;
                    pila.push(v);
                }
            }
        }
        comp.sort_unstable_by(|&a, &b| {
            dist[b as usize]
                .total_cmp(&dist[a as usize])
                .then(a.cmp(&b))
        });
        let total: f64 = comp.iter().map(|&k| malla.area[k as usize]).sum();
        let mut acum = 0.0;
        for &k in &comp {
            let a = malla.area[k as usize];
            let q = ((acum + 0.5 * a) / total).clamp(0.0, 1.0);
            edad[k as usize] =
                edad_de_cuantil(&cfg.edad_tt_ini_bordes_myr, &cfg.edad_tt_ini_frac, q);
            acum += a;
        }
    }
    edad
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
    /// Fondo oceánico consumido en el último paso (km²) y la parte por márgenes erosivos.
    pub area_subducida_km2: f64,
    pub area_erosiva_km2: f64,
    /// Bordes del último paso (nota de bordes §8): dientes recolocados en un hueco de su placa y
    /// sin hueco (esperan ocultos en su placa; adenda v5), creación residual en huecos que no son
    /// de dorsal, área reasignada por el suavizado (km²) y material continental de los dientes
    /// (km³).
    pub dientes_recolocados_km2: f64,
    pub dientes_sin_hueco_km2: f64,
    /// Muestras sueltas lejos de su placa cuyo material pasa a la placa donde están (km²).
    pub dientes_a_otra_placa_km2: f64,
    pub creacion_residual_km2: f64,
    pub area_reasignada_km2: f64,
    /// Islas absorbidas por la placa que las rodea en el último paso (km²).
    pub area_islas_km2: f64,
    pub dientes_continentales_km3: f64,
    /// Material continental de los dientes del último paso por su destino (km³): recolocados a
    /// menos de `L_borde`, recolocados en la segunda pasada, sumados a su placa y sumados a otra.
    pub dientes_cont_por_via_km3: [f64; 4],
    /// Nacimientos, muertes y convergencia difusa del último paso (nota de fosas §8).
    pub fosas_paso: ConteoFosas,
    /// Diagnóstico de procedencia (nota de fosas v6): tipo de cada lote ([`lote`]); el 0 es
    /// «ninguno».
    pub lotes: Vec<u8>,
    /// Diagnóstico de los choques (adenda v7 y v8): corteza de tipo continental solapada (0) y
    /// perdida (1) por la clase de la muestra que entra (continental < h_1, h_1–h_2, > h_2, sobre
    /// fondo oceánico; sin la de arco) y por la vía que la puso allí ([`via`]), km³.
    pub diag_choque_km3: [[[f64; via::N]; 4]; 2],
    /// Diagnóstico de la adenda v9: corteza de arco que los choques dejan arriba (`a_i·η_arco·V_sol`,
    /// km³) sobre un continente (0) y fuera de él, sobre fondo oceánico o sobre otro arco (1).
    pub diag_arco_choque_km3: [f64; 2],
    /// Lados de rift aún en fase de estiramiento (adenda v8 §6.10).
    rifts_activos: Vec<rifts::LadoRift>,
    /// Suturas hechas (nota de rifts §3): el Myr de cada una; su id es la posición más uno.
    pub suturas_hechas: Vec<f64>,
    /// Coste del flujo lateral (diagnóstico): subpasos, muestras de la red por subpaso (sumadas) y
    /// subpasos por clase del espesor máximo de la red (50–60, 60–70, 70–80, 80–90, 90–100, >100 km).
    pub flujo_subpasos: u64,
    pub flujo_muestras: u64,
    pub flujo_subpasos_por_hmax: [u64; 6],
    /// Veces que se rehace la red del flujo, lados recorridos y tiempo (s) en rehacerla y en los
    /// subpasos (diagnóstico del coste; el reloj no decide nada).
    pub flujo_rehechos: u64,
    /// Segundos en partes de la erosión y de las métricas (diagnóstico del coste): erosión, nivel
    /// del mar, sedimento al mar; márgenes pasivos, perfil de los márgenes, bloques, índice de
    /// supercontinente y el resto de las métricas.
    pub seg_partes: [f64; 8],
    pub flujo_lados: u64,
    pub flujo_seg: [f64; 2],
    /// Celdas del mundo de la costa de los márgenes pasivos del paso (trabajo de las métricas).
    costa_margen: Vec<u32>,
    /// Potencia de cada fuerza en cada placa en el último reparto de velocidades (nota de fosas
    /// §10, medida 3): índice de la placa, su área (km²) y `τ·ω` del tirón de la losa, el empuje de
    /// dorsal, la resistencia de los choques y el arrastre basal (en equilibrio suman cero).
    pub potencias: Vec<(u32, f64, [f64; 4])>,
    /// Espesor continental máximo (km) de las muestras que recibieron, en el último paso, dientes
    /// sin hueco de su placa y muestras sueltas de otra placa (diagnóstico de los picos).
    pub max_tras_dientes_km: [f64; 2],
    /// Traza de diagnóstico (pruebas): con `Some`, cada paso añade sus etapas. `None` no cuesta nada.
    pub traza: Option<Vec<EtapaTraza>>,
    traza_reloj: Option<std::time::Instant>,
    /// Muestra que la traza sigue etapa a etapa (índice de placa, celda local; diagnóstico).
    pub vigilada: Option<(u32, u32)>,
    /// Márgenes pasivos del paso anterior, (id de placa, celda local) → edad (Myr), ordenados:
    /// para saber cuáles terminan (nota de fosas v4 §8).
    margenes_previos: Vec<((u64, u32), f64)>,
    /// Consumo del último paso por la celda de borde a la que se asigna (celda, código del tipo de
    /// borde, km²) y solape de los choques por la celda del frente (celda, código, km³): las mismas
    /// cuentas que `area_subducida_por_borde_km2` y `choque_solapado_por_borde_km3`.
    pub consumo_en_borde_paso: Vec<(u32, u8, f64)>,
    pub solape_en_borde_paso: Vec<(u32, u8, f64)>,
    /// Longitud de fosa (km, sumada por pasos tras el rodaje) por clase de convergencia
    /// perpendicular ([`CLASES_CONVERGENCIA_CM_ANIO`]).
    pub fosas_por_convergencia_km: [f64; 10],
    /// Dónde se consumió fondo oceánico en el último paso (celda del frente y km²) y dónde nació
    /// (celdas con corteza nueva): para las pruebas de los bordes.
    pub consumo_paso: Vec<(u32, f64)>,
    /// La placa que se hunde en cada tramo del último paso (índice) y el fondo consumido (km²;
    /// diagnóstico de la nota de demografía de placas, D5).
    pub hundido_paso: Vec<(u32, f64)>,
    /// Encuentros entre dorsales y fosas del último paso (nota de demografía de placas v2, D6),
    /// también con la rama D apagada.
    pub dorsal_fosa_paso: Vec<EncuentroDorsalFosa>,
    /// Área continental cuya edad tectonotérmica volvió a 0 en el último paso, por causa: rift,
    /// arco, choque y delaminación (km²; nota de isostasia con litosfera §8).
    pub reinicio_km2_paso: [f64; 4],
    /// Márgenes pasivos por sus celdas de costa (longitud, edad media y máxima) y su perfil (corteza
    /// en la costa y franja estirada): se calculan cada 25 Myr y en el último paso, y se repiten
    /// entre medias (solo se usan sus valores finales y sus medias).
    ultimos_margenes: (f64, f64, f64),
    ultimo_perfil: (f64, f64),
    /// Los márgenes pasivos como entidades (diagnóstico; `ResumenTectonica::margenes_rift`) y el de
    /// cada `rift_id`.
    pub margenes_rift: Vec<MargenRift>,
    margen_de_rift: std::collections::BTreeMap<u32, usize>,
    /// Myr de la rotura de cada rift (por su número; NaN si aún no se ha roto; diagnóstico).
    pub rotura_rift_myr: Vec<f64>,
    /// Muestras de margen pasivo de la foto anterior del diagnóstico del supercontinente (id de su
    /// placa y celda local).
    margenes_foto: Vec<(u64, u32)>,
    /// Fotos del diagnóstico del supercontinente (cada 25 Myr).
    pub fotos_supercontinente: Vec<FotoSupercontinente>,
    /// Rifts empezados (el id del último; diagnóstico de los márgenes, `Corteza::rift_id`).
    pub rifts_empezados: u32,
    pub creacion_paso: Vec<u32>,
    // Trabajo (se reserva una vez).
    cov: Vec<[(u32, u32); MAXC]>,
    ncov: Vec<u8>,
    hueco: Vec<bool>,
    perdidas: Vec<Perdida>,
    campo: Campo,
    ruido_def: [Perlin3; 3],
    /// Vecindades de radio `L_borde` (borde suavizado) y tipo de tramo de cada celda en solape o en
    /// hueco del paso.
    vecindad: Vecindad,
    tipo_k: Vec<TipoTramo>,
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
        let t0 = -cfg.t_deriva_myr;
        // Reloj de los rifts (nota de rifts v2 §6.1): la nota no fija el de los cratones de partida;
        // cada uno empieza con un tiempo unido al azar en [0, τ_ais] (sin sincronizar sus rupturas).
        let reloj_craton: Vec<f64> = orden[..nc]
            .iter()
            .map(|&i| t0 - cfg.tau_ais_myr * azar(seed, 0, i as u64, P_CRATON, 2))
            .collect();
        let corteza = |k: usize| -> Corteza {
            let d = malla.dir[k];
            if es_cont[k] {
                let x = rh.fbm(d[0] * 6.0, d[1] * 6.0, d[2] * 6.0, 3, 0.5, 2.0) / std_h;
                // El cratón de la celda: el de más puntos (el mismo criterio que la reparte).
                let dd = deformar(d, 4.0 * cfg.deformacion, 3.0);
                let j = (0..nc)
                    .max_by(|&a, &b| {
                        let pa = -crate::geometry::angle_between(dd, cratones[a].0) / cratones[a].1;
                        let pb = -crate::geometry::angle_between(dd, cratones[b].0) / cratones[b].1;
                        pa.total_cmp(&pb).then(b.cmp(&a))
                    })
                    .unwrap_or(0);
                Corteza {
                    reinicio_myr: reloj_craton.get(j).copied().unwrap_or(f64::NEG_INFINITY),
                    ..Corteza::continental(cfg.h0_km + cfg.sigma_h0_km * x.clamp(-3.0, 3.0))
                }
            } else {
                let u = 0.5 + 0.5 * rf.fbm(d[0] * 4.0, d[1] * 4.0, d[2] * 4.0, 2, 0.5, 2.0) / 0.35;
                Corteza {
                    edad_oc: 150.0 * u.clamp(0.0, 1.0),
                    ..Corteza::oceanica(cfg.h_oc_km)
                }
            }
        };
        let mut cortezas: Vec<Corteza> = (0..malla.num).map(corteza).collect();
        // Edades tectonotérmicas de partida (nota de isostasia con litosfera §6.5).
        let edades = edades_tt_iniciales(cfg, &malla, &es_cont);
        for (c, &t) in cortezas.iter_mut().zip(&edades) {
            if c.cont {
                c.edad_tt = t;
            }
        }
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
        // El borde suavizado necesita al menos las vecinas: en una malla más gruesa que L_borde (las
        // de las pruebas), el radio efectivo es 1,5 lados de celda.
        let vecindad = Vecindad::nueva(&malla, cfg.l_borde_km.max(1.5 * malla.lado_km));
        let mut d = Self {
            cfg: cfg.clone(),
            seed,
            cov: vec![[(NADA, NADA); MAXC]; num],
            ncov: vec![0; num],
            hueco: vec![false; num],
            perdidas: Vec::new(),
            campo: Campo::nuevo(num),
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
            area_subducida_km2: 0.0,
            area_erosiva_km2: 0.0,
            dientes_recolocados_km2: 0.0,
            dientes_sin_hueco_km2: 0.0,
            dientes_a_otra_placa_km2: 0.0,
            creacion_residual_km2: 0.0,
            area_reasignada_km2: 0.0,
            area_islas_km2: 0.0,
            dientes_continentales_km3: 0.0,
            fosas_por_convergencia_km: [0.0; 10],
            max_tras_dientes_km: [0.0; 2],
            fosas_paso: ConteoFosas::default(),
            lotes: vec![lote::NINGUNO],
            diag_choque_km3: [[[0.0; via::N]; 4]; 2],
            diag_arco_choque_km3: [0.0; 2],
            rifts_activos: Vec::new(),
            suturas_hechas: Vec::new(),
            flujo_subpasos: 0,
            flujo_muestras: 0,
            flujo_subpasos_por_hmax: [0; 6],
            flujo_rehechos: 0,
            seg_partes: [0.0; 8],
            flujo_lados: 0,
            flujo_seg: [0.0; 2],
            costa_margen: Vec::new(),
            potencias: Vec::new(),
            dientes_cont_por_via_km3: [0.0; 4],
            traza: None,
            traza_reloj: None,
            vigilada: None,
            margenes_previos: Vec::new(),
            consumo_en_borde_paso: Vec::new(),
            solape_en_borde_paso: Vec::new(),
            consumo_paso: Vec::new(),
            hundido_paso: Vec::new(),
            dorsal_fosa_paso: Vec::new(),
            reinicio_km2_paso: [0.0; 4],
            ultimos_margenes: (0.0, 0.0, 0.0),
            ultimo_perfil: (0.0, 0.0),
            margenes_rift: Vec::new(),
            margen_de_rift: Default::default(),
            rotura_rift_myr: vec![f64::NAN],
            margenes_foto: Vec::new(),
            fotos_supercontinente: Vec::new(),
            rifts_empezados: 0,
            creacion_paso: Vec::new(),
            ruido_def,
            vecindad,
            tipo_k: Vec::new(),
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

    /// Un paso de la deriva (§9, G2 a–g), con el orden de la adenda de balance §9: remuestreo con el
    /// fondo consumido, bordes, fosas (régimen, arcos, erosión por subducción, sedimento), choques,
    /// delaminación, erosión con el destino de su sedimento, nivel del mar y balance.
    pub fn dar_paso(&mut self) {
        let dt = self.cfg.dt_myr;
        if self.traza.is_some() {
            self.traza_reloj = Some(std::time::Instant::now());
        }
        self.fosas_paso = ConteoFosas::default();
        self.envejecer_tt();
        let v0 = self.inventario_continental();
        let c0 = self.cuentas;
        self.velocidades();
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.q = Quat::de_rotacion(add(p.omega, p.empuje), dt)
                .componer(p.q)
                .normalizado();
            p.rot = p.q.matriz();
            p.empuje = [0.0; 3];
        }
        self.trazar("velocidades");
        self.recalcular_mundo();
        self.rasterizar();
        self.trazar("rasterizar");
        self.resolver_solapes();
        self.trazar("resolver_solapes");
        self.recolocar_dientes();
        self.trazar("recolocar_dientes");
        self.rellenar_huecos();
        self.trazar("rellenar_huecos");
        let (choques, tramos, difusas) = self.reconstruir_mapas();
        self.estirar_rifts();
        self.trazar("reconstruir_mapas");
        // Suavizado de la pertenencia de los bordes oceánicos y absorción de las islas (nota de
        // bordes §6.5).
        self.area_reasignada_km2 = self.suavizar_pertenencia();
        self.trazar("suavizar_pertenencia");
        self.area_islas_km2 = self.absorber_islas();
        self.trazar("absorber_islas");
        self.absorber_difusa(&difusas);
        self.trazar("absorber_difusa");
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.cort.iter_mut().for_each(|c| c.dh_dt = 0.0);
        }
        self.clasificar_bordes();
        self.trazar("clasificar_bordes");
        self.procesos_de_fosa(&tramos);
        self.trazar("procesos_de_fosa");
        self.engrosar_choques(&choques);
        self.trazar("engrosar_choques");
        self.delaminar();
        self.trazar("delaminar");
        self.flujo_lateral();
        self.trazar("flujo_lateral");

        let t0 = std::time::Instant::now();
        let erosion = self.erosion_y_envejecimiento();
        let t1 = std::time::Instant::now();
        // El nivel del mar y el reparto del sedimento ven las mismas alturas.
        let z = self.alturas();
        self.zeta_m = self.nivel_del_mar_con(&z);
        let t2 = std::time::Instant::now();
        self.sedimento_al_mar(&erosion, z);
        self.seg_partes[0] += (t1 - t0).as_secs_f64();
        self.seg_partes[1] += (t2 - t1).as_secs_f64();
        self.seg_partes[2] += t2.elapsed().as_secs_f64();
        self.trazar("erosion_y_sedimento");
        let mut cambio = self.suturas();
        cambio |= self.rifts();
        // Muerte de placas (nota de demografía de placas v2), después de los rifts: subducción de
        // dorsales (rama D) y captura de microplacas (A2).
        cambio |= self.subduccion_de_dorsales();
        cambio |= self.capturas();
        cambio |= self.saltos_de_dorsal();
        if cambio {
            for p in self.placas.iter_mut().filter(|p| p.viva) {
                p.actualizar_cap(&self.malla);
            }
            self.recalcular_mundo();
            self.tapar_huecos_del_mundo();
            self.clasificar_bordes();
        }
        self.trazar("eventos_de_placas");
        // Nota de fosas §9: el estado de los tramos con los bordes del final del paso, y los
        // nacimientos espontáneos.
        self.avanzar_fosas();
        // La invasión de transformantes y márgenes pasivos desde los extremos de las fosas (v4
        // §6.4) y los nacimientos espontáneos, que dejan puesto en los bordes el estado de los
        // tramos.
        self.invadir();
        self.nacimientos_espontaneos();
        self.trazar("fosas");
        self.paso += 1;
        self.t_myr = self.t0_myr + self.paso as f64 * dt;
        self.metricas_del_paso(v0, &c0);
        self.trazar("metricas");
    }

    /// Cierra una etapa de la traza de diagnóstico: su tiempo (sin contar lo que cuesta medir el
    /// espesor) y el espesor continental máximo al acabarla. A media etapa los mapas del mundo no
    /// están al día: el máximo se toma de todas las muestras de las placas (también las ocultas).
    fn trazar(&mut self, etapa: &'static str) {
        let Some(t0) = self.traza_reloj else {
            return;
        };
        let segundos = t0.elapsed().as_secs_f64();
        let (mut h_max_km, mut max_en) = (0.0, (NADA, NADA));
        for (pi, p) in self.placas.iter().enumerate().filter(|(_, p)| p.viva) {
            for (s, c) in p.cort.iter().enumerate() {
                if c.cont && c.h > h_max_km {
                    (h_max_km, max_en) = (c.h, (pi as u32, p.celda[s]));
                }
            }
        }
        let vigilada = self.vigilada.and_then(|(p, l)| {
            self.placas
                .get(p as usize)
                .filter(|pl| pl.viva)
                .and_then(|pl| pl.get(l))
                .copied()
        });
        let huecos = (0..self.malla.num)
            .filter(|&k| {
                let (p, l) = (self.dueno[k], self.rep[k]);
                p == NADA
                    || l == NADA
                    || self
                        .placas
                        .get(p as usize)
                        .is_none_or(|pl| pl.mapa.get(l as usize).is_none_or(|&s| s == NADA))
            })
            .count() as u32;
        if let Some(t) = self.traza.as_mut() {
            t.push(EtapaTraza {
                etapa,
                h_max_km,
                max_en,
                vigilada,
                huecos,
                segundos,
            });
        }
        self.traza_reloj = Some(std::time::Instant::now());
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

    /// Un paso sin movimiento de las placas (prueba de relajación de la adenda de balance §10: «se
    /// detiene la convergencia»): solo actúan la delaminación y la erosión con su sedimento.
    pub fn paso_sin_movimiento(&mut self) {
        self.envejecer_tt();
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.cort.iter_mut().for_each(|c| c.dh_dt = 0.0);
        }
        self.delaminar();
        let erosion = self.erosion_y_envejecimiento();
        let z = self.alturas();
        self.zeta_m = self.nivel_del_mar_con(&z);
        self.sedimento_al_mar(&erosion, z);
        self.paso += 1;
        self.t_myr = self.t0_myr + self.paso as f64 * self.cfg.dt_myr;
    }

    /// Envejecimiento de la edad tectonotérmica, al principio de cada paso (nota de isostasia con
    /// litosfera §6.1 y §9), y puesta a cero del área reiniciada en el paso.
    fn envejecer_tt(&mut self) {
        let dt = self.cfg.dt_myr;
        self.reinicio_km2_paso = [0.0; 4];
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            for c in p.cort.iter_mut() {
                c.edad_tt += dt;
            }
        }
    }

    /// Espesor máximo de la corteza continental que ve el mundo (km).
    pub fn espesor_max_continental(&self) -> f64 {
        (0..self.malla.num)
            .filter_map(|k| self.corteza(k).filter(|c| c.cont).map(|c| c.h))
            .fold(0.0, f64::max)
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
    /// resolver huecos y solapes, poner al día los mapas y el estado de los tramos convergentes
    /// (para las pruebas de la malla y de las fosas).
    pub fn paso_cinematico(&mut self) {
        let dt = self.cfg.dt_myr;
        self.envejecer_tt();
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.q = Quat::de_rotacion(add(p.omega, p.empuje), dt)
                .componer(p.q)
                .normalizado();
            p.rot = p.q.matriz();
        }
        self.recalcular_mundo();
        self.rasterizar();
        self.resolver_solapes();
        self.recolocar_dientes();
        self.rellenar_huecos();
        let (_, tramos, difusas) = self.reconstruir_mapas();
        self.estirar_rifts();
        self.area_subducida_km2 = tramos.iter().map(|t| t.a_sub).sum();
        self.area_reasignada_km2 = self.suavizar_pertenencia();
        self.area_islas_km2 = self.absorber_islas();
        self.fosas_paso = ConteoFosas::default();
        self.absorber_difusa(&difusas);
        // Estado de los tramos convergentes (nota de fosas), sin fuerzas.
        self.clasificar_bordes();
        self.avanzar_fosas();
        self.invadir();
        // Deja puesto en los bordes el estado de los tramos.
        self.nacimientos_espontaneos();
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

    /// Inventario de material continental de las placas (km³, `V_cont` de la adenda de balance):
    /// todas sus muestras, también las que el mundo no ve este paso; la corteza de tipo continental
    /// y el sedimento por su masa.
    pub fn inventario_continental(&self) -> f64 {
        let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
        self.placas
            .iter()
            .filter(|p| p.viva)
            .map(|p| {
                p.cort
                    .iter()
                    .zip(&p.celda)
                    .map(|(c, &l)| c.v_cont(rs, rc) * self.malla.area[l as usize])
                    .sum::<f64>()
            })
            .sum()
    }

    /// Material continental (km³) que ve el mundo: `Σ (h_c + S·ρ_s/ρ_c)·A` por celda.
    pub fn volumen_continental(&self) -> f64 {
        let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
        (0..self.malla.num)
            .filter_map(|k| {
                self.corteza(k)
                    .map(|c| c.v_cont(rs, rc) * self.malla.area[k])
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
        self.nivel_del_mar_con(&self.alturas())
    }

    /// El nivel del mar con las alturas `z` de [`Deriva::alturas`] ya calculadas.
    fn nivel_del_mar_con(&self, z: &[f64]) -> f64 {
        let vol = self.cfg.gel_m / 1000.0 * self.malla.area_total();
        isostasia::nivel_del_mar(z, &self.malla.area, vol)
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
                        contacto_myr: f64::NEG_INFINITY,
                        choque_visto_myr: f64::NEG_INFINITY,
                        sutura_myr: 0.0,
                        habia_subduccion: false,
                        habia_colision: false,
                        choque_solapado: 0.0,
                        choque_arco: 0.0,
                        choque_perdido: 0.0,
                        choque_ultimo_myr: f64::NEG_INFINITY,
                        choque_avisado: false,
                        choque_inicio_myr: 0.0,
                        regimen: None,
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
        let mut t_tiron = vec![[0.0; 3]; n];
        let mut acop: Vec<(usize, usize, Mat3)> = Vec::new();
        for b in &self.bordes {
            let x = self.malla.dir[b.k as usize];
            let p = b.p as usize;
            // Tirón de la losa (§6.2), que crece con su longitud (nota de fosas §6.5): mientras la
            // fosa viva, aunque en un paso las placas se deslicen, y en un choque hasta que la losa
            // se rompe (v4 §6.6), con la edad del fondo que se hundía.
            let tira = b.p_subduce && b.losa > 0.0;
            let f = if tira {
                let edad = if b.edad_losa > 0.0 {
                    b.edad_losa
                } else {
                    self.corteza(b.k as usize).map_or(0.0, |c| c.edad_oc)
                };
                esc(
                    b.normal,
                    self.cfg.f_sp * (edad / self.cfg.t_sat_myr).sqrt().min(1.0) * b.losa,
                )
            } else if b.tipo == TipoBorde::Divergente {
                esc(b.normal, -self.cfg.f_rp)
            } else {
                [0.0; 3]
            };
            if b.tipo == TipoBorde::Colision {
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
            }
            let par = esc(cross(x, f), r * b.largo_km);
            t[p] = add(t[p], par);
            if b.p_subduce && b.losa > 0.0 {
                t_tiron[p] = add(t_tiron[p], par);
            }
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
        // Potencias (τ·ω) de cada fuerza en cada placa, antes del marco sin rotación neta: tirón,
        // empuje (el resto del par de borde), choques (−Σ acoplamientos) y arrastre (−R²·M·ω).
        self.potencias.clear();
        let mut t_choque = vec![[0.0; 3]; n];
        for (p, q, gg) in &acop {
            let d = sub(omega[*p], omega[*q]);
            let f = aplicar(gg, d);
            t_choque[*p] = sub(t_choque[*p], f);
            t_choque[*q] = add(t_choque[*q], f);
        }
        let area_p = self.area_placa();
        for &p in &vivas {
            let w = omega[p];
            let drag = esc(aplicar(&m[p], w), -r * r);
            let tiron = dot(t_tiron[p], w);
            let empuje = dot(t[p], w) - tiron;
            self.potencias.push((
                p as u32,
                area_p[p],
                [tiron, empuje, dot(t_choque[p], w), dot(drag, w)],
            ));
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

    /// Tras un evento de placas (fusión, rift, absorción, salto de dorsal), una celda del mundo puede
    /// quedarse sin muestra hasta el paso siguiente, que lo rasteriza todo de nuevo. Mientras, ve la
    /// muestra de su primera vecina con corteza (en el orden fijo de la malla): no se crea corteza.
    fn tapar_huecos_del_mundo(&mut self) {
        let num = self.malla.num;
        let vacia = |d: &Deriva, k: usize| {
            let (p, l) = (d.dueno[k], d.rep[k]);
            p == NADA
                || l == NADA
                || !d.placas[p as usize].viva
                || d.placas[p as usize].mapa[l as usize] == NADA
        };
        let huecos: Vec<usize> = (0..num).filter(|&k| vacia(self, k)).collect();
        for k in huecos {
            if let Some(&v) = self.malla.vec[k]
                .iter()
                .find(|&&v| v != NADA && !vacia(self, v as usize))
            {
                self.dueno[k] = self.dueno[v as usize];
                self.rep[k] = self.rep[v as usize];
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
        // Tipo de tramo de cada solape y de cada hueco, con el borde suavizado (nota de bordes §6.3).
        self.clasificar_reclamos();
        // (id menor, id mayor) → (Σ edad·A y Σ A de cada lado en solapes O+O; Σ H·A y Σ A en C+C).
        // Solo en tramos convergentes: los dientes de un borde que se desliza no deciden quién se
        // hunde.
        let mut medias: Vec<((u64, u64), [f64; 8])> = Vec::new();
        for k in 0..self.malla.num {
            let n = self.ncov[k] as usize;
            if n < 2 || self.tipo_k[k] != TipoTramo::Convergente {
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
            if n >= 2 && self.tipo_k[k] != TipoTramo::Convergente {
                // Diente de un tramo transformante o divergente (§6.4): no se consume nada. La celda
                // queda para la placa del lado del borde suavizado donde cae (la de más φ) y la
                // muestra de la otra se recolocará en un hueco de su propia placa.
                let f = self.fi(k, true);
                let gana = (0..n)
                    .max_by(|&a, &b| {
                        let (pa, pb) = (self.cov[k][a].0, self.cov[k][b].0);
                        f.de(pa).total_cmp(&f.de(pb)).then(
                            self.sorteo(k as u64, self.placas[pa as usize].id)
                                .cmp(&self.sorteo(k as u64, self.placas[pb as usize].id)),
                        )
                    })
                    .expect("hay reclamos");
                for i in (0..n).filter(|&i| i != gana) {
                    let (p, l) = self.cov[k][i];
                    self.perdidas.push(Perdida {
                        k: k as u32,
                        p,
                        l,
                        choque: false,
                        convergente: false,
                        difusa: false,
                    });
                }
                self.dueno[k] = self.cov[k][gana].0;
                self.rep[k] = self.cov[k][gana].1;
                continue;
            }
            let mut gana = self.cov[k][0];
            let mut perd: Vec<(u32, u32, bool)> = Vec::new();
            for i in 1..n {
                let otra = self.cov[k][i];
                let (g, pierde, choque) = self.torneo(gana, otra);
                perd.push((pierde.0, pierde.1, choque));
                gana = g;
            }
            // Una muestra suelta (su placa apenas está alrededor: menos de PHI_SUELTA del área a
            // menos de L_borde) no gana: es una isla de su placa dentro de otra y se comería, paso a
            // paso, la corteza que le pasa por debajo. La celda queda para la placa de más φ y la
            // isla es un diente. Si pierde, se consume como siempre (es corteza enterrada que
            // reaparece).
            let f = self.fi(k, true);
            if f.de(gana.0) < PHI_SUELTA
                && let Some(b) = (0..n)
                    .map(|i| self.cov[k][i])
                    .filter(|c| f.de(c.0) >= PHI_SUELTA)
                    .max_by(|a, b| {
                        f.de(a.0).total_cmp(&f.de(b.0)).then(
                            self.sorteo(k as u64, self.placas[a.0 as usize].id)
                                .cmp(&self.sorteo(k as u64, self.placas[b.0 as usize].id)),
                        )
                    })
            {
                perd.retain(|x| (x.0, x.1) != (b.0, b.1));
                self.perdidas.push(Perdida {
                    k: k as u32,
                    p: gana.0,
                    l: gana.1,
                    choque: false,
                    convergente: false,
                    difusa: false,
                });
                gana = b;
            }
            for (p, l, choque) in perd {
                // Nota de fosas §6.2: sin una fosa activa entre las dos placas, el tramo es difuso y
                // lo que converge se absorbe engrosando; con ella, se consume (un choque, siempre).
                let difusa = !choque && self.hunde_en_fosa(gana, (p, l)).is_none();
                self.perdidas.push(Perdida {
                    k: k as u32,
                    p,
                    l,
                    choque,
                    convergente: true,
                    difusa,
                });
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
                // Nota de fosas: en una fosa activa, la polaridad fijada al nacer; si no (tramo
                // difuso), la que tendría al nacer (6.3): entre dos fondos que no flotan, el más
                // viejo.
                let hunde = match self.hunde_en_fosa(a, b) {
                    Some(h) => h,
                    None => {
                        if ca.edad_oc != cb.edad_oc {
                            if ca.edad_oc > cb.edad_oc { a.0 } else { b.0 }
                        } else if ida > idb {
                            a.0
                        } else {
                            b.0
                        }
                    }
                };
                if hunde == a.0 {
                    subduce(a, b, self)
                } else {
                    subduce(b, a, self)
                }
            }
        }
    }

    /// Dientes de tramos transformantes o divergentes (nota de bordes §6.4): la muestra que solo
    /// perdió celdas en solapes que no convergen no se consume; va al hueco de su propia placa (uno
    /// que toca su placa, de cualquier tipo) más cercano a menos de `r_busca = L_borde`, con su
    /// volumen. Los que no lo encuentran buscan, a menos de 3·L_borde, un hueco de tramo
    /// transformante o convergente que toque su placa (adenda v5: el reparto de los dientes era lo que
    /// más subía los picos). Desempate sembrado. Va antes de repartir los huecos, para que un diente
    /// llene el hueco que su placa deja detrás al deslizarse.
    fn recolocar_dientes(&mut self) {
        self.dientes_recolocados_km2 = 0.0;
        self.dientes_continentales_km3 = 0.0;
        self.dientes_cont_por_via_km3 = [0.0; 4];
        // Muestras que aún representan una celda propia o que perdieron alguna en un tramo
        // convergente (esas se consumen): no son dientes.
        let mut queda: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
        for k in 0..self.malla.num {
            for i in 0..self.ncov[k] as usize {
                let (p, l) = self.cov[k][i];
                if self.dueno[k] == p {
                    queda.insert((p, l));
                }
            }
        }
        for per in self.perdidas.iter().filter(|x| x.convergente) {
            queda.insert((per.p, per.l));
        }
        let mut dientes: Vec<(u32, u32, u32)> = self
            .perdidas
            .iter()
            .filter(|x| !x.convergente && !queda.contains(&(x.p, x.l)))
            .map(|x| (x.p, x.l, x.k))
            .collect();
        dientes.sort_unstable();
        dientes.dedup_by_key(|x| (x.0, x.1));
        // Hueco libre que toca la placa `pi` (el que deja detrás al deslizarse).
        let libre_junto = |d: &Deriva, g: u32, pi: u32| {
            d.hueco[g as usize]
                && d.dueno[g as usize] == NADA
                && d.malla.vec[g as usize]
                    .iter()
                    .any(|&v| v != NADA && d.dueno[v as usize] == pi)
        };
        let mas_cerca = |d: &Deriva, xd: V3, l: u32, g1: u32, g2: u32| {
            dot(xd, d.malla.dir[g1 as usize])
                .total_cmp(&dot(xd, d.malla.dir[g2 as usize]))
                .then(
                    d.sorteo(g1 as u64, l as u64)
                        .cmp(&d.sorteo(g2 as u64, l as u64)),
                )
        };
        // Primera pasada: un hueco de cualquier tipo a menos de L_borde.
        let mut sin_hueco: Vec<(u32, u32, u32)> = Vec::new();
        for (pi, l, kd) in dientes {
            let xd = self.malla.dir[kd as usize];
            let hueco = self
                .vecindad
                .de(kd as usize)
                .iter()
                .copied()
                .filter(|&g| libre_junto(self, g, pi))
                .max_by(|&g1, &g2| mas_cerca(self, xd, l, g1, g2));
            if !hueco.is_some_and(|g| self.poner_diente(pi, l, g, 0)) {
                sin_hueco.push((pi, l, kd));
            }
        }
        // Segunda pasada: los huecos de tramos transformantes o convergentes, que esperan dientes y
        // si no reciben ninguno se llenan con fondo nuevo, a menos de 3·L_borde. Un diente sin hueco
        // y un hueco sin diente son las dos caras del mismo borde en escalera; los de dorsal no, para
        // no poner corteza continental en una dorsal.
        if sin_hueco.is_empty() {
            return;
        }
        let tipados: Vec<u32> = (0..self.malla.num as u32)
            .filter(|&g| {
                self.hueco[g as usize]
                    && self.dueno[g as usize] == NADA
                    && matches!(
                        self.tipo_k.get(g as usize),
                        Some(TipoTramo::Transformante | TipoTramo::Convergente)
                    )
            })
            .collect();
        let cos_max = (3.0 * self.cfg.l_borde_km / self.malla.r_km).cos();
        for (pi, l, kd) in sin_hueco {
            let xd = self.malla.dir[kd as usize];
            let hueco = tipados
                .iter()
                .copied()
                .filter(|&g| {
                    dot(xd, self.malla.dir[g as usize]) >= cos_max && libre_junto(self, g, pi)
                })
                .max_by(|&g1, &g2| mas_cerca(self, xd, l, g1, g2));
            if let Some(g) = hueco {
                self.poner_diente(pi, l, g, 1);
            }
        }
    }

    /// Un diente sin hueco con su placa alrededor: al menos un cuarto de las celdas con placa a menos
    /// de `r_busca` de donde perdió su celda son de su placa.
    fn diente_con_su_placa(&self, pi: u32, kd: u32) -> bool {
        let (mut propias, mut vecinas) = (0usize, 0usize);
        for &j in self.vecindad.de(kd as usize) {
            let p = self.dueno[j as usize];
            if p != NADA && self.rep[j as usize] != NADA {
                vecinas += 1;
                if p == pi {
                    propias += 1;
                }
            }
        }
        propias > 0 && 4 * propias >= vecinas
    }

    /// Lleva el diente (muestra `l` de la placa `pi`) al hueco `g` con su volumen. Devuelve si lo
    /// puso (la celda local del hueco puede estar ocupada).
    fn poner_diente(&mut self, pi: u32, l: u32, g: u32, via: usize) -> bool {
        let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
        let gu = g as usize;
        let a = self.malla.area[l as usize];
        let p = &mut self.placas[pi as usize];
        let lg = self.malla.localizar(aplicar_t(&p.rot, self.malla.dir[gu])) as u32;
        if p.mapa[lg as usize] != NADA {
            return false;
        }
        let Some(c) = p.quitar(l) else {
            return false;
        };
        // Mismo volumen en una celda de otra área: se ajusta el espesor.
        let f = a / self.malla.area[lg as usize];
        let mut c2 = Corteza {
            h: if c.cont {
                c.h * f
            } else {
                c.h - c.hc + c.hc * f
            },
            hc: c.hc * f,
            sed: c.sed * f,
            proc: c.proc.escalada(f),
            ..c
        };
        c2.proc.lote = self.lotes.len() as u32;
        p.insertar(lg, c2, g);
        self.lotes.push(lote::DIENTE);
        self.dueno[gu] = pi;
        self.rep[gu] = lg;
        self.hueco[gu] = false;
        self.dientes_recolocados_km2 += a;
        let v = c.v_cont(rs, rc) * a;
        self.dientes_continentales_km3 += v;
        self.dientes_cont_por_via_km3[via] += v;
        true
    }

    /// Huecos (§6.3): cada celda sin placa pasa a la placa cubierta más cercana (empate: id menor);
    /// ahí nacerá corteza oceánica nueva.
    fn rellenar_huecos(&mut self) {
        let num = self.malla.num;
        // Los huecos de tramos transformantes o convergentes esperan a los dientes recolocados
        // (nota de bordes §6.4): se reparten en `reconstruir_mapas`.
        let mut frente: Vec<u32> = (0..num as u32)
            .filter(|&k| {
                self.dueno[k as usize] == NADA
                    && !matches!(
                        self.tipo_k.get(k as usize),
                        Some(TipoTramo::Transformante | TipoTramo::Convergente)
                    )
            })
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
    /// Devuelve la corteza continental que la placa de abajo mete bajo la de arriba (choques) y los
    /// tramos de fosa con el fondo oceánico consumido en el paso (adenda de balance §6.0).
    fn reconstruir_mapas(&mut self) -> (Vec<Choque>, Vec<Tramo>, Vec<Difusa>) {
        let num = self.malla.num;

        let cfg = self.cfg.clone();
        let (rs, rc) = (cfg.rho_s, cfg.rho_c);
        self.dientes_sin_hueco_km2 = 0.0;
        self.dientes_a_otra_placa_km2 = 0.0;
        self.max_tras_dientes_km = [0.0; 2];
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
        // Frente de cada muestra perdida: la celda del mundo donde se perdió (la primera), en un
        // tramo convergente (se consume) o no (es un diente: se recoloca; nota de bordes §6.4).
        let mut frente_de: Vec<(u32, u32, u32)> = self
            .perdidas
            .iter()
            .filter(|x| x.convergente)
            .map(|x| (x.p, x.l, x.k))
            .collect();
        frente_de.sort_unstable();
        // Muestras con alguna pérdida en una fosa activa (las demás convergentes son de tramos
        // difusos: se absorben, nota de fosas §6.2).
        let mut en_fosa: Vec<(u32, u32)> = self
            .perdidas
            .iter()
            .filter(|x| x.convergente && !x.difusa)
            .map(|x| (x.p, x.l))
            .collect();
        en_fosa.sort_unstable();
        en_fosa.dedup();
        let mut difusas: Vec<Difusa> = Vec::new();
        let mut diente_de: Vec<(u32, u32, u32)> = self
            .perdidas
            .iter()
            .filter(|x| !x.convergente)
            .map(|x| (x.p, x.l, x.k))
            .collect();
        diente_de.sort_unstable();
        let mut dientes: Vec<(u32, u32, u32, Corteza)> = Vec::new();
        let mut choques: Vec<Choque> = Vec::new();
        let mut tramos: Vec<Tramo> = Vec::new();
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
            for (l, _) in quitar {
                let buscar = |lista: &[(u32, u32, u32)]| {
                    let i = lista.partition_point(|x| (x.0, x.1) < (pi as u32, l));
                    lista
                        .get(i)
                        .filter(|x| (x.0, x.1) == (pi as u32, l))
                        .map(|x| x.2)
                };
                let frente = buscar(&frente_de);
                let diente = buscar(&diente_de);
                // Un diente sin hueco con su placa alrededor no se reparte (adenda v5): se queda en
                // ella como muestra oculta, con su volumen, y el paso siguiente vuelve a intentarlo.
                // Repartido, engrosaba los bordes que se deslizan y subía los picos.
                if frente.is_none()
                    && let Some(kd) = diente
                    && self.diente_con_su_placa(pi as u32, kd)
                {
                    let a = self.malla.area[l as usize];
                    if let Some(c) = self.placas[pi].get(l) {
                        self.dientes_cont_por_via_km3[2] += c.v_cont(rs, rc) * a;
                    }
                    self.dientes_sin_hueco_km2 += a;
                    continue;
                }
                let Some(c) = self.placas[pi].quitar(l) else {
                    continue;
                };
                let a = self.malla.area[l as usize];
                let Some(k) = frente else {
                    match diente {
                        // Solo perdió celdas en tramos que no convergen: es un diente.
                        Some(kd) => dientes.push((pi as u32, l, kd, c)),
                        // Toda muestra consumida perdió alguna celda en un solape: no pasa.
                        None => self.cuentas.choque_sin_sitio_km3 += c.v_cont(rs, rc) * a,
                    }
                    continue;
                };
                if en_fosa.binary_search(&(pi as u32, l)).is_err() {
                    difusas.push(Difusa {
                        k,
                        ganadora: self.dueno[k as usize],
                        perdedora: pi as u32,
                        c,
                        a,
                    });
                    continue;
                }
                // Corteza de tipo continental de la placa de abajo (§6.5): la de un continente con
                // su sedimento, o la que un arco o la acreción pusieron sobre el fondo oceánico.
                let v_sol = if c.cont {
                    c.v_cont(rs, rc) * a
                } else {
                    c.hc * a
                };
                if v_sol > 0.0 {
                    // La eficacia depende del grosor de la corteza que entra (adenda v7 §6.5): se
                    // hunde sobre todo el margen adelgazado. La parte de arco se queda con η_arco
                    // (adenda v9): η = a_i·η_arco + (1 − a_i)·η(h).
                    let fa = c.arco;
                    let eta_h = cfg.eta_choque(c.h);
                    let eta = fa * cfg.eta_arco + (1.0 - fa) * eta_h;
                    choques.push(Choque {
                        k,
                        vol: eta * v_sol,
                        eta,
                        vol_arco: fa * cfg.eta_arco * v_sol,
                    });
                    self.cuentas.choque_solapado_km3 += v_sol;
                    self.cuentas.choque_perdida_km3 += (1.0 - eta) * v_sol;
                    // Diagnóstico por grosor y por vía, de la parte que no es de arco.
                    let v_na = (1.0 - fa) * v_sol;
                    if v_na > 0.0 {
                        let clase = if !c.cont {
                            3
                        } else if c.h < cfg.h1_choque_km {
                            0
                        } else if c.h < cfg.h2_choque_km {
                            1
                        } else {
                            2
                        };
                        self.cuentas.choque_solapado_por_grosor_km3[clase] += v_na;
                        self.cuentas.choque_perdida_por_grosor_km3[clase] += (1.0 - eta_h) * v_na;
                        let tot: f64 = c.proc.vias.iter().map(|&x| x as f64).sum();
                        if tot > 0.0 {
                            for (i, &x) in c.proc.vias.iter().enumerate() {
                                let f = x as f64 / tot;
                                self.diag_choque_km3[0][clase][i] += v_na * f;
                                self.diag_choque_km3[1][clase][i] += (1.0 - eta_h) * v_na * f;
                            }
                        }
                    }
                    // ¿Cuelga aún la losa de una fosa entre las dos (v4 §6.6)?
                    let arriba = self.dueno[k as usize];
                    let con_losa = arriba != NADA
                        && (self
                            .estado_fosa(pi as u32, l, arriba)
                            .is_some_and(|e| e.activa)
                            || self
                                .estado_fosa(arriba, self.rep[k as usize], pi as u32)
                                .is_some_and(|e| e.activa));
                    if con_losa {
                        self.cuentas.choque_perdida_con_losa_km3 += (1.0 - eta) * v_sol;
                    } else {
                        self.cuentas.choque_perdida_sin_losa_km3 += (1.0 - eta) * v_sol;
                    }
                    self.anotar_choque(pi, k, v_sol, (1.0 - eta) * v_sol, fa, eta);
                }
                // Fondo oceánico que se hunde (§6.0) con su sedimento (§6.4).
                if !c.cont {
                    tramos.push(Tramo {
                        k,
                        a_sub: a,
                        v_llega: c.sed * rs / rc * a,
                        s_area: c.sed * a,
                        edad_area: c.edad_oc * a,
                        placa: pi as u32,
                        a_placa: a,
                    });
                }
            }
        }

        // Muestras sueltas (dientes sin hueco y sin su placa alrededor: menos de un cuarto de las
        // celdas a menos de r_busca): su material pasa a las celdas de alrededor, de cualquier placa,
        // por área; su fondo oceánico desaparece sin contar como consumo.
        let h_cont_min = cfg.h_cont_min_km;
        for (_pi, l, kd, c) in dientes {
            let a = self.malla.area[l as usize];
            self.dientes_continentales_km3 += c.v_cont(rs, rc) * a;
            let destino: Vec<u32> = self
                .vecindad
                .de(kd as usize)
                .iter()
                .copied()
                .filter(|&j| self.dueno[j as usize] != NADA && self.rep[j as usize] != NADA)
                .collect();
            let cual = 1;
            self.dientes_a_otra_placa_km2 += a;
            self.dientes_cont_por_via_km3[3] += c.v_cont(rs, rc) * a;
            let a_tot: f64 = destino.iter().map(|&j| self.malla.area[j as usize]).sum();
            if a_tot <= 0.0 {
                self.cuentas.choque_sin_sitio_km3 += c.v_cont(rs, rc) * a;
                continue;
            }
            for &j in &destino {
                let ju = j as usize;
                let (pj, lj) = (self.dueno[ju], self.rep[ju]);
                let aj = self.malla.area[lj as usize];
                let w = self.malla.area[ju] / a_tot;
                if let Some(d) = self.placas[pj as usize].get_mut(lj) {
                    let r = a * w / aj;
                    d.sumar_hc(
                        c.hc * r,
                        c.arco,
                        c.edad_tt,
                        rs,
                        rc,
                        h_cont_min,
                        via::DIENTES,
                    );
                    d.sed += c.sed * r;
                    if d.cont {
                        self.max_tras_dientes_km[cual] = self.max_tras_dientes_km[cual].max(d.h);
                    }
                }
            }
        }
        // Huecos de tramos transformantes o convergentes que no llenó ningún diente: a la placa de
        // más φ, con fondo oceánico nuevo (creación residual: la pequeña apertura real del borde).
        self.creacion_residual_km2 = 0.0;
        // Diagnóstico de procedencia: qué huecos son de creación residual.
        let mut residual = vec![false; num];
        for (k, es_residual) in residual.iter_mut().enumerate() {
            if self.dueno[k] != NADA || !self.hueco[k] {
                continue;
            }
            *es_residual = true;
            let destino = self.fi(k, true).mayor().map(|x| x.0).or_else(|| {
                self.malla.vec[k]
                    .iter()
                    .filter(|&&v| v != NADA && self.dueno[v as usize] != NADA)
                    .map(|&v| self.dueno[v as usize])
                    .min()
            });
            if let Some(p) = destino {
                self.dueno[k] = p;
                self.creacion_residual_km2 += self.malla.area[k];
            }
        }
        // Un tramo por celda de frente (orden fijo por celda y placa).
        tramos.sort_unstable_by_key(|t| (t.k, t.placa));
        let mut por_celda: Vec<Tramo> = Vec::new();
        for t in tramos {
            match por_celda.last_mut() {
                Some(u) if u.k == t.k => {
                    u.a_sub += t.a_sub;
                    u.v_llega += t.v_llega;
                    u.s_area += t.s_area;
                    u.edad_area += t.edad_area;
                    if u.placa == t.placa {
                        u.a_placa += t.a_placa;
                    } else if t.a_placa > u.a_placa {
                        u.placa = t.placa;
                        u.a_placa = t.a_placa;
                    }
                }
                _ => por_celda.push(t),
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
        self.creacion_paso.clear();
        for (k, &es_residual) in residual.iter().enumerate() {
            if !self.hueco[k] {
                continue;
            }
            self.area_nueva_km2 += self.malla.area[k];
            self.creacion_paso.push(k as u32);
            let pi = self.dueno[k];
            if pi == NADA {
                continue;
            }
            let p = &mut self.placas[pi as usize];
            let l = self.malla.localizar(aplicar_t(&p.rot, self.malla.dir[k])) as u32;
            if p.mapa[l as usize] == NADA {
                let mut c = Corteza::oceanica(cfg.h_oc_km);
                c.proc.nace = if es_residual {
                    nace::HUECO
                } else {
                    nace::DORSAL
                };
                p.insertar(l, c, k as u32);
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
                // Solo se tapa con corteza nueva (la celda del mundo era un hueco): copiar corteza
                // existente crearía material (adenda §0, conservación exacta).
                if !self.hueco[k2] {
                    continue;
                }
                let mut c = Corteza::oceanica(cfg.h_oc_km);
                c.proc.nace = if residual[k2] {
                    nace::HUECO
                } else {
                    nace::DORSAL
                };
                self.placas[pi as usize].insertar(l2, c, k2 as u32);
            }
        }
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            p.actualizar_cap(&self.malla);
        }
        self.consumo_paso = por_celda.iter().map(|t| (t.k, t.a_sub)).collect();
        self.hundido_paso = por_celda.iter().map(|t| (t.placa, t.a_sub)).collect();
        (choques, por_celda, difusas)
    }

    /// Lleva la cuenta de un choque entre la placa `abajo` (índice) y la dueña de la celda `k`, y
    /// avisa de su inicio (adenda de balance §7).
    fn anotar_choque(
        &mut self,
        abajo: usize,
        k: u32,
        v_sol: f64,
        perdido: f64,
        f_arco: f64,
        eta: f64,
    ) {
        let arriba = self.dueno[k as usize];
        if arriba == NADA {
            return;
        }
        let (ida, idb) = (self.placas[abajo].id, self.placas[arriba as usize].id);
        let i = self.par_idx(ida, idb);
        let t = self.t_myr;
        let par = &mut self.pares[i];
        par.choque_solapado += v_sol;
        par.choque_perdido += perdido;
        if f_arco > 0.0 {
            par.choque_arco += f_arco * v_sol;
        }
        par.choque_ultimo_myr = t;
        if !par.choque_avisado {
            par.choque_avisado = true;
            par.choque_inicio_myr = t;
            let clase = if f_arco >= 0.5 {
                "arco_continente"
            } else {
                "continente_continente"
            };
            let d = self.malla.dir[k as usize];
            self.evento_det(
                "inicio_choque",
                vec![ida, idb],
                0.0,
                0.0,
                Some(d),
                Detalle {
                    clase: Some(clase.to_string()),
                    eta: Some(eta),
                    ..Detalle::default()
                },
            );
        }
    }

    /// Muestras que reciben lo que se calcula para cada celda del mundo: la que el mundo ve en ella
    /// (su representante) y las ocultas del dueño cuyo centro cae en ella, para que no se queden
    /// atrás (I2). Una muestra visible solo recibe por su celda: si recibiera por la de su centro
    /// (a menudo la vecina), el sedimento de una depresión podría levantar una montaña al lado.
    fn indice_inverso(&self) -> (Vec<u32>, Vec<u32>) {
        let num = self.malla.num;
        // Muestras que el mundo ve (representantes de alguna celda).
        let mut visible: Vec<Vec<bool>> = self
            .placas
            .iter()
            .map(|p| vec![false; if p.viva { p.cort.len() } else { 0 }])
            .collect();
        let mut rep_de = vec![NADA; num];
        for (k, rk) in rep_de.iter_mut().enumerate() {
            let (p, l) = (self.dueno[k], self.rep[k]);
            if p == NADA || l == NADA {
                continue;
            }
            let s = self.placas[p as usize].mapa[l as usize];
            if s != NADA {
                visible[p as usize][s as usize] = true;
                *rk = s;
            }
        }
        let mut off = vec![0u32; num + 1];
        for k in 0..num {
            if rep_de[k] != NADA {
                off[k + 1] += 1;
            }
        }
        for (pi, p) in self.placas.iter().enumerate() {
            if !p.viva {
                continue;
            }
            for (s, &k) in p.mundo.iter().enumerate() {
                if self.dueno[k as usize] == pi as u32 && !visible[pi][s] {
                    off[k as usize + 1] += 1;
                }
            }
        }
        for k in 0..num {
            off[k + 1] += off[k];
        }
        let mut pos = off.clone();
        let mut lista = vec![0u32; off[num] as usize];
        for k in 0..num {
            if rep_de[k] != NADA {
                lista[pos[k] as usize] = rep_de[k];
                pos[k] += 1;
            }
        }
        for (pi, p) in self.placas.iter().enumerate() {
            if !p.viva {
                continue;
            }
            for (s, &k) in p.mundo.iter().enumerate() {
                if self.dueno[k as usize] == pi as u32 && !visible[pi][s] {
                    lista[pos[k as usize] as usize] = s as u32;
                    pos[k as usize] += 1;
                }
            }
        }
        (off, lista)
    }

    /// Reparte el volumen `vol` (km³) entre las muestras receptoras de la celda del mundo `k`
    /// ([`Deriva::indice_inverso`]): todas reciben el mismo espesor `vol / Σ área`, así que lo
    /// repartido es exactamente `vol` (conservación, adenda §0). `f` recibe la muestra, su área y el
    /// espesor que le toca. Devuelve el volumen repartido (0 sin dueño).
    fn en_celda(
        &mut self,
        inv: &(Vec<u32>, Vec<u32>),
        k: usize,
        vol: f64,
        solo_cont: bool,
        mut f: impl FnMut(&mut Corteza, f64, f64),
    ) -> f64 {
        let pi = self.dueno[k];
        if pi == NADA {
            return 0.0;
        }
        let (off, lista) = inv;
        let (a, b) = (off[k] as usize, off[k + 1] as usize);
        let p = &mut self.placas[pi as usize];
        let vale = |c: &Corteza| !solo_cont || c.cont;
        let total: f64 = lista[a..b]
            .iter()
            .filter(|&&s| vale(&p.cort[s as usize]))
            .map(|&s| self.malla.area[p.celda[s as usize] as usize])
            .sum();
        if total <= 0.0 {
            // Ninguna muestra (que valga) cae en `k`: va a su representante, si vale.
            let l = self.rep[k];
            let ar = self.malla.area[l as usize];
            return match p.get_mut(l) {
                Some(c) if vale(c) => {
                    f(c, ar, vol / ar);
                    vol
                }
                _ => 0.0,
            };
        }
        for &s in &lista[a..b] {
            if vale(&p.cort[s as usize]) {
                let ar = self.malla.area[p.celda[s as usize] as usize];
                f(&mut p.cort[s as usize], ar, vol / total);
            }
        }
        vol
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
            // La longitud, por el gradiente del indicador (el método con que se mide la Tierra); la
            // normal, la del borde suavizado a L_borde (nota de bordes §6.2), sin los dientes.
            let normal = self
                .normal_suave(k, p, q, false)
                .unwrap_or_else(|| normalize(add(esc(e, ge / gmod), esc(nn, gn / gmod))));
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
                || v_n.abs() < self.cfg.theta_tr_grados.to_radians().sin() * vmod
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
                        // Provisional: el estado del tramo (fosa activa con su polaridad o
                        // convergencia difusa) lo pone `aplicar_estado_fosas`.
                        let idp = self.placas[p as usize].id;
                        let idq = self.placas[q as usize].id;
                        let hunde_p =
                            cp.edad_oc > cq.edad_oc || (cp.edad_oc == cq.edad_oc && idp > idq);
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
                v_rel: vmod,
                tipo,
                p_subduce,
                losa: 0.0,
                edad_losa: 0.0,
            });
        }
        self.aplicar_estado_fosas();
    }

    // --- d. Fosas: régimen, arcos, erosión por subducción y sedimento (adenda de balance §6) ------

    /// Procesos de cada tramo de fosa (adenda de balance §6.1–§6.4, en este orden):
    /// - régimen del margen: erosivo si converge a más de `v_ero` y le llega menos de `S_ero` de
    ///   sedimento; si no, acrecionario;
    /// - arco: `p_arc · A_sub` km³ en la franja `d_arco ± w_arco/2` de la placa de arriba, con la
    ///   tasa del tipo de corteza de cada celda (continental o, si no, oceánica o de arco);
    /// - erosión por subducción: `p_SE · A_sub` km³ de la corteza continental de la placa de arriba,
    ///   desde la celda más cercana a la fosa hacia dentro (hasta `d_arco + w_arco/2`);
    /// - sedimento que llega: en un margen acrecionario `f_acr` se pega a la celda del frente y el
    ///   resto va al manto; en uno erosivo, todo al manto.
    fn procesos_de_fosa(&mut self, frentes: &[Tramo]) {
        self.area_subducida_km2 = frentes.iter().map(|t| t.a_sub).sum();
        self.area_erosiva_km2 = 0.0;
        self.consumo_en_borde_paso.clear();
        if frentes.is_empty() {
            return;
        }
        let cfg = self.cfg.clone();
        let (rs, rc) = (cfg.rho_s, cfg.rho_c);
        let num = self.malla.num;
        let r = self.malla.r_km;
        let mut borde_en = vec![NADA; num];
        for (i, b) in self.bordes.iter().enumerate() {
            borde_en[b.k as usize] = i as u32;
        }
        let tramos = self.tramos_en_el_borde(frentes, &borde_en);
        let tramos = &tramos[..];
        // Dónde ocurre el consumo, por el tipo de borde del tramo (diagnóstico de la forma de los
        // bordes: en la Tierra casi todo entra por las fosas).
        for t in tramos {
            let i = borde_en[t.k as usize];
            let tipo = if i == NADA {
                0
            } else {
                self.bordes[i as usize].tipo.codigo() as usize
            };
            self.cuentas.area_subducida_por_borde_km2[tipo] += t.a_sub;
            self.consumo_en_borde_paso.push((t.k, tipo as u8, t.a_sub));
        }
        // 6.2 Régimen de cada tramo, con la convergencia perpendicular al borde.
        let v_tramo: Vec<f64> = tramos
            .iter()
            .map(|t| {
                let k = t.k as usize;
                let arriba = self.dueno[k];
                let i = borde_en[k];
                if i != NADA && self.bordes[i as usize].p == arriba {
                    self.bordes[i as usize].v_n.max(0.0)
                } else {
                    // Celda de frente sin borde (la placa de abajo perdió más de una fila en el
                    // paso): la velocidad relativa entera.
                    let x = self.malla.dir[k];
                    let pa = &self.placas[arriba as usize];
                    let pb = &self.placas[t.placa as usize];
                    let vrel = sub(
                        velocidad(add(pb.omega, pb.empuje), x, r),
                        velocidad(add(pa.omega, pa.empuje), x, r),
                    );
                    dot(vrel, vrel).sqrt()
                }
            })
            .collect();
        let erosivo: Vec<bool> = tramos
            .iter()
            .zip(&v_tramo)
            .map(|(t, &v)| margen_erosivo(&cfg, v / CM_ANIO, t.s_area / t.a_sub))
            .collect();
        // Distancia a la fosa en la placa de arriba, desde cada tramo y sin salir de la placa.
        let fuentes: Vec<(u32, f64, u32)> = tramos
            .iter()
            .enumerate()
            .map(|(i, t)| (t.k, 0.5 * self.malla.lado_km, i as u32))
            .collect();
        let max_d = cfg.d_arco_km + 0.5 * cfg.w_arco_km;
        let dueno = &self.dueno;
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(&self.malla, &fuentes, |u, v| dueno[u] == dueno[v], max_d);
        let (lo, hi) = (
            cfg.d_arco_km - 0.5 * cfg.w_arco_km,
            cfg.d_arco_km + 0.5 * cfg.w_arco_km,
        );
        // Celdas de cada tramo en orden de distancia (desempate por celda) y franja del arco.
        let mut orden: Vec<(u32, u32)> = Vec::new();
        let mut area_franja = vec![0.0; tramos.len()];
        for &k in &campo.orden {
            let ku = k as usize;
            let f = campo.fuente[ku];
            if f == NADA || self.dueno[ku] != self.dueno[tramos[f as usize].k as usize] {
                continue;
            }
            orden.push((k, f));
            let d = campo.dist[ku];
            if d >= lo && d <= hi {
                area_franja[f as usize] += self.malla.area[ku];
            }
        }
        let dist: Vec<f64> = orden.iter().map(|&(k, _)| campo.dist[k as usize]).collect();
        self.campo = campo;
        // En un borde dentado, la franja de detrás se la quedan los tramos de los dientes: el de una
        // muesca no tiene franja propia. Su erosión por subducción actúa desde el tramo con franja
        // más cercano de su placa, si está a menos de `d_arco + w_arco/2`.
        let con_franja: Vec<usize> = (0..tramos.len())
            .filter(|&i| area_franja[i] > 0.0)
            .collect();
        let cos_max = (max_d / r).cos();
        let destino: Vec<usize> = (0..tramos.len())
            .map(|i| {
                if area_franja[i] > 0.0 {
                    return i;
                }
                let ki = tramos[i].k as usize;
                let xi = self.malla.dir[ki];
                con_franja
                    .iter()
                    .copied()
                    .filter(|&j| self.dueno[tramos[j].k as usize] == self.dueno[ki])
                    .map(|j| (dot(xi, self.malla.dir[tramos[j].k as usize]), j))
                    .filter(|&(c, _)| c >= cos_max)
                    .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
                    .map_or(i, |(_, j)| j)
            })
            .collect();
        let mut a_se = vec![0.0; tramos.len()];
        for (i, t) in tramos.iter().enumerate() {
            a_se[destino[i]] += t.a_sub;
        }
        // Reparto del arco (6.1): cada placa de arriba recibe exactamente `Σ A_sub` de sus tramos,
        // repartido por su franja con el peso de la convergencia perpendicular al borde del tramo
        // de cada celda (suave, de la cinemática de las placas; uniforme si ningún tramo converge).
        // Así un tramo de muesca no concentra su arco en el vecino y el engrosamiento sigue a la
        // convergencia.
        let q: Vec<f64> = v_tramo.clone();
        let np = self.placas.len();
        let (mut s_placa, mut w_placa) = (vec![0.0; np], vec![0.0; np]);
        for t in tramos {
            s_placa[self.dueno[t.k as usize] as usize] += t.a_sub;
        }
        for (&(k, f), &d) in orden.iter().zip(&dist) {
            if d >= lo && d <= hi {
                w_placa[self.dueno[k as usize] as usize] +=
                    self.malla.area[k as usize] * q[f as usize];
            }
        }
        // Placa con franja pero sin ningún tramo convergente: reparto uniforme.
        let mut q = q;
        let uniforme: Vec<bool> = (0..np)
            .map(|pl| w_placa[pl] <= 0.0 && s_placa[pl] > 0.0)
            .collect();
        for (&(k, f), &d) in orden.iter().zip(&dist) {
            let pl = self.dueno[k as usize] as usize;
            if uniforme[pl] {
                q[f as usize] = 1.0;
                if d >= lo && d <= hi {
                    w_placa[pl] += self.malla.area[k as usize];
                }
            }
        }
        let inv = self.indice_inverso();
        let h_cont_min = cfg.h_cont_min_km;
        // 6.1 Arcos: el volumen de cada celda de la franja (sin tope: la corteza que engorda de más
        // se desparrama con el flujo lateral, adenda v3 §6.6b).
        let mut arco_dh: Vec<(u32, f64)> = Vec::new();
        for (&(k, f), &d) in orden.iter().zip(&dist) {
            if d < lo || d > hi {
                continue;
            }
            let ku = k as usize;
            let pl = self.dueno[ku] as usize;
            let continental = self.corteza(ku).is_some_and(|c| c.continente());
            let p_arc = if continental {
                cfg.p_arc_cont
            } else {
                cfg.p_arc_oc
            };
            let vol = p_arc * s_placa[pl] * self.malla.area[ku] * q[f as usize] / w_placa[pl];
            if vol > 0.0 {
                arco_dh.push((k, vol));
            }
        }
        arco_dh.sort_unstable_by_key(|x| x.0);
        let mut pasa_a_cont = 0.0;
        let mut reinicio = 0.0;
        for (k, vol) in arco_dh {
            let ku = k as usize;
            let mut a_cont = 0.0;
            let puesto = self.en_celda(&inv, ku, vol, false, |c, a, dh| {
                if c.sumar_hc(dh, 1.0, 0.0, rs, rc, h_cont_min, via::ARCO) {
                    a_cont += a;
                }
                c.dh_dt += dh / cfg.dt_myr;
                c.edad_orog = 0.0;
                // La franja del arco que recibe corteza se calienta: su edad tectonotérmica vuelve
                // a 0 (nota de isostasia con litosfera §6.2).
                if c.cont {
                    reinicio += a;
                }
                c.edad_tt = 0.0;
            });
            self.cuentas.arcos_km3 += puesto;
            pasa_a_cont += a_cont;
        }
        self.reinicio_km2_paso[1] += reinicio;
        for t in tramos {
            if w_placa[self.dueno[t.k as usize] as usize] <= 0.0 {
                let continental = self.corteza(t.k as usize).is_some_and(|c| c.continente());
                let p_arc = if continental {
                    cfg.p_arc_cont
                } else {
                    cfg.p_arc_oc
                };
                self.cuentas.arcos_sin_franja_km3 += p_arc * t.a_sub;
            }
        }
        if pasa_a_cont > 0.0 {
            self.cuentas.arco_a_continental_km2 += pasa_a_cont;
            self.evento("acrecion_arco", vec![], 0.0, pasa_a_cont, None);
        }
        // 6.3 Erosión por subducción: de la celda más cercana a la fosa hacia dentro.
        let mut pendiente: Vec<f64> = a_se.iter().map(|&a| cfg.p_se * a).collect();
        let mut retroceso: Vec<(u32, f64, u32)> = Vec::new();
        for &(k, f) in &orden {
            let fi = f as usize;
            if pendiente[fi] <= 0.0 {
                continue;
            }
            let edad = tramos[fi].edad_area / tramos[fi].a_sub.max(1e-12);
            let (quitado, a_perdida) = self.tomar_hc(&inv, k as usize, pendiente[fi], edad);
            pendiente[fi] -= quitado;
            self.cuentas.erosion_subduccion_km3 += quitado;
            if a_perdida > 0.0 {
                retroceso.push((self.dueno[k as usize], a_perdida, k));
            }
        }
        self.cuentas.erosion_subduccion_sin_corteza_km3 +=
            pendiente.iter().map(|x| x.max(0.0)).sum::<f64>();
        // 6.4 Sedimento que llega a la fosa: en un margen acrecionario, `f_acr` se pega a la placa
        // de arriba y el resto se hunde; en uno erosivo, todo se hunde. Lo que se pega forma el
        // prisma: por igual en el antearco del tramo (de la fosa hasta la franja del arco), como un
        // prisma real de 50–200 km; si no hay antearco, en la celda del frente.
        let mut antearco: Vec<Vec<u32>> = vec![Vec::new(); tramos.len()];
        for (&(k, f), &d) in orden.iter().zip(&dist) {
            if d < lo {
                antearco[f as usize].push(k);
            }
        }
        for (i, t) in tramos.iter().enumerate() {
            if erosivo[i] {
                self.area_erosiva_km2 += t.a_sub;
            }
            if t.v_llega <= 0.0 {
                continue;
            }
            self.cuentas.sedimento_a_fosa_km3 += t.v_llega;
            let mut pegado = 0.0;
            if !erosivo[i] {
                let vol = cfg.f_acr * t.v_llega;
                let celdas = if antearco[i].is_empty() {
                    vec![t.k]
                } else {
                    antearco[i].clone()
                };
                let a_tot: f64 = celdas.iter().map(|&k| self.malla.area[k as usize]).sum();
                for k in celdas {
                    let v = vol * self.malla.area[k as usize] / a_tot;
                    pegado += self.en_celda(&inv, k as usize, v, false, |c, _, dh| {
                        // El sedimento acrecionado no es de arco (adenda v9).
                        c.sumar_hc(dh, 0.0, 0.0, rs, rc, h_cont_min, via::ACRECION);
                    });
                }
            }
            self.cuentas.sedimento_acrecionado_km3 += pegado;
            self.cuentas.sedimento_subducido_km3 += t.v_llega - pegado;
        }
        self.eventos_de_margen(tramos, &erosivo, &retroceso);
    }

    /// Agrupa el consumo de cada celda de frente en la celda de borde más cercana de la placa de
    /// arriba (sin salir de ella): cuando la placa de abajo pierde más de una fila en un paso, las
    /// filas de dentro son de la misma fosa. Cada tramo queda así en el borde, con su franja de
    /// arco detrás (orden fijo por celda).
    ///
    /// Un frente a menos de `L_borde` de una fosa del mismo par de placas es de esa fosa (adenda
    /// v5): donde la fosa gira y el borde pasa a deslizarse, la celda de borde más cercana ya es de
    /// la transformante, pero lo que se hunde entra por la fosa.
    fn tramos_en_el_borde(&mut self, frentes: &[Tramo], borde_en: &[u32]) -> Vec<Tramo> {
        let fuentes: Vec<(u32, f64, u32)> = self.bordes.iter().map(|b| (b.k, 0.0, b.k)).collect();
        let dueno = &self.dueno;
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(
            &self.malla,
            &fuentes,
            |u, v| dueno[u] == dueno[v],
            3.0 * self.malla.lado_km,
        );
        let es_fosa = |j: u32, arriba: u32, abajo: u32| {
            let i = borde_en[j as usize];
            i != NADA && {
                let b = &self.bordes[i as usize];
                b.tipo == TipoBorde::Subduccion && b.p == arriba && b.q == abajo
            }
        };
        let mut tramos: Vec<Tramo> = frentes
            .iter()
            .map(|t| {
                let f = campo.fuente[t.k as usize];
                let k = if f != NADA && borde_en[f as usize] != NADA {
                    f
                } else {
                    t.k
                };
                let arriba = self.dueno[t.k as usize];
                if es_fosa(k, arriba, t.placa) {
                    return Tramo { k, ..*t };
                }
                let x = self.malla.dir[t.k as usize];
                let fosa = self
                    .vecindad
                    .de(t.k as usize)
                    .iter()
                    .copied()
                    .filter(|&j| self.dueno[j as usize] == arriba && es_fosa(j, arriba, t.placa))
                    .max_by(|&a, &b| {
                        dot(x, self.malla.dir[a as usize])
                            .total_cmp(&dot(x, self.malla.dir[b as usize]))
                            .then(b.cmp(&a))
                    });
                Tramo {
                    k: fosa.unwrap_or(k),
                    ..*t
                }
            })
            .collect();
        self.campo = campo;
        tramos.sort_unstable_by_key(|t| (t.k, t.placa));
        let mut out: Vec<Tramo> = Vec::new();
        for t in tramos {
            match out.last_mut() {
                Some(u) if u.k == t.k => {
                    u.a_sub += t.a_sub;
                    u.v_llega += t.v_llega;
                    u.s_area += t.s_area;
                    u.edad_area += t.edad_area;
                    if u.placa == t.placa {
                        u.a_placa += t.a_placa;
                    } else if t.a_placa > u.a_placa {
                        u.placa = t.placa;
                        u.a_placa = t.a_placa;
                    }
                }
                _ => out.push(t),
            }
        }
        out
    }

    /// Quita hasta `vol` km³ de corteza de tipo continental de las muestras del dueño de la celda
    /// del mundo `k` (en proporción a la que tiene cada una: lo quitado es exacto). Devuelve lo
    /// quitado y el área de las muestras que se quedaron sin corteza continental (pasan a fondo
    /// oceánico de edad `edad`).
    fn tomar_hc(
        &mut self,
        inv: &(Vec<u32>, Vec<u32>),
        k: usize,
        vol: f64,
        edad: f64,
    ) -> (f64, f64) {
        let pi = self.dueno[k];
        if pi == NADA || vol <= 0.0 {
            return (0.0, 0.0);
        }
        let h_oc = self.cfg.h_oc_km;
        let (off, lista) = inv;
        let (a, b) = (off[k] as usize, off[k + 1] as usize);
        let p = &mut self.placas[pi as usize];
        let rep = [p.mapa[self.rep[k] as usize]];
        let muestras: &[u32] = if a == b { &rep } else { &lista[a..b] };
        if muestras[0] == NADA {
            return (0.0, 0.0);
        }
        let disponible: f64 = muestras
            .iter()
            .map(|&s| p.cort[s as usize].hc * self.malla.area[p.celda[s as usize] as usize])
            .sum();
        if disponible <= 0.0 {
            return (0.0, 0.0);
        }
        let f = (vol / disponible).min(1.0);
        let (mut quitado, mut a_perdida) = (0.0, 0.0);
        for &s in muestras {
            let ar = self.malla.area[p.celda[s as usize] as usize];
            let c = &mut p.cort[s as usize];
            let dh = if f >= 1.0 { c.hc } else { c.hc * f };
            quitado += dh * ar;
            if c.quitar_hc(dh, h_oc, edad) {
                a_perdida += ar;
            }
        }
        (quitado, a_perdida)
    }

    /// Eventos de los márgenes (adenda de balance §7), agregados por paso: cambio de régimen de un
    /// par de placas (el de la mayor parte de su área subducida) y retroceso del margen.
    fn eventos_de_margen(
        &mut self,
        tramos: &[Tramo],
        erosivo: &[bool],
        retroceso: &[(u32, f64, u32)],
    ) {
        // (id de arriba, id de abajo) → (área erosiva, área total, una celda).
        let mut pares: Vec<((u64, u64), f64, f64, u32)> = Vec::new();
        for (t, &e) in tramos.iter().zip(erosivo) {
            let arriba = self.dueno[t.k as usize];
            if arriba == NADA {
                continue;
            }
            let clave = (
                self.placas[arriba as usize].id,
                self.placas[t.placa as usize].id,
            );
            match pares.iter_mut().find(|x| x.0 == clave) {
                Some(x) => {
                    x.1 += if e { t.a_sub } else { 0.0 };
                    x.2 += t.a_sub;
                }
                None => pares.push((clave, if e { t.a_sub } else { 0.0 }, t.a_sub, t.k)),
            }
        }
        for ((arriba, abajo), a_e, a_t, k) in pares {
            let ero = a_e > 0.5 * a_t;
            let i = self.par_idx(arriba, abajo);
            let antes = self.pares[i].regimen;
            self.pares[i].regimen = Some((arriba, ero));
            if antes.is_some_and(|(a, e)| a == arriba && e != ero) {
                let largo: f64 = self
                    .bordes
                    .iter()
                    .filter(|b| {
                        b.tipo == TipoBorde::Subduccion
                            && b.p_subduce
                            && self.placas[b.p as usize].id == abajo
                            && self.placas[b.q as usize].id == arriba
                    })
                    .map(|b| b.largo_km)
                    .sum();
                let clase = if ero { "erosivo" } else { "acrecionario" };
                let d = self.malla.dir[k as usize];
                self.evento_det(
                    "cambio_regimen",
                    vec![arriba, abajo],
                    largo,
                    a_t,
                    Some(d),
                    Detalle {
                        clase: Some(clase.to_string()),
                        ..Detalle::default()
                    },
                );
            }
        }
        let mut por_placa: Vec<(u32, f64, u32)> = Vec::new();
        for &(p, a, k) in retroceso {
            match por_placa.iter_mut().find(|x| x.0 == p) {
                Some(x) => x.1 += a,
                None => por_placa.push((p, a, k)),
            }
        }
        for (p, a, k) in por_placa {
            let id = self.placas[p as usize].id;
            let d = self.malla.dir[k as usize];
            self.evento("retroceso_margen", vec![id], 0.0, a, Some(d));
        }
    }

    // --- d. Choques y mesetas (§6.4) -----------------------------------------------------------

    /// La corteza que la placa de abajo mete bajo la de arriba (`η·V_sol`, adenda de balance §6.5)
    /// engruesa a esta desde el frente hacia dentro: uniforme en `W_min_orog`, sin pasar de `H_max`
    /// (el exceso sigue hacia el interior). La de un continente solo engruesa corteza continental;
    /// la de un arco o un prisma sobre fondo oceánico, cualquier corteza de la placa de arriba.
    fn engrosar_choques(&mut self, choques: &[Choque]) {
        // La franja de un choque es toda la corteza de la placa de arriba a menos de W_min_orog:
        // si su corteza continental es una celda suelta (un arco ya continental en una placa
        // oceánica), restringirla a la continental mete en esa celda todo el continente de abajo.
        // Dónde ocurre el solape, por el tipo de borde de la celda del frente (diagnóstico).
        let mut tipo_en = vec![0u8; self.malla.num];
        for b in &self.bordes {
            tipo_en[b.k as usize] = b.tipo.codigo();
        }
        self.solape_en_borde_paso.clear();
        for c in choques {
            let v = c.vol / c.eta.max(1e-12);
            self.cuentas.choque_solapado_por_borde_km3[tipo_en[c.k as usize] as usize] += v;
            self.solape_en_borde_paso
                .push((c.k, tipo_en[c.k as usize], v));
        }
        if !choques.is_empty() {
            self.engrosar_desde(choques, false);
        }
    }

    fn engrosar_desde(&mut self, choques: &[Choque], solo_cont: bool) {
        let cfg = self.cfg.clone();
        // Volumen por celda de frente y si todo él es de arco.
        let mut por_frente: Vec<Choque> = choques.to_vec();
        por_frente.sort_unstable_by_key(|x| x.k);
        let mut agrupado: Vec<(u32, f64, f64)> = Vec::new();
        for c in por_frente {
            match agrupado.last_mut() {
                Some(u) if u.0 == c.k => {
                    u.1 += c.vol;
                    u.2 += c.vol_arco;
                }
                _ => agrupado.push((c.k, c.vol, c.vol_arco)),
            }
        }
        let fuentes: Vec<(u32, f64, u32)> = agrupado
            .iter()
            .enumerate()
            .map(|(i, &(k, _, _))| (k, 0.0, i as u32))
            .collect();
        let dueno = self.dueno.clone();
        let fuente_placa: Vec<u32> = agrupado
            .iter()
            .map(|&(k, _, _)| dueno[k as usize])
            .collect();
        let cabe: Vec<bool> = (0..self.malla.num)
            .map(|k| self.corteza(k).is_some_and(|c| c.cont || !solo_cont))
            .collect();
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(
            &self.malla,
            &fuentes,
            |u, v| dueno[u] == dueno[v] && cabe[v],
            cfg.w_min_orog_km,
        );
        // Franja de W_min_orog detrás de los frentes, en la placa de arriba, partida en zonas de
        // choque (componentes conexas). El volumen de cada zona se reparte por igual en su franja
        // («uniforme en W_min_orog», §6.4 del principal): con una cuenca por frente, los frentes de
        // dentro de un frente de dos filas se quedan con una sola celda y la apilan.
        let num = self.malla.num;
        let mut en_franja = vec![false; num];
        let mut franja: Vec<u32> = Vec::new();
        for &k in &campo.orden {
            let ku = k as usize;
            let f = campo.fuente[ku];
            if f != NADA
                && dueno[ku] == fuente_placa[f as usize]
                && (campo.dist[ku] < cfg.w_min_orog_km || campo.dist[ku] == 0.0)
            {
                en_franja[ku] = true;
                franja.push(k);
            }
        }
        self.campo = campo;
        franja.sort_unstable();
        let mut zona = vec![NADA; num];
        let mut nz = 0u32;
        let mut pila: Vec<u32> = Vec::new();
        for &k in &franja {
            if zona[k as usize] != NADA {
                continue;
            }
            zona[k as usize] = nz;
            pila.push(k);
            while let Some(c) = pila.pop() {
                for &v in &self.malla.vec[c as usize] {
                    if v != NADA
                        && en_franja[v as usize]
                        && zona[v as usize] == NADA
                        && dueno[v as usize] == dueno[c as usize]
                    {
                        zona[v as usize] = nz;
                        pila.push(v);
                    }
                }
            }
            nz += 1;
        }
        let mut vol_zona = vec![0.0; nz as usize];
        let mut area_zona = vec![0.0; nz as usize];
        let mut vol_arco_zona = vec![0.0; nz as usize];
        let mut sin_sitio = 0.0;
        for &(k, v, v_arco) in &agrupado {
            let z = zona[k as usize];
            if z == NADA {
                sin_sitio += v;
                continue;
            }
            vol_zona[z as usize] += v;
            vol_arco_zona[z as usize] += v_arco;
        }
        for &k in &franja {
            area_zona[zona[k as usize] as usize] += self.malla.area[k as usize];
        }

        let (rs, rc) = (cfg.rho_s, cfg.rho_c);
        let inv = self.indice_inverso();
        let mut dh_mundo: Vec<(u32, f64, f64)> = Vec::new();
        for &k in &franja {
            let z = zona[k as usize] as usize;
            if vol_zona[z] > 0.0 {
                dh_mundo.push((
                    k,
                    vol_zona[z] * self.malla.area[k as usize] / area_zona[z],
                    (vol_arco_zona[z] / vol_zona[z]).clamp(0.0, 1.0),
                ));
                // Diagnóstico de la v9: corteza de arco que un choque deja sobre un continente o
                // fuera de él (sobre fondo oceánico o sobre otro arco).
                let en_continente = self.corteza(k as usize).is_some_and(|c| c.continente());
                self.diag_arco_choque_km3[usize::from(!en_continente)] +=
                    vol_arco_zona[z] * self.malla.area[k as usize] / area_zona[z];
            }
        }
        self.cuentas.choque_sin_sitio_km3 += sin_sitio;
        dh_mundo.sort_unstable_by_key(|x| x.0);
        let h_cont_min = cfg.h_cont_min_km;
        let mut reinicio = 0.0;
        for (k, vol, fa) in dh_mundo {
            let puesto = self.en_celda(&inv, k as usize, vol, solo_cont, |c, a, dh| {
                // Lo que deja un choque sobre un continente se une a él: fracción de arco 0 (adenda
                // v9). Fuera de un continente (sobre fondo oceánico o sobre otro arco), la corteza de
                // arco aún no se ha unido a ninguno y sigue siendo de arco (incremento-4-diseno.md
                // §60).
                let f_choque = if c.continente() { 0.0 } else { fa };
                c.sumar_hc(dh, f_choque, 0.0, rs, rc, h_cont_min, via::CHOQUE);
                c.dh_dt += dh / cfg.dt_myr;
                c.edad_orog = 0.0;
                // La franja que engruesa el choque se calienta (nota de isostasia con litosfera
                // §6.2).
                if c.cont {
                    reinicio += a;
                }
                c.edad_tt = 0.0;
            });
            self.cuentas.choque_engrosamiento_km3 += puesto;
            self.cuentas.choque_sin_sitio_km3 += vol - puesto;
        }
        self.reinicio_km2_paso[2] += reinicio;
    }

    /// Delaminación (adenda de balance v5 §6.6): por encima de `h_del` la base de la corteza pasa a
    /// eclogita, más densa que el manto, y se desprende. La caída de la raíz de los arcos no va
    /// aparte: ya está en las tasas netas de los arcos. El exceso decae con la forma exponencial
    /// exacta, estable con cualquier `Δt`: `h_c ← h_del + (h_c − h_del)·e^(−Δt/τ_del)`.
    fn delaminar(&mut self) {
        let cfg = &self.cfg;
        let mut total = 0.0;
        let mut reinicio = 0.0;
        for p in self.placas.iter_mut().filter(|p| p.viva) {
            for s in 0..p.cort.len() {
                let c = &mut p.cort[s];
                if c.hc > cfg.h_del_km {
                    let dh = c.hc - tras_delaminar(c.hc, cfg.h_del_km, cfg.dt_myr, cfg.tau_del_myr);
                    let antes = c.hc;
                    c.hc -= dh;
                    c.h -= dh;
                    c.anotar_hc(antes, -dh, 0);
                    let a = self.malla.area[p.celda[s] as usize];
                    total += dh * a;
                    // Al desprenderse la raíz sube astenosfera caliente: la columna que pierde más
                    // de ε_del se calienta (nota de isostasia con litosfera §6.2).
                    if dh > cfg.eps_del_km {
                        c.edad_tt = 0.0;
                        if c.cont {
                            reinicio += a;
                        }
                    }
                }
            }
        }
        self.cuentas.delaminacion_km3 += total;
        self.reinicio_km2_paso[3] += reinicio;
    }

    /// Flujo lateral de la corteza engrosada (adenda de balance v6 §6.6b): la corteza inferior de
    /// una columna de más de `h_flujo`, caliente y débil, se desparrama hacia sus vecinas de lado de
    /// la misma placa con corteza de tipo continental y menos espesor:
    /// `V_ik = D_0·(c_i/c_ref)³ · (h_c,i − h_c,k) / d_ik · ℓ_ik · Δt`, con `c_i = h_c,i − h_flujo` el
    /// espesor del canal débil: el flujo en un canal crece con el cubo de su espesor (Clark y Royden
    /// 2000), así que los engrosamientos excesivos se aplanan mucho más deprisa que los moderados.
    ///
    /// Va por la malla del mundo, entre celdas vecinas de la misma placa y con la muestra que ve cada
    /// una: la rejilla propia de una placa está girada respecto a la del mundo, y la corteza nueva o
    /// recolocada queda en celdas sueltas de esa rejilla, sin vecinas con las que fluir. En un borde
    /// de choque continente–continente también fluye a la otra placa (el orógeno abarca las dos).
    ///
    /// Las muestras ocultas (las de la placa que ninguna celda del mundo ve este paso) también
    /// fluyen (adenda v5): la corteza es corteza, se vea o no. Cada una intercambia con la muestra
    /// visible de la celda donde cae su centro, con la misma ley y el acoplamiento de un lado de
    /// celda (`ℓ = lado`, `d = d_min`).
    ///
    /// Cada subpaso calcula todos los `V_ik` con los espesores de su inicio y los aplica a la vez
    /// (función pura del estado; conserva el volumen por construcción). Los subpasos se adaptan al
    /// mayor `D_ef = D_0·(c/c_ref)³` de cada uno, con `D_ef·Δt_sub/d² ≤ 1/16` (una muestra puede
    /// verla más de una celda).
    fn flujo_lateral(&mut self) {
        let cfg = &self.cfg;
        if cfg.d0_flujo_km2_myr <= 0.0 {
            return;
        }
        let malla = &self.malla;
        let d2 = malla.dist_lado_min_km * malla.dist_lado_min_km;
        let (h_flujo, h_cont_min) = (cfg.h_flujo_km, cfg.h_cont_min_km);
        let (d0, c_ref) = (cfg.d0_flujo_km2_myr, cfg.c_ref_km);
        let difus = |hi: f64| d0 * ((hi - h_flujo) / c_ref).powi(3);
        // Celdas fuente: las que ven una muestra de más de h_flujo (y las que lo pasen al recibir).
        let hc_de = |placas: &[Placa], k: usize, dueno: &[u32], rep: &[u32]| -> Option<f64> {
            let p = dueno[k];
            if p == NADA || rep[k] == NADA {
                return None;
            }
            placas[p as usize].get(rep[k]).map(|c| c.hc)
        };
        let mut fuente: Vec<u32> = (0..malla.num as u32)
            .filter(|&k| {
                hc_de(&self.placas, k as usize, &self.dueno, &self.rep).is_some_and(|h| h > h_flujo)
            })
            .collect();
        let mut es_fuente = vec![false; malla.num];
        for &k in &fuente {
            es_fuente[k as usize] = true;
        }
        // Parejas (placa, celda local de la visible, celda local de una oculta en la misma celda del
        // mundo) con corteza de tipo continental en alguna de las dos.
        let (off, lista) = self.indice_inverso();
        // Solo cuentan las parejas que pueden fluir: la de una celda fuente o con la oculta de más de
        // h_flujo. Las demás (casi todas, en el interior de los continentes) se añaden si su celda
        // pasa a ser fuente al recibir.
        let parejas_de = |d: &Deriva, k: usize| -> Vec<(u32, u32, u32)> {
            let (p, lr) = (d.dueno[k], d.rep[k]);
            let mut v = Vec::new();
            if p == NADA || lr == NADA {
                return v;
            }
            let pl = &d.placas[p as usize];
            let Some(cr) = pl.get(lr) else {
                return v;
            };
            for &sm in &lista[off[k] as usize..off[k + 1] as usize] {
                let lh = pl.celda[sm as usize];
                if lh != lr && (cr.hc > 0.0 || pl.cort[sm as usize].hc > 0.0) {
                    v.push((p, lr, lh));
                }
            }
            v
        };
        let mut ocultas: Vec<(u32, u32, u32)> = Vec::new();
        for k in 0..malla.num {
            let es_fuente = fuente.binary_search(&(k as u32)).is_ok();
            for par in parejas_de(self, k) {
                let pl = &self.placas[par.0 as usize];
                if es_fuente || pl.get(par.2).is_some_and(|c| c.hc > h_flujo) {
                    ocultas.push(par);
                }
            }
        }
        if fuente.is_empty() && ocultas.is_empty() {
            return;
        }
        let (d_min, lado) = (malla.dist_lado_min_km, malla.lado_km);
        // Celdas de un borde de choque continente–continente: ahí el orógeno abarca las dos placas
        // y la corteza inferior también fluye a través de la sutura (si no, una astilla de la placa
        // de arriba, rodeada de otras placas, se queda con toda la corteza que se mete debajo).
        let mut choque_con: HashMap<u32, u32> = HashMap::new();
        for b in &self.bordes {
            if b.tipo == TipoBorde::Colision {
                choque_con.insert(b.k, b.q);
            }
        }
        // Las muestras que intervienen, en un índice compacto (ordenado por placa y celda local, el
        // orden en que se aplican los cambios), y los lados de cada celda fuente: se rehacen solo
        // cuando entra una celda fuente nueva. Cada subpaso hace las mismas cuentas y en el mismo
        // orden que recorrer las celdas del mundo una a una, sin buscar las muestras cada vez.
        let mut red = RedFlujo::default();
        let mut rehacer = true;
        let mut recibe: Vec<u32> = Vec::new();
        let mut movido = 0.0;
        let mut resta = cfg.dt_myr;
        while resta > 1e-12 {
            if rehacer {
                let t0 = std::time::Instant::now();
                red.ampliar(self, &fuente, &ocultas, &choque_con);
                self.flujo_seg[0] += t0.elapsed().as_secs_f64();
                self.flujo_rehechos += 1;
                rehacer = false;
            }
            let t1 = std::time::Instant::now();
            // Subpaso estable con el canal más grueso de ahora.
            let h_max = red.hc.iter().fold(h_flujo, |m, &h| m.max(h));
            if h_max <= h_flujo {
                break;
            }
            let dt_sub = resta.min(d2 / (16.0 * difus(h_max)));
            resta -= dt_sub;
            self.flujo_subpasos += 1;
            self.flujo_muestras += red.muestras.len() as u64;
            self.flujo_lados += red.lados.len() as u64;
            self.flujo_subpasos_por_hmax[(((h_max - h_flujo) / 10.0) as usize).min(5)] += 1;
            for g in &red.grupos {
                let hi = red.hc[g.i as usize];
                if hi <= h_flujo {
                    continue;
                }
                for e in &red.lados[g.desde as usize..g.hasta as usize] {
                    let hk = red.hc[e.j as usize];
                    if hk <= 0.0 || hk >= hi {
                        continue;
                    }
                    let v = difus(hi) * (hi - hk) / e.d * e.l * dt_sub;
                    red.delta[g.i as usize] -= v;
                    red.delta[e.j as usize] += v;
                    let va = v * red.arco[g.i as usize];
                    red.delta_arco[g.i as usize] -= va;
                    red.delta_arco[e.j as usize] += va;
                    let ve = v * red.edad[g.i as usize];
                    red.delta_edad[g.i as usize] -= ve;
                    red.delta_edad[e.j as usize] += ve;
                    // Solo hace falta mirar después las que aún no son fuente.
                    if !es_fuente[e.k2 as usize] {
                        recibe.push(e.k2);
                    }
                }
            }
            // Entre la muestra visible de cada celda y sus ocultas.
            for &(ir, ih) in &red.parejas {
                let (cr, ch) = (red.hc[ir as usize], red.hc[ih as usize]);
                let ((hi, li), (hk, lk)) = if cr >= ch {
                    ((cr, ir), (ch, ih))
                } else {
                    ((ch, ih), (cr, ir))
                };
                if hi <= h_flujo || hk <= 0.0 || hk >= hi {
                    continue;
                }
                let v = difus(hi) * (hi - hk) / d_min * lado * dt_sub;
                red.delta[li as usize] -= v;
                red.delta[lk as usize] += v;
                let va = v * red.arco[li as usize];
                red.delta_arco[li as usize] -= va;
                red.delta_arco[lk as usize] += va;
                let ve = v * red.edad[li as usize];
                red.delta_edad[li as usize] -= ve;
                red.delta_edad[lk as usize] += ve;
            }
            for x in 0..red.orden.len() {
                let i = red.orden[x] as usize;
                let dv = red.delta[i];
                if dv == 0.0 {
                    continue;
                }
                red.delta[i] = 0.0;
                let dva = red.delta_arco[i];
                red.delta_arco[i] = 0.0;
                let dve = red.delta_edad[i];
                red.delta_edad[i] = 0.0;
                let (p, l) = red.muestras[i];
                let a = malla.area[l as usize];
                let sp = red.pos[i];
                if sp == NADA {
                    continue;
                }
                let c = &mut self.placas[p as usize].cort[sp as usize];
                let antes = c.hc;
                c.hc += dv / a;
                c.h += dv / a;
                c.anotar_hc(antes, dv / a, via::FLUJO);
                c.arco = if c.hc > 0.0 {
                    ((c.arco * antes * a + dva) / (c.hc * a)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                if c.hc > 0.0 {
                    c.edad_tt = ((c.edad_tt * antes * a + dve) / (c.hc * a)).max(0.0);
                }
                red.arco[i] = c.arco;
                red.edad[i] = c.edad_tt;
                red.hc[i] = c.hc;
                if dv > 0.0 {
                    movido += dv;
                    if !c.cont && c.h > h_cont_min {
                        c.cont = true;
                    }
                }
            }
            // Las celdas que pasaron de h_flujo al recibir también son fuente, con sus ocultas.
            recibe.sort_unstable();
            recibe.dedup();
            for &k in &recibe {
                if fuente.binary_search(&k).is_ok() {
                    continue;
                }
                if hc_de(&self.placas, k as usize, &self.dueno, &self.rep)
                    .is_some_and(|h| h > h_flujo)
                {
                    let i = fuente.partition_point(|&x| x < k);
                    fuente.insert(i, k);
                    es_fuente[k as usize] = true;
                    for par in parejas_de(self, k as usize) {
                        if !ocultas.contains(&par) {
                            ocultas.push(par);
                        }
                    }
                    rehacer = true;
                }
            }
            recibe.clear();
            self.flujo_seg[1] += t1.elapsed().as_secs_f64();
        }
        self.cuentas.flujo_lateral_km3 += movido;
    }

    // --- e. Erosión tectónica, envejecimiento y sedimento (§6.6; adenda de balance §6.7) -------

    /// Envejece la corteza y gasta la tierra emergida (adenda de balance v5 §6.7): donde la columna
    /// asoma (`z > 0` sobre el mar) se retira `E = E_1·(z/1 km)^n_ero` km/Myr de roca, primero el
    /// sedimento y luego la corteza de tipo continental (la base basáltica no se erosiona). En la
    /// Tierra la erosión crece mucho más que proporcional con el relieve en las cordilleras activas y
    /// es de metros por Myr en llanuras y escudos (Montgomery y Brandon 2002); la altura sobre el mar
    /// hace de relieve local. Lo retirado en un paso no baja la columna del nivel del mar (con su
    /// rebote isostático). Devuelve la roca erosionada en cada celda del mundo (km³).
    fn erosion_y_envejecimiento(&mut self) -> Vec<(u32, f64)> {
        // El fondo de una cuenca cerrada (por debajo de su cota de desagüe) no se erosiona: allí se
        // deposita. Si no, se lleva su propio sedimento, que vuelve a la cuenca, y la corteza de
        // debajo no se gasta nunca.
        let z = self.alturas();
        let abierto = self.mar_abierto(&z, self.zeta_m);
        let (zf, _, _) = self.inundacion(&z, &abierto);
        let cuenca: Vec<bool> = (0..self.malla.num)
            .map(|k| zf[k].is_finite() && zf[k] > z[k] + 1.0)
            .collect();
        // La celda del mundo que ve cada muestra (la que la tiene de representante; si no la ve
        // ninguna, la de su centro): la cuenca cerrada es la de esa celda, no la de su centro, que a
        // menudo es la vecina (una muestra alta junto a una depresión no se erosionaba nunca).
        let mut ve: Vec<Vec<u32>> = self
            .placas
            .iter()
            .map(|p| if p.viva { p.mundo.clone() } else { Vec::new() })
            .collect();
        for k in 0..self.malla.num {
            let (pi, l) = (self.dueno[k], self.rep[k]);
            if pi == NADA || l == NADA {
                continue;
            }
            let s = self.placas[pi as usize].mapa[l as usize];
            if s != NADA {
                ve[pi as usize][s as usize] = k as u32;
            }
        }
        let cfg = &self.cfg;
        let (rs, rc) = (cfg.rho_s, cfg.rho_c);
        let zeta = self.zeta_m;
        let mut ero: Vec<(u32, f64)> = Vec::new();
        let mut total = 0.0;
        for (pi, p) in self.placas.iter_mut().enumerate().filter(|(_, p)| p.viva) {
            for (s, &k_ve) in ve[pi].iter().enumerate() {
                let k_ve = k_ve as usize;
                let c = &mut p.cort[s];
                let zi = (isostasia::altura(cfg, c) - zeta) / 1000.0;
                if zi > 0.0 && !cuenca[k_ve] {
                    // Lo que baja la superficie por km de roca retirada, con su isostasia.
                    let baja = isostasia::baja_por_km(cfg, c);
                    let e = cfg.e1_km_myr * zi.powf(cfg.n_ero) * cfg.dt_myr;
                    let sed_roca = c.sed * rs / rc;
                    let roca = e.min(zi / baja).min(sed_roca + c.hc);
                    if roca > 0.0 {
                        if sed_roca >= roca {
                            c.sed -= roca * rc / rs;
                        } else {
                            let dh = roca - sed_roca;
                            let antes = c.hc;
                            c.sed = 0.0;
                            c.h -= dh;
                            c.hc -= dh;
                            c.anotar_hc(antes, -dh, 0);
                        }
                        let v = roca * self.malla.area[p.celda[s] as usize];
                        ero.push((k_ve as u32, v));
                        total += v;
                    }
                }
                if !c.cont {
                    c.edad_oc += cfg.dt_myr;
                }
                c.edad_orog += cfg.dt_myr;
            }
        }
        self.cuentas.erosion_km3 += total;
        ero.sort_unstable_by_key(|x| x.0);
        ero
    }

    /// Espesor de sedimento (km) que lleva la columna de la celda del mundo `k` hasta la altura
    /// `z_obj` (m), con su isostasia ([`isostasia::sedimento_hasta_columna`]).
    fn sed_hasta(&self, k: usize, z_obj: f64) -> f64 {
        self.corteza(k).map_or(0.0, |c0| {
            isostasia::sedimento_hasta_columna(&self.cfg, c0, z_obj)
        })
    }

    /// Mar abierto: las celdas de mar (`z < zeta`) conectadas al océano mundial, la mayor masa de
    /// mar conectada. Cualquier otro mar es cerrado y, para el sedimento, una cuenca
    /// ([`Deriva::inundacion`]), como el Caspio.
    fn mar_abierto(&self, z: &[f64], zeta: f64) -> Vec<bool> {
        let num = self.malla.num;
        let mut comp = vec![NADA; num];
        let mut areas: Vec<f64> = Vec::new();
        let mut pila: Vec<u32> = Vec::new();
        for k0 in 0..num {
            if z[k0] >= zeta || comp[k0] != NADA {
                continue;
            }
            let c = areas.len() as u32;
            let mut a = 0.0;
            comp[k0] = c;
            pila.push(k0 as u32);
            while let Some(k) = pila.pop() {
                a += self.malla.area[k as usize];
                for &v in &self.malla.vec[k as usize] {
                    if v != NADA && z[v as usize] < zeta && comp[v as usize] == NADA {
                        comp[v as usize] = c;
                        pila.push(v);
                    }
                }
            }
            areas.push(a);
        }
        let Some(mundial) = areas
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i as u32)
        else {
            return vec![false; num];
        };
        comp.iter().map(|&c| c == mundial).collect()
    }

    /// Inundación por prioridad desde la costa del mar abierto (Barnes et al. 2014) sobre las
    /// alturas `z`: cota de desagüe de cada celda que no es mar abierto (tierra o mar cerrado; `∞`
    /// si no llega al mar), la celda hacia la que drena y el orden de salida (de la cota de desagüe
    /// más baja a la más alta). Un mar cerrado es una cuenca: su cota de desagüe es la de su orilla.
    fn inundacion(&self, z: &[f64], abierto: &[bool]) -> (Vec<f64>, Vec<u32>, Vec<u32>) {
        let num = self.malla.num;
        let mut zf = vec![f64::INFINITY; num];
        let mut padre = vec![NADA; num];
        let mut heap = BinaryHeap::new();
        for k in 0..num {
            if abierto[k]
                && self.malla.vec[k]
                    .iter()
                    .any(|&v| v != NADA && !abierto[v as usize])
            {
                zf[k] = self.zeta_m;
                heap.push(Item {
                    d: self.zeta_m,
                    k: k as u32,
                });
            }
        }
        let mut orden: Vec<u32> = Vec::new();
        while let Some(Item { d, k }) = heap.pop() {
            let ku = k as usize;
            if !abierto[ku] {
                orden.push(k);
            }
            for &v in &self.malla.vec[ku] {
                if v == NADA {
                    continue;
                }
                let vu = v as usize;
                if abierto[vu] || zf[vu] < f64::INFINITY {
                    continue;
                }
                zf[vu] = z[vu].max(d);
                padre[vu] = k;
                heap.push(Item { d: zf[vu], k: v });
            }
        }
        (zf, padre, orden)
    }

    /// Destino del sedimento de la erosión (adenda de balance §6.7), sin perder nada:
    /// - baja por la pendiente hasta el mar: una inundación por prioridad desde la costa (Barnes et
    ///   al. 2014) da a cada celda de tierra su cota de desagüe y la celda hacia la que drena;
    /// - en una cuenca cerrada (cota de desagüe por encima del suelo) se queda hasta llenarla, y lo
    ///   que sobra sigue hacia el desagüe;
    /// - en el mar llena las celdas más cercanas a la desembocadura (por el mar) hasta la plataforma
    ///   (−130 m) y sigue mar adentro. Es la capa mínima de la adenda; su reparto fino (talud,
    ///   llanura abisal, compactación) es de la nota del fondo oceánico.
    fn sedimento_al_mar(&mut self, ero: &[(u32, f64)], z: Vec<f64>) {
        if ero.is_empty() {
            return;
        }
        let cfg = self.cfg.clone();
        let num = self.malla.num;
        let zeta = self.zeta_m;
        let z_plat = zeta - 130.0;
        let a_roca = cfg.rho_s / cfg.rho_c;
        let mut vol = vec![0.0; num];
        for &(k, v) in ero {
            vol[k as usize] += v;
        }
        // 1) Inundación por prioridad desde las celdas de mar con tierra al lado.
        let abierto = self.mar_abierto(&z, zeta);
        let (zf, padre, orden) = self.inundacion(&z, &abierto);
        let mut heap = BinaryHeap::new();
        // 2) De la cota de desagüe más alta a la más baja: cada celda recibe lo suyo y lo de
        //    arriba, llena su depresión y pasa el resto a su desagüe.
        let mut deposito: Vec<(u32, f64)> = Vec::new();
        let mut a_mar: Vec<(u32, f64)> = Vec::new();
        let mut cuencas = 0.0;
        for &k in orden.iter().rev() {
            let ku = k as usize;
            let mut resto = vol[ku];
            vol[ku] = 0.0;
            if resto <= 0.0 {
                continue;
            }
            if zf[ku] > z[ku] {
                let cap = self.sed_hasta(ku, zf[ku]) * a_roca * self.malla.area[ku];
                let d = resto.min(cap);
                if d > 0.0 {
                    deposito.push((k, d));
                    cuencas += d;
                    resto -= d;
                }
            }
            if resto > 0.0 {
                let pa = padre[ku];
                if pa == NADA {
                    deposito.push((k, resto));
                    cuencas += resto;
                } else if abierto[pa as usize] {
                    a_mar.push((pa, resto));
                } else {
                    vol[pa as usize] += resto;
                }
            }
        }
        // Tierra sin salida al mar (sin ninguna celda de mar en el planeta) y erosión en el mar.
        for (k, &v) in vol.iter().enumerate() {
            if v > 0.0 {
                if abierto[k] {
                    a_mar.push((k as u32, v));
                } else {
                    deposito.push((k as u32, v));
                    cuencas += v;
                }
            }
        }
        // 3) En el mar, desde cada desembocadura (en orden de celda): las celdas más cercanas por
        //    el mar, hasta la plataforma.
        a_mar.sort_unstable_by_key(|x| x.0);
        let mut libre = vec![f64::NAN; num];
        let mut dist = vec![f64::INFINITY; num];
        let mut tocadas: Vec<u32> = Vec::new();
        let (mut al_mar, mut desbordado) = (0.0, 0.0);
        let mut i = 0;
        while i < a_mar.len() {
            let m = a_mar[i].0;
            let mut resto = 0.0;
            while i < a_mar.len() && a_mar[i].0 == m {
                resto += a_mar[i].1;
                i += 1;
            }
            al_mar += resto;
            heap.clear();
            dist[m as usize] = 0.0;
            tocadas.push(m);
            heap.push(Item { d: 0.0, k: m });
            while let Some(Item { d, k }) = heap.pop() {
                let ku = k as usize;
                if d > dist[ku] {
                    continue;
                }
                if libre[ku].is_nan() {
                    libre[ku] = self.sed_hasta(ku, z_plat) * a_roca * self.malla.area[ku];
                }
                let dd = resto.min(libre[ku]);
                if dd > 0.0 {
                    deposito.push((k, dd));
                    libre[ku] -= dd;
                    resto -= dd;
                }
                if resto <= 0.0 {
                    break;
                }
                for (j, &v) in self.malla.vec[ku].iter().enumerate() {
                    if v == NADA || !abierto[v as usize] {
                        continue;
                    }
                    let nd = d + self.malla.dvec[ku][j];
                    if nd < dist[v as usize] {
                        if dist[v as usize].is_infinite() {
                            tocadas.push(v);
                        }
                        dist[v as usize] = nd;
                        heap.push(Item { d: nd, k: v });
                    }
                }
            }
            for &k in &tocadas {
                dist[k as usize] = f64::INFINITY;
            }
            tocadas.clear();
            if resto > 0.0 {
                // El mar de la desembocadura ya está lleno hasta la plataforma: un mar cerrado
                // pequeño, como el último resto de un océano entre dos continentes que chocan,
                // rodeado de montañas que se erosionan. Lo que sobra sigue hasta el mar más cercano
                // con sitio, aunque tenga que cruzar tierra: el río pasa por encima del mar relleno.
                // (Antes se quedaba todo en la desembocadura, sin tope: muestras de cientos de km de
                // sedimento, que luego pasaban a corteza.)
                desbordado += resto;
                heap.clear();
                dist[m as usize] = 0.0;
                tocadas.push(m);
                heap.push(Item { d: 0.0, k: m });
                while let Some(Item { d, k }) = heap.pop() {
                    let ku = k as usize;
                    if d > dist[ku] {
                        continue;
                    }
                    if abierto[ku] {
                        if libre[ku].is_nan() {
                            libre[ku] = self.sed_hasta(ku, z_plat) * a_roca * self.malla.area[ku];
                        }
                        let dd = resto.min(libre[ku]);
                        if dd > 0.0 {
                            deposito.push((k, dd));
                            libre[ku] -= dd;
                            resto -= dd;
                        }
                        if resto <= 0.0 {
                            break;
                        }
                    }
                    for (j, &v) in self.malla.vec[ku].iter().enumerate() {
                        if v == NADA {
                            continue;
                        }
                        let nd = d + self.malla.dvec[ku][j];
                        if nd < dist[v as usize] {
                            if dist[v as usize].is_infinite() {
                                tocadas.push(v);
                            }
                            dist[v as usize] = nd;
                            heap.push(Item { d: nd, k: v });
                        }
                    }
                }
                for &k in &tocadas {
                    dist[k as usize] = f64::INFINITY;
                }
                tocadas.clear();
                if resto > 0.0 {
                    // Ningún mar del planeta tiene sitio (no puede pasar con océanos).
                    deposito.push((m, resto));
                }
            }
        }
        self.cuentas.sedimento_desbordado_km3 += desbordado;
        self.cuentas.sedimento_al_mar_km3 += al_mar;
        self.cuentas.sedimento_en_cuencas_km3 += cuencas;
        // 4) Depósito exacto (con su masa: la roca `v` ocupa `v·ρ_c/ρ_s` de sedimento).
        deposito.sort_unstable_by_key(|x| x.0);
        let inv = self.indice_inverso();
        for (k, v) in deposito {
            let vol_sed = v / a_roca;
            let puesto = self.en_celda(&inv, k as usize, vol_sed, false, |c, _, ds| c.sed += ds);
            // Sin dueño la celda no puede recibirlo (no pasa tras rellenar los huecos).
            self.cuentas.sedimento_sin_destino_km3 += (vol_sed - puesto) * a_roca;
        }
    }

    // --- f. Eventos de placas (§6.4–§6.5) ------------------------------------------------------

    fn evento(&mut self, tipo: &str, placas: Vec<u64>, largo: f64, area: f64, punto: Option<V3>) {
        self.evento_det(tipo, placas, largo, area, punto, Detalle::default());
    }

    fn evento_det(
        &mut self,
        tipo: &str,
        placas: Vec<u64>,
        largo: f64,
        area: f64,
        punto: Option<V3>,
        det: Detalle,
    ) {
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
            clase: det.clase,
            eta: det.eta,
            solapado_km3: det.solapado_km3,
            perdido_km3: det.perdido_km3,
            edad_fondo_myr: det.edad_fondo_myr,
            edad_fosa_myr: det.edad_fosa_myr,
            losa_km: det.losa_km,
            tiempo_choque_myr: det.tiempo_choque_myr,
            trozos_km2: det.trozos_km2,
            sutura_km: det.sutura_km,
            extension_km: det.extension_km,
            fosa_km: det.fosa_km,
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

    /// Pasa la muestra `c` de la celda local `lb` de una placa con rotación `rot_b` a la placa `a`
    /// (no se copia): a la celda local libre de `a` más cercana a su posición, la propia o sus
    /// vecinas a uno y dos anillos (orden fijo), con el espesor ajustado al área de la celda; si no
    /// hay ninguna libre, su material se suma a la muestra más delgada de las que ocupan su posición
    /// y sus dos anillos. Con `estado`, el de su tramo convergente pasa con ella. La corteza se
    /// conserva exactamente (adenda §0).
    fn mover_muestra(
        &mut self,
        a: usize,
        rot_b: &Mat3,
        lb: u32,
        c: Corteza,
        estado: Option<EstadoFosa>,
    ) {
        let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
        let x = aplicar(rot_b, self.malla.dir[lb as usize]);
        let ar_b = self.malla.area[lb as usize];
        let pa = &self.placas[a];
        let l0 = self.malla.localizar(aplicar_t(&pa.rot, x)) as u32;
        // Celda libre más cercana: la propia, o sus vecinas a uno y dos anillos (orden fijo).
        let mut libre = (pa.mapa[l0 as usize] == NADA).then_some(l0);
        if libre.is_none() {
            'buscar: for &v1 in &self.malla.vec[l0 as usize] {
                if v1 == NADA {
                    continue;
                }
                if pa.mapa[v1 as usize] == NADA {
                    libre = Some(v1);
                    break 'buscar;
                }
            }
        }
        if libre.is_none() {
            'buscar2: for &v1 in &self.malla.vec[l0 as usize] {
                if v1 == NADA {
                    continue;
                }
                for &v2 in &self.malla.vec[v1 as usize] {
                    if v2 != NADA && pa.mapa[v2 as usize] == NADA {
                        libre = Some(v2);
                        break 'buscar2;
                    }
                }
            }
        }
        let mundo = self.malla.localizar(x) as u32;
        match libre {
            Some(l) => {
                // Mismo volumen en una celda de otra área: se ajusta el espesor (la base basáltica
                // no es material continental y no cambia).
                let f = ar_b / self.malla.area[l as usize];
                let c2 = Corteza {
                    h: if c.cont {
                        c.h * f
                    } else {
                        c.h - c.hc + c.hc * f
                    },
                    hc: c.hc * f,
                    sed: c.sed * f,
                    proc: c.proc.escalada(f),
                    ..c
                };
                self.placas[a].insertar(l, c2, mundo);
                if let Some(e) = estado {
                    self.placas[a].fosas.insert(l, e);
                }
            }
            None => {
                // Sin sitio cerca: su material continental se suma a la muestra más delgada de las
                // que ocupan su posición y sus dos anillos (desempate por celda); la oceánica que lo
                // recibe pasa a continental si supera H_cont_min.
                let pa = &self.placas[a];
                let mut cand: Vec<u32> = vec![l0];
                for &v1 in &self.malla.vec[l0 as usize] {
                    if v1 == NADA {
                        continue;
                    }
                    cand.push(v1);
                    cand.extend(self.malla.vec[v1 as usize].iter().filter(|&&v| v != NADA));
                }
                cand.sort_unstable();
                cand.dedup();
                let destino = cand
                    .iter()
                    .copied()
                    .filter_map(|l| pa.get(l).map(|d| (d.h_eq(rs, rc), l)))
                    .min_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)))
                    .map_or(l0, |x| x.1);
                let ar_a = self.malla.area[destino as usize];
                let h_cont_min = self.cfg.h_cont_min_km;
                if let Some(d) = self.placas[a].get_mut(destino) {
                    let r = ar_b / ar_a;
                    d.sumar_hc(c.hc * r, c.arco, c.edad_tt, rs, rc, h_cont_min, via::FUSION);
                    d.sed += c.sed * r;
                }
            }
        }
    }

    /// Funde la placa `b` en la `a`: cada muestra de `b` pasa a `a` ([`Deriva::mover_muestra`]), con
    /// el estado de su tramo convergente salvo el del borde entre las dos, que ya no existe. Así la
    /// corteza se conserva exactamente (adenda §0).
    fn fundir(&mut self, a: usize, b: usize) {
        let muestras: Vec<(u32, Corteza)> = {
            let pb = &self.placas[b];
            pb.celda
                .iter()
                .copied()
                .zip(pb.cort.iter().copied())
                .collect()
        };
        let rot_b = self.placas[b].rot;
        for (lb, c) in muestras {
            let estado = self.placas[b]
                .fosas
                .get(&lb)
                .copied()
                .filter(|e| !self.es_placa(e.otra, a as u32));
            self.mover_muestra(a, &rot_b, lb, c, estado);
        }
        for k in 0..self.malla.num {
            if self.dueno[k] == b as u32 {
                self.dueno[k] = a as u32;
                self.rep[k] = self
                    .malla
                    .localizar(aplicar_t(&self.placas[a].rot, self.malla.dir[k]))
                    as u32;
            }
        }
        // La placa fundida es ya parte de `a`: el estado de los tramos de otras placas que la
        // miraba mira ahora a `a`.
        let mut anc = std::mem::take(&mut self.placas[b].ancestros);
        anc.push(self.placas[b].id);
        self.placas[a].ancestros.extend(anc);
        self.placas[a].recortar_ancestros();
        self.placas[b].morir();
        self.placas[a].actualizar_cap(&self.malla);
    }

    /// Pasa a la placa `r` las muestras de la placa `p` que caen en las celdas del mundo `franja`
    /// (rama D de la nota de demografía de placas v2), con la conservación de `fundir`. No pasan con
    /// ellas ni el estado de sus tramos convergentes (la fosa de enfrente se apaga) ni el de su
    /// invasión, y `r` no hereda a `p` como antepasada: los tramos de otras placas que miraban a `p`
    /// no miran a `r`. Devuelve el área que pasa (km²).
    fn trasladar_franja(&mut self, r: usize, p: usize, franja: &[u32]) -> f64 {
        let en: std::collections::HashSet<u32> = franja.iter().copied().collect();
        let mut sel: Vec<u32> = self.placas[p]
            .celda
            .iter()
            .zip(&self.placas[p].mundo)
            .filter(|(_, m)| en.contains(m))
            .map(|(&l, _)| l)
            .collect();
        sel.sort_unstable();
        let rot_p = self.placas[p].rot;
        let mut area = 0.0;
        for lb in sel {
            let Some(c) = self.placas[p].quitar(lb) else {
                continue;
            };
            self.placas[p].fosas.remove(&lb);
            self.placas[p].invadidas.remove(&lb);
            area += self.malla.area[lb as usize];
            self.mover_muestra(r, &rot_p, lb, c, None);
        }
        for &k in franja {
            let ku = k as usize;
            if self.dueno[ku] == p as u32 {
                self.dueno[ku] = r as u32;
                self.rep[ku] = self
                    .malla
                    .localizar(aplicar_t(&self.placas[r].rot, self.malla.dir[ku]))
                    as u32;
            }
        }
        self.placas[p].actualizar_cap(&self.malla);
        self.placas[r].actualizar_cap(&self.malla);
        area
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
            // Un choque que añade un continente reinicia el reloj del bloque que resulta (nota de
            // rifts v2 §6.1): los bloques de las dos placas que tocan el frente, si hay de los dos.
            self.reiniciar_reloj_de_sutura(a, b);
            self.estampar(b, lote::SUTURA);
            self.marcar_sutura(a, b);
            self.fundir(a, b);
            self.evento("sutura", ids, l, 0.0, Some(punto));
            cambio = true;
        }
        cambio
    }

    /// Reloj de los rifts en una sutura entre las placas `a` y `b` (nota de rifts v2 §6.1): si las dos
    /// tienen un bloque continental que toca su frente de choque, todos esos bloques empiezan de
    /// cero, porque juntos forman un bloque nuevo. Se llama antes de fundirlas.
    fn reiniciar_reloj_de_sutura(&mut self, a: usize, b: usize) {
        let frente: std::collections::HashSet<u32> = self
            .bordes
            .iter()
            .filter(|bo| {
                bo.tipo == TipoBorde::Colision
                    && ((bo.p as usize == a && bo.q as usize == b)
                        || (bo.p as usize == b && bo.q as usize == a))
            })
            .map(|bo| bo.k)
            .collect();
        if frente.is_empty() {
            return;
        }
        let bloques = self.bloques_continentales();
        let tocan = |pl: usize| -> Vec<usize> {
            bloques
                .iter()
                .enumerate()
                .filter(|(_, bl)| {
                    bl.placa_indice() == pl
                        && bl.celdas_del_mundo().iter().any(|k| frente.contains(k))
                })
                .map(|(i, _)| i)
                .collect()
        };
        let (ba, bb) = (tocan(a), tocan(b));
        if ba.is_empty() || bb.is_empty() {
            return;
        }
        let celdas: Vec<u32> = ba
            .iter()
            .chain(&bb)
            .flat_map(|&i| bloques[i].celdas_del_mundo().iter().copied())
            .collect();
        self.reiniciar_reloj(&celdas);
    }

    /// Lado de cada muestra de la placa `pi` respecto al círculo máximo de polo `polo` deformado
    /// por ruido (`true`, el lado `+polo`).
    fn lados_por_circulo(&self, pi: usize, polo: V3) -> Vec<bool> {
        let amp = self.cfg.deformacion;
        let madre = &self.placas[pi];
        madre
            .celda
            .iter()
            .map(|&l| {
                let x = aplicar(&madre.rot, self.malla.dir[l as usize]);
                let f = 2.5;
                dot(x, polo)
                    + amp * self.ruido_def[0].fbm(x[0] * f, x[1] * f, x[2] * f, 3, 0.5, 2.0)
                    >= 0.0
            })
            .collect()
    }

    /// Parte la placa `pi` por un círculo máximo de polo `polo` deformado por ruido. Devuelve los
    /// índices de las dos hijas (la del lado `+polo` primero), o `None` si una quedaría vacía o
    /// demasiado pequeña.
    fn partir(&mut self, pi: usize, polo: V3, centro: V3) -> Option<(usize, usize)> {
        let lados = self.lados_por_circulo(pi, polo);
        self.partir_por_lados(pi, &lados, polo, centro)
    }

    /// Parte la placa `pi` en dos por un corte cualquiera (nota de rifts §6.3): `lados[s]` dice a
    /// qué hija va cada muestra (`true`, la primera). La apertura es un giro alrededor del eje a 90°
    /// de `centro` en el plano de corte de polo `polo` (la primera hija se aleja hacia `+polo`).
    /// Devuelve los índices de las dos hijas, o `None` si una quedaría por debajo de `A_min`.
    fn partir_por_lados(
        &mut self,
        pi: usize,
        lados: &[bool],
        polo: V3,
        centro: V3,
    ) -> Option<(usize, usize)> {
        let cfg = self.cfg.clone();
        let madre = &self.placas[pi];
        let mut a_mas = 0.0;
        let mut a_menos = 0.0;
        for (s, &lado) in lados.iter().enumerate() {
            let a = self.malla.area[madre.celda[s] as usize];
            if lado {
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
        for (s, &lado_s) in lados.iter().enumerate() {
            let l = madre.celda[s];
            let c = madre.cort[s];
            let h = if lado_s { &mut h1 } else { &mut h2 };
            h.insertar(l, c, madre.mundo[s]);
            if let Some(e) = madre.fosas.get(&l) {
                h.fosas.insert(l, *e);
            }
            if let Some(e) = madre.invadidas.get(&l) {
                h.invadidas.insert(l, *e);
            }
        }
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

    /// Diagnóstico de procedencia: marca todas las muestras de la placa `pi` con un lote nuevo de
    /// tipo `tipo` ([`lote`]). No cambia nada del modelo.
    fn estampar(&mut self, pi: usize, tipo: u8) {
        let id = self.lotes.len() as u32;
        self.lotes.push(tipo);
        for c in &mut self.placas[pi].cort {
            c.proc.lote = id;
        }
    }

    /// Celdas de borde con una fosa activa entre las dos placas del borde.
    fn bordes_con_fosa(&self) -> Vec<bool> {
        self.bordes
            .iter()
            .map(|b| {
                b.tipo == TipoBorde::Subduccion
                    && self
                        .estado_fosa(b.p, self.rep[b.k as usize], b.q)
                        .is_some_and(|e| e.activa)
            })
            .collect()
    }

    /// Distancias (km) por dentro de la placa `p` a las celdas del mundo `fuentes` (a media celda),
    /// hasta `max_d`: solo las celdas alcanzadas.
    fn distancias_en_placa(&mut self, p: u32, fuentes: &[u32], max_d: f64) -> HashMap<u32, f64> {
        self.distancias_y_fuente_en_placa(p, fuentes, max_d)
            .into_iter()
            .map(|(k, (d, _))| (k, d))
            .collect()
    }

    /// Como [`Deriva::distancias_en_placa`], con la fuente más cercana de cada celda (su posición en
    /// `fuentes`).
    fn distancias_y_fuente_en_placa(
        &mut self,
        p: u32,
        fuentes: &[u32],
        max_d: f64,
    ) -> HashMap<u32, (f64, u32)> {
        let media = 0.5 * self.malla.lado_km;
        let f: Vec<(u32, f64, u32)> = fuentes
            .iter()
            .enumerate()
            .map(|(i, &k)| (k, media, i as u32))
            .collect();
        let dueno = &self.dueno;
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(
            &self.malla,
            &f,
            |u, v| dueno[u] == p && dueno[v] == p,
            max_d,
        );
        let d: HashMap<u32, (f64, u32)> = campo
            .orden
            .iter()
            .map(|&k| (k, (campo.dist[k as usize], campo.fuente[k as usize])))
            .collect();
        self.campo = campo;
        d
    }

    /// Rama D de la nota de demografía de placas v2: subducción de dorsales. Un tramo de dorsal
    /// activa entre las placas `p` y `r` a menos de `d_dc` de una fosa activa donde `p` se hunde
    /// bajo una tercera placa (medido por dentro de `p`: la franja de `p` entre la dorsal y la fosa
    /// es más estrecha que `d_dc`) muere: la franja de `p` entre ese tramo y la fosa pasa a `r` con la conservación de
    /// `fundir` ([`Deriva::trasladar_franja`]). Así la dorsal ya no separa dos placas, la fosa de
    /// enfrente se apaga (`r` no hereda su estado) y el borde nuevo entre `r` y la placa de arriba
    /// se clasifica en el paso siguiente según su movimiento. Si `p` se queda sin muestras, se
    /// retira, como las placas fantasma. La franja son las celdas de `p` a menos de `d_dc` de la fosa
    /// cuya suma de distancias a la fosa y al tramo de dorsal no pasa de `d_dc` más una celda: las
    /// que quedan entre los dos.
    ///
    /// Apunta en `dorsal_fosa_paso` todos los encuentros del paso (D6). Orden fijo: pares (p, r) por
    /// índice; una franja por placa y por paso, sin tocar una placa que ya cambió en el paso (lo
    /// demás, en el paso siguiente).
    fn subduccion_de_dorsales(&mut self) -> bool {
        self.dorsal_fosa_paso.clear();
        let d_dc = self.cfg.d_dc_km;
        if d_dc <= 0.0 {
            return false;
        }
        let lado = self.malla.lado_km;
        let con_fosa = self.bordes_con_fosa();
        // Bordes de fosa activa donde se hunde cada placa.
        let mut fosa_de: std::collections::BTreeMap<u32, Vec<usize>> = Default::default();
        for (i, (b, &f)) in self.bordes.iter().zip(&con_fosa).enumerate() {
            if f && b.p_subduce {
                fosa_de.entry(b.p).or_default().push(i);
            }
        }
        if fosa_de.is_empty() {
            return false;
        }
        // Bordes de dorsal de cada placa con fosa, por la placa del otro lado.
        let mut dorsal_de: std::collections::BTreeMap<(u32, u32), Vec<usize>> = Default::default();
        for (i, b) in self.bordes.iter().enumerate() {
            if b.tipo == TipoBorde::Divergente && b.p != b.q && fosa_de.contains_key(&b.p) {
                dorsal_de.entry((b.p, b.q)).or_default().push(i);
            }
        }
        type Accion = (usize, usize, Vec<u32>, f64, f64, Vec<u64>, V3);
        let mut acciones: Vec<Accion> = Vec::new();
        for (&(p, r), dors) in &dorsal_de {
            // La fosa tiene que ser con una tercera placa: una dorsal y una fosa entre las mismas dos
            // placas son un solo borde que cambia de tipo cerca de su polo de rotación, no una dorsal
            // que llega a una fosa.
            let fosa_pr: Vec<usize> = fosa_de[&p]
                .iter()
                .copied()
                .filter(|&i| self.bordes[i].q != r)
                .collect();
            if fosa_pr.is_empty() {
                continue;
            }
            let fuentes: Vec<u32> = fosa_pr.iter().map(|&i| self.bordes[i].k).collect();
            let dfu = self.distancias_y_fuente_en_placa(p, &fuentes, d_dc + lado);
            let df: HashMap<u32, f64> = dfu.iter().map(|(&k, &(d, _))| (k, d)).collect();
            // La fosa tiene que estar delante de la dorsal: el tramo de fosa más cercano, a menos de
            // `θ_dc` de la dirección de apertura hacia dentro de `p`. Así cuenta la franja entre las
            // dos (Farallón) y no el costado de un punto triple, donde la dorsal llega de lado.
            let cos_dc = self.cfg.theta_dc_grados.to_radians().cos();
            let delante = |i: usize| -> bool {
                let b = &self.bordes[i];
                let Some(&(d, f)) = dfu.get(&b.k) else {
                    return false;
                };
                if d >= d_dc {
                    return false;
                }
                let x = self.malla.dir[b.k as usize];
                let y = self.malla.dir[fuentes[f as usize] as usize];
                let v = sub(y, x);
                let v = sub(v, esc(x, dot(v, x)));
                let nv = dot(v, v).sqrt();
                if nv < 1e-12 {
                    return false;
                }
                dot(v, esc(b.normal, -1.0)) / nv >= cos_dc
            };
            let cerca: Vec<usize> = dors.iter().copied().filter(|&i| delante(i)).collect();
            if cerca.is_empty() {
                continue;
            }
            let dorsal_km: f64 = cerca.iter().map(|&i| self.bordes[i].largo_km).sum();
            let dist_min = cerca
                .iter()
                .map(|&i| df[&self.bordes[i].k])
                .fold(f64::INFINITY, f64::min);
            self.dorsal_fosa_paso.push(EncuentroDorsalFosa {
                p: self.placas[p as usize].id,
                r: self.placas[r as usize].id,
                dorsal_km,
                dist_min_km: dist_min,
            });
            // La franja: entre el tramo de dorsal cercano y la fosa.
            let fuentes_d: Vec<u32> = cerca.iter().map(|&i| self.bordes[i].k).collect();
            let dd = self.distancias_en_placa(p, &fuentes_d, d_dc + lado);
            let mut franja: Vec<u32> = df
                .iter()
                .filter(|&(k, &x)| x < d_dc && dd.get(k).is_some_and(|&y| x + y <= d_dc + lado))
                .map(|(&k, _)| k)
                .collect();
            if franja.is_empty() {
                continue;
            }
            franja.sort_unstable();
            // La fosa que se apaga: la de `p` en la franja, y las placas de arriba.
            let mut fosa_km = 0.0;
            let mut arriba: Vec<u64> = Vec::new();
            for &i in &fosa_pr {
                let b = &self.bordes[i];
                if franja.binary_search(&b.k).is_ok() {
                    fosa_km += b.largo_km;
                    arriba.push(self.placas[b.q as usize].id);
                }
            }
            arriba.sort_unstable();
            arriba.dedup();
            let punto = self.malla.dir[self.bordes[cerca[0]].k as usize];
            acciones.push((
                p as usize, r as usize, franja, dorsal_km, fosa_km, arriba, punto,
            ));
        }
        let mut cambio = false;
        let mut tocadas: Vec<usize> = Vec::new();
        for (p, r, franja, dorsal_km, fosa_km, arriba, punto) in acciones {
            if tocadas.contains(&p)
                || tocadas.contains(&r)
                || !self.placas[p].viva
                || !self.placas[r].viva
            {
                continue;
            }
            let (id_p, id_r) = (self.placas[p].id, self.placas[r].id);
            // Procedencia: la franja forma un lote nuevo.
            let lote_id = self.lotes.len() as u32;
            self.lotes.push(lote::DORSAL_FOSA);
            {
                let en: std::collections::HashSet<u32> = franja.iter().copied().collect();
                let pl = &mut self.placas[p];
                for (c, m) in pl.cort.iter_mut().zip(&pl.mundo) {
                    if en.contains(m) {
                        c.proc.lote = lote_id;
                    }
                }
            }
            let area = self.trasladar_franja(r, p, &franja);
            let retirada = self.placas[p].cort.is_empty();
            if retirada {
                self.placas[p].morir();
            }
            let mut ids = vec![id_p, id_r];
            ids.extend(arriba);
            self.evento_det(
                "subduccion_dorsal",
                ids,
                dorsal_km,
                area,
                Some(punto),
                Detalle {
                    clase: Some(if retirada { "retirada" } else { "captura" }.to_string()),
                    fosa_km: Some(fosa_km),
                    ..Detalle::default()
                },
            );
            tocadas.push(p);
            tocadas.push(r);
            cambio = true;
        }
        cambio
    }

    /// A2. Captura de microplacas (nota de demografía de placas §6): una placa de menos de `A_micro`
    /// que lleva `τ_micro` Myr sin fosa activa propia (ni como placa que baja ni como placa de
    /// arriba) pasa a la vecina con la que comparte más borde (empates, por el índice de placa), con
    /// la misma conservación que A1. Sustituye a la absorción de toda placa de menos de `A_min`, de
    /// la que solo se libraban las que tenían una losa que tiraba. Una placa sin área ni borde (la
    /// consumida entera) pasa a la placa bajo sus muestras ocultas o, si no le queda ninguna, se
    /// retira.
    fn capturas(&mut self) -> bool {
        let areas = self.area_placa();
        let a_micro = self.cfg.a_micro_mkm2 * 1e6;
        let dt = self.cfg.dt_myr;
        let con_fosa = self.bordes_con_fosa();
        let mut fosa_propia = vec![false; self.placas.len()];
        for (b, &fosa) in self.bordes.iter().zip(&con_fosa) {
            if fosa {
                fosa_propia[b.p as usize] = true;
                fosa_propia[b.q as usize] = true;
            }
        }
        let mut capturar: Vec<usize> = Vec::new();
        for (pi, p) in self.placas.iter_mut().enumerate() {
            if !p.viva {
                continue;
            }
            if areas[pi] < a_micro && !fosa_propia[pi] {
                p.sin_fosa_myr += dt;
            } else {
                p.sin_fosa_myr = 0.0;
            }
            if areas[pi] < a_micro && p.sin_fosa_myr >= self.cfg.tau_micro_myr {
                capturar.push(pi);
            }
        }
        let mut cambio = false;
        for pi in capturar {
            if !self.placas[pi].viva {
                continue;
            }
            // La vecina con más borde común; sin borde, la placa bajo la mayoría de sus muestras.
            let mut borde: Vec<(u32, f64)> = Vec::new();
            for b in &self.bordes {
                if b.p as usize == pi && self.placas[b.q as usize].viva {
                    match borde.iter_mut().find(|x| x.0 == b.q) {
                        Some(x) => x.1 += b.largo_km,
                        None => borde.push((b.q, b.largo_km)),
                    }
                }
            }
            let mut q = borde
                .iter()
                .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
                .map(|x| x.0);
            if q.is_none() {
                let mut bajo: Vec<(u32, u32)> = Vec::new();
                for &k in &self.placas[pi].mundo {
                    let d = self.dueno[k as usize];
                    if d == NADA || d as usize == pi || !self.placas[d as usize].viva {
                        continue;
                    }
                    match bajo.iter_mut().find(|x| x.0 == d) {
                        Some(x) => x.1 += 1,
                        None => bajo.push((d, 1)),
                    }
                }
                q = bajo
                    .iter()
                    .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
                    .map(|x| x.0);
            }
            let area = areas[pi];
            match q {
                Some(q) => {
                    let ids = vec![self.placas[pi].id, self.placas[q as usize].id];
                    self.estampar(pi, lote::CAPTURA);
                    self.fundir(q as usize, pi);
                    self.evento("captura_microplaca", ids, 0.0, area, None);
                }
                None if self.placas[pi].cort.is_empty() => {
                    let ids = vec![self.placas[pi].id];
                    self.placas[pi].morir();
                    self.evento("captura_microplaca", ids, 0.0, 0.0, None);
                }
                None => continue,
            }
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
        match self.partir(pi, polo, c) {
            Some((h1, h2)) => {
                self.estampar(h1, lote::SALTO_DORSAL);
                self.estampar(h2, lote::SALTO_DORSAL);
                let ids = vec![id, self.placas[h1].id, self.placas[h2].id];
                self.evento("salto_dorsal", ids, 0.0, a_oc[pi], Some(c));
                true
            }
            None => false,
        }
    }

    // --- g. Métricas y eventos de inicio (§7–§8) -----------------------------------------------

    fn metricas_del_paso(&mut self, v0: f64, c0: &CuentasCorteza) {
        let num = self.malla.num;
        // Balance del paso (adenda §6.9): cambio del inventario frente a fuentes menos sumideros.
        let v1 = self.inventario_continental();
        let error = if v0 > 0.0 {
            ((v1 - v0) - (self.cuentas.explicado() - c0.explicado())).abs() / v0
        } else {
            0.0
        };
        self.cuentas.error_balance_max_rel = self.cuentas.error_balance_max_rel.max(error);
        let au = 1.0 / (self.cfg.dt_myr * 1e6);
        let c = self.cuentas;
        let (rs, rc) = (self.cfg.rho_s, self.cfg.rho_c);
        let rodaje = self.t_myr - self.t0_myr <= self.cfg.t_rodaje_myr;
        let omegas: Vec<V3> = self.placas.iter().map(|p| p.omega).collect();
        let v_rms = self.v_rms_de(&omegas) / CM_ANIO;
        self.ultima_v_rms = v_rms;
        let (mut dors, mut fosas, mut col, mut trans, mut perimetro) = (0.0, 0.0, 0.0, 0.0, 0.0);
        let (mut difusas, mut edad_fosas, mut losa_fosas) = (0.0, 0.0, 0.0);
        let (mut conv_fosa, mut fosa_lenta) = (0.0, 0.0);
        let (mut conv_col, mut largo_col) = (0.0, 0.0);
        for b in &self.bordes {
            if b.tipo == TipoBorde::Colision {
                conv_col += b.v_n.max(0.0) / CM_ANIO * b.largo_km;
                largo_col += b.largo_km;
            }
            if b.tipo == TipoBorde::Subduccion && b.p_subduce {
                let v = b.v_n.max(0.0) / CM_ANIO;
                conv_fosa += v * b.largo_km;
                if v < 2.0 {
                    fosa_lenta += b.largo_km;
                }
                if !rodaje {
                    let i = CLASES_CONVERGENCIA_CM_ANIO.partition_point(|&x| x <= v) - 1;
                    self.fosas_por_convergencia_km[i] += b.largo_km;
                }
            }
            match b.tipo {
                TipoBorde::Divergente => dors += 0.5 * b.largo_km,
                TipoBorde::Subduccion if b.p_subduce => fosas += b.largo_km,
                TipoBorde::Colision => col += 0.5 * b.largo_km,
                TipoBorde::Transformante => trans += 0.5 * b.largo_km,
                TipoBorde::Difusa => difusas += 0.5 * b.largo_km,
                _ => {}
            }
            perimetro += 0.5 * b.largo_km;
            // Edad de las fosas activas y fracción del tirón de su losa, en el lado que se hunde.
            if b.tipo == TipoBorde::Subduccion
                && b.p_subduce
                && let Some(e) = self.estado_fosa(b.p, self.rep[b.k as usize], b.q)
            {
                edad_fosas += e.edad_myr * b.largo_km;
                losa_fosas += b.losa * b.largo_km;
            }
        }
        let (mut a_oc, mut edad_a, mut edad_max, mut a_cont) = (0.0, 0.0, 0.0f64, 0.0);
        let (mut h_max, mut h_cont, mut sed_tot, mut sed_oc) = (0.0f64, 0.0, 0.0, 0.0);
        let mut a_70 = 0.0;
        // Nota de isostasia con litosfera §8: área continental y altura de la tierra por clase de
        // edad tectonotérmica, y profundidad de los márgenes estirados distales (corteza de menos de
        // 20 km) por tiempo desde la rotura.
        let zeta = self.zeta_m;
        let (mut a_edad, mut a_tierra_edad, mut z_tierra_edad) = ([0.0; 5], [0.0; 5], [0.0; 5]);
        let (mut prof_margen, mut tect_margen, mut a_margen) = ([0.0; 6], [0.0; 6], [0.0; 6]);
        for k in 0..num {
            if let Some(cz) = self.corteza(k) {
                let a = self.malla.area[k];
                h_max = h_max.max(cz.h);
                sed_tot += cz.sed * rs / rc * a;
                if cz.cont {
                    a_cont += a;
                    h_cont += cz.h * a;
                    if cz.h > 70.0 {
                        a_70 += a;
                    }
                    let ic = CLASES_EDAD_TT_MYR.partition_point(|&x| x <= cz.edad_tt) - 1;
                    a_edad[ic] += a;
                    let z = isostasia::altura(&self.cfg, cz) - zeta;
                    if z >= 0.0 {
                        a_tierra_edad[ic] += a;
                        z_tierra_edad[ic] += z * a;
                    }
                    if cz.rift_id != 0 && cz.h < 20.0 {
                        let im = CLASES_MARGEN_MYR.partition_point(|&x| x <= cz.edad_tt) - 1;
                        prof_margen[im] -= z * a;
                        // El basamento sin su sedimento, con agua encima (lo que mide el
                        // «backstripping» y da McKenzie, 1978).
                        let sin_sed = Corteza { sed: 0.0, ..*cz };
                        tect_margen[im] -= (isostasia::altura(&self.cfg, &sin_sed) - zeta) * a;
                        a_margen[im] += a;
                    }
                } else {
                    a_oc += a;
                    edad_a += cz.edad_oc * a;
                    edad_max = edad_max.max(cz.edad_oc);
                    sed_oc += cz.sed * a;
                }
            }
        }
        let t0 = std::time::Instant::now();
        // Los márgenes por sus celdas de costa y su perfil, cada 25 Myr y en el último paso.
        let cada = (25.0 / self.cfg.dt_myr).round().max(1.0) as u64;
        let ultimo = self.paso >= (self.cfg.t_deriva_myr / self.cfg.dt_myr).round() as u64;
        let toca = self.paso.is_multiple_of(cada) || ultimo;
        if toca {
            self.ultimos_margenes = self.margenes_pasivos();
        }
        let margenes = self.ultimos_margenes;
        let t1 = std::time::Instant::now();
        let cont = self.celdas_continente();
        if toca {
            self.ultimo_perfil = self.perfil_de_margenes(&cont);
        }
        let (costa_margen_km, franja_estirada_km) = self.ultimo_perfil;
        self.margenes_rift_paso();
        let t2 = std::time::Instant::now();
        let (bloques_continentales, bloque_mayor_frac) = self.metricas_de_bloques(&cont);
        let t3 = std::time::Instant::now();
        let sc = self.indice_supercontinente();
        self.seg_partes[3] += (t1 - t0).as_secs_f64();
        self.seg_partes[4] += (t2 - t1).as_secs_f64();
        self.seg_partes[5] += (t3 - t2).as_secs_f64();
        self.seg_partes[6] += t3.elapsed().as_secs_f64();
        if sc >= 0.7 && !self.super_formado {
            self.super_formado = true;
            self.evento("supercontinente_formado", vec![], 0.0, a_cont, None);
        } else if sc < 0.7 && self.super_formado {
            self.super_formado = false;
            self.evento("supercontinente_roto", vec![], 0.0, a_cont, None);
        }
        self.eventos_de_inicio();
        self.foto_supercontinente_si_toca();
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
            transformantes_km: trans,
            edad_media_oceano_myr: if a_oc > 0.0 { edad_a / a_oc } else { 0.0 },
            edad_max_oceano_myr: edad_max,
            volumen_continental_km3: self.volumen_continental(),
            area_continental_frac: a_cont / a_tot,
            supercontinente: sc,
            nivel_mar_m: self.zeta_m,
            f_arc_au: (c.arcos_km3 - c0.arcos_km3) * au,
            f_se_au: (c.erosion_subduccion_km3 - c0.erosion_subduccion_km3) * au,
            f_sed_au: (c.sedimento_subducido_km3 - c0.sedimento_subducido_km3) * au,
            f_col_au: (c.choque_perdida_km3 + c.choque_sin_sitio_km3
                - c0.choque_perdida_km3
                - c0.choque_sin_sitio_km3)
                * au,
            f_del_au: (c.delaminacion_km3 - c0.delaminacion_km3) * au,
            f_den_au: (c.sedimento_al_mar_km3 - c0.sedimento_al_mar_km3) * au,
            f_acr_au: (c.sedimento_acrecionado_km3 - c0.sedimento_acrecionado_km3) * au,
            inventario_km3: v1,
            espesor_max_km: h_max,
            espesor_medio_cont_km: if a_cont > 0.0 { h_cont / a_cont } else { 0.0 },
            area_mas_70km_frac: if a_cont > 0.0 { a_70 / a_cont } else { 0.0 },
            area_subducida_km2_anio: self.area_subducida_km2 * au,
            area_creada_km2_anio: self.area_nueva_km2 * au,
            fosa_erosiva_frac: if self.area_subducida_km2 > 0.0 {
                self.area_erosiva_km2 / self.area_subducida_km2
            } else {
                0.0
            },
            sedimento_km3: sed_tot,
            sedimento_medio_oceano_km: if a_oc > 0.0 { sed_oc / a_oc } else { 0.0 },
            error_balance_rel: error,
            perimetro_km: perimetro,
            dientes_recolocados_km2_anio: self.dientes_recolocados_km2 * au,
            dientes_sin_hueco_km2_anio: self.dientes_sin_hueco_km2 * au,
            dientes_a_otra_placa_km2_anio: self.dientes_a_otra_placa_km2 * au,
            creacion_residual_km2_anio: self.creacion_residual_km2 * au,
            area_reasignada_km2_anio: self.area_reasignada_km2 * au,
            area_islas_km2_anio: self.area_islas_km2 * au,
            dientes_continentales_au: self.dientes_continentales_km3 * au,
            convergencia_media_fosas_cm_anio: if fosas > 0.0 { conv_fosa / fosas } else { 0.0 },
            fosa_lenta_frac: if fosas > 0.0 { fosa_lenta / fosas } else { 0.0 },
            difusas_km: difusas,
            area_difusa_km2_anio: self.fosas_paso.area_difusa_km2 * au,
            edad_media_fosas_myr: if fosas > 0.0 { edad_fosas / fosas } else { 0.0 },
            losa_media_frac: if fosas > 0.0 { losa_fosas / fosas } else { 0.0 },
            nacimientos: self.fosas_paso.nacen,
            muertes: self.fosas_paso.mueren,
            invadido_km: self.fosas_paso.invadido_km,
            separar: self.fosas_paso.separar,
            roturas: self.fosas_paso.roturas,
            convergencia_media_choques_cm_anio: if largo_col > 0.0 {
                conv_col / largo_col
            } else {
                0.0
            },
            f_col_con_losa_au: (c.choque_perdida_con_losa_km3 - c0.choque_perdida_con_losa_km3)
                * au,
            f_col_sin_losa_au: (c.choque_perdida_sin_losa_km3 - c0.choque_perdida_sin_losa_km3)
                * au,
            margen_pasivo_km: margenes.0,
            edad_media_margen_myr: margenes.1,
            edad_max_margen_myr: margenes.2,
            bloques_continentales,
            bloque_mayor_frac,
            rifts_estirando: self.rifts_activos.len() as u32,
            dorsal_fosa_n: self.dorsal_fosa_paso.len() as u32,
            dorsal_fosa_km: self.dorsal_fosa_paso.iter().map(|e| e.dorsal_km).sum(),
            edad_tt_area_frac: a_edad.map(|x| if a_cont > 0.0 { x / a_cont } else { 0.0 }),
            altura_tierra_por_edad_m: std::array::from_fn(|i| {
                if a_tierra_edad[i] > 0.0 {
                    z_tierra_edad[i] / a_tierra_edad[i]
                } else {
                    0.0
                }
            }),
            tierra_por_edad_km2: a_tierra_edad,
            reinicio_km2: self.reinicio_km2_paso,
            margen_prof_m: std::array::from_fn(|i| {
                if a_margen[i] > 0.0 {
                    prof_margen[i] / a_margen[i]
                } else {
                    0.0
                }
            }),
            margen_tect_m: std::array::from_fn(|i| {
                if a_margen[i] > 0.0 {
                    tect_margen[i] / a_margen[i]
                } else {
                    0.0
                }
            }),
            margen_area_km2: a_margen,
            costa_margen_km,
            franja_estirada_km,
        });
    }

    /// Márgenes pasivos por sus celdas de costa (nota de fosas v4 §8): celdas continentales (no de
    /// arco) con fondo oceánico de su placa al lado y sin otra placa a menos de `L_borde`. Su edad es
    /// la del fondo más viejo de al lado (el tiempo desde que se abrió ese océano). Devuelve la
    /// longitud (km) y la edad media ponderada por longitud y la máxima (Myr), y deja sus celdas en
    /// `costa_margen` y `margenes_previos`. La vida de los márgenes se mide como entidades
    /// ([`MargenRift`]): por celdas de costa, el vaivén de la malla la falsea.
    fn margenes_pasivos(&mut self) -> (f64, f64, f64) {
        let num = self.malla.num;
        let lado = self.malla.lado_km;
        let mut ahora: Vec<((u64, u32), f64)> = Vec::new();
        self.costa_margen.clear();
        let (mut largo, mut edad_l, mut edad_max) = (0.0, 0.0, 0.0f64);
        for k in 0..num {
            let p = self.dueno[k];
            if p == NADA || self.rep[k] == NADA || !self.corteza(k).is_some_and(|c| c.continente())
            {
                continue;
            }
            let edad = self.malla.vec[k]
                .iter()
                .filter(|&&w| w != NADA && self.dueno[w as usize] == p)
                .filter_map(|&w| {
                    self.corteza(w as usize)
                        .filter(|c| !c.cont)
                        .map(|c| c.edad_oc)
                })
                .fold(f64::NEG_INFINITY, f64::max);
            if !edad.is_finite()
                || self.vecindad.de(k).iter().any(|&j| {
                    let d = self.dueno[j as usize];
                    d != NADA && d != p
                })
            {
                continue;
            }
            ahora.push(((self.placas[p as usize].id, self.rep[k]), edad));
            self.costa_margen.push(k as u32);
            largo += lado;
            edad_l += edad * lado;
            edad_max = edad_max.max(edad);
        }
        ahora.sort_unstable_by_key(|x| x.0);
        self.margenes_previos = ahora;
        (
            largo,
            if largo > 0.0 { edad_l / largo } else { 0.0 },
            edad_max,
        )
    }

    /// Perfil de los márgenes pasivos del paso (nota de rifts §8), desde las celdas de su costa
    /// (`costa_margen`) hacia dentro por la corteza de su continente: espesor medio de la corteza a
    /// menos de 50 km de la costa y anchura de la franja estirada, el centro de la primera clase de
    /// distancia (0–50, 50–100, 100–150, 150–200 y 200–300 km) en que la corteza llega al 90 % de la
    /// de 200–300 km. Sin márgenes, (0, 0).
    fn perfil_de_margenes(&mut self, cont: &[bool]) -> (f64, f64) {
        if self.costa_margen.is_empty() {
            return (0.0, 0.0);
        }
        let media = 0.5 * self.malla.lado_km;
        let fuentes: Vec<(u32, f64, u32)> =
            self.costa_margen.iter().map(|&k| (k, media, 0)).collect();
        let dueno = &self.dueno;
        let mut campo = std::mem::take(&mut self.campo);
        campo.calcular(
            &self.malla,
            &fuentes,
            |u, v| dueno[u] == dueno[v] && cont[v],
            300.0,
        );
        let limites = [50.0, 100.0, 150.0, 200.0, 300.0];
        let mut suma = [(0.0f64, 0.0f64); 5];
        for &k in &campo.orden {
            let ku = k as usize;
            let i = limites
                .iter()
                .position(|&x| campo.dist[ku] < x)
                .unwrap_or(4);
            if let Some(c) = self.corteza(ku) {
                suma[i].0 += c.h * self.malla.area[ku];
                suma[i].1 += self.malla.area[ku];
            }
        }
        self.campo = campo;
        let perfil: Vec<f64> = suma
            .iter()
            .map(|x| if x.1 > 0.0 { x.0 / x.1 } else { f64::NAN })
            .collect();
        let centros = [25.0, 75.0, 125.0, 175.0, 250.0];
        let franja = centros
            .iter()
            .zip(&perfil)
            .find(|&(_, &h)| h >= 0.9 * perfil[4])
            .map_or(0.0, |(&c, _)| c);
        (
            if perfil[0].is_finite() {
                perfil[0]
            } else {
                0.0
            },
            franja,
        )
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
            if self.pares[i].choque_avisado && self.t_myr - self.pares[i].choque_ultimo_myr > 20.0 {
                self.fin_de_choque(i);
            }
            if c.is_none() && self.t_myr - self.pares[i].choque_visto_myr > 20.0 {
                self.pares[i].choque = None;
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
        // Olvidar los pares de placas que ya no existen (sus choques acaban al fundirse).
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
        for i in 0..self.pares.len() {
            let (a, b) = (self.pares[i].a, self.pares[i].b);
            if self.pares[i].choque_avisado
                && (vivas.binary_search(&a).is_err() || vivas.binary_search(&b).is_err())
            {
                self.fin_de_choque(i);
            }
        }
        self.pares
            .retain(|p| vivas.binary_search(&p.a).is_ok() && vivas.binary_search(&p.b).is_ok());
    }

    /// Fin de un choque (adenda de balance §7): tipo, eficacia efectiva y volúmenes acumulados.
    fn fin_de_choque(&mut self, i: usize) {
        let par = &mut self.pares[i];
        let (sol, perd, arco) = (par.choque_solapado, par.choque_perdido, par.choque_arco);
        let duracion = (par.choque_ultimo_myr - par.choque_inicio_myr).max(0.0);
        let placas = vec![par.a, par.b];
        par.choque_avisado = false;
        par.choque_solapado = 0.0;
        par.choque_perdido = 0.0;
        par.choque_arco = 0.0;
        let clase = if arco > 0.5 * sol {
            "arco_continente"
        } else {
            "continente_continente"
        };
        self.evento_det(
            "fin_choque",
            placas,
            0.0,
            0.0,
            None,
            Detalle {
                clase: Some(clase.to_string()),
                eta: (sol > 0.0).then(|| 1.0 - perd / sol),
                solapado_km3: Some(sol),
                perdido_km3: Some(perd),
                tiempo_choque_myr: Some(duracion),
                ..Detalle::default()
            },
        );
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

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_mezcla_de_edades_es_por_volumen_y_no_cambia_ningun_volumen() {
        // Nota de isostasia con litosfera §10, prueba 3: juntar corteza mezcla la edad
        // tectonotérmica por volumen y no toca ningún volumen (el sedimento pasa a roca como antes).
        let (rs, rc) = (2400.0, 2800.0);
        let mut c = Corteza {
            edad_tt: 1000.0,
            sed: 1.4,
            ..Corteza::continental(30.0)
        };
        let roca = 1.4 * rs / rc;
        let (h0, hc0) = (c.h, c.hc);
        c.sumar_hc(10.0, 0.0, 200.0, rs, rc, 20.0, via::CHOQUE);
        assert!((c.h - (h0 + roca + 10.0)).abs() < 1e-12);
        assert!((c.hc - (hc0 + roca + 10.0)).abs() < 1e-12);
        assert_eq!(c.sed, 0.0);
        let esperada = (1000.0 * (hc0 + roca) + 200.0 * 10.0) / (hc0 + roca + 10.0);
        assert!((c.edad_tt - esperada).abs() < 1e-9, "{}", c.edad_tt);
        // Una columna oceánica sin corteza de tipo continental toma la edad del aporte.
        let mut o = Corteza {
            edad_tt: 80.0,
            ..Corteza::oceanica(7.0)
        };
        o.sumar_hc(5.0, 1.0, 0.0, rs, rc, 20.0, via::ARCO);
        assert_eq!(o.edad_tt, 0.0);
    }
}
