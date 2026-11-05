//! Agotamiento de humedad a favor del viento (nota de sombra pluviométrica §3).
//!
//! El aire húmedo que sube por una cordillera se enfría y suelta su vapor en barlovento; al bajar
//! por sotavento se calienta, pero no recupera el agua. Por eso, detrás de las montañas llueve
//! menos durante cientos de km. Para cada punto de tierra se recorre el camino del viento hacia
//! atrás, en pasos de `d_oro`, hasta el mar o hasta `L_traj`, y se suman las subidas del relieve de
//! referencia (`h_oro`, el mismo de la lluvia orográfica), pesadas por la recarga sobre tierra:
//!
//! ```text
//! Λ = Σ_j max(0, Δh_oro,j) · e^(−s_j / L_recarga)          w = e^(−Λ / H_v)
//! ```
//!
//! Por decisión del responsable (S3, lo más realista), `Λ` no suma cada subida del camino: es la
//! **altura de saturación** del aire, que solo sube cuando el terreno supera lo que el aire ya subió y
//! se recarga sobre tierra con `L_recarga` (ver [`lambda_camino`]). Para una cordillera que se alza
//! sobre un llano da lo mismo; sobre terreno ondulado, las lomas pequeñas ya no secan el aire.
//!
//! Cómo se calcula:
//! - **Anillos.** El viento del modelo es zonal: sopla solo hacia el este o el oeste, según la
//!   franja de latitud. Por eso el camino hacia atrás de un punto es su propio paralelo. `Λ` se
//!   calcula sobre anillos: paralelos separados `d_oro` en latitud, con puntos cada `d_oro` a lo
//!   largo de cada uno. Si algún día el viento tiene componente norte-sur, habrá que cambiar los
//!   anillos por caminos de verdad.
//! - **Relieve de cada punto.** El de referencia de la nota de lluvia orográfica: el de nacimiento
//!   al nivel `L_oro` más la erosión vista a ese nivel. No depende de la malla.
//! - **Mar.** Un punto cuyo relieve de referencia queda bajo el nivel del mar. Ahí el aire se
//!   recarga y la suma se corta; el tramo de la costa cuenta la subida desde el nivel del mar.
//! - **Celdas.** La de nivel `L_oro` o más fina toma `w` interpolado en su centro. La más gruesa,
//!   la media de sus descendientes de nivel `L_oro` (como mucho 8×8 muestras). Así, juntar o
//!   separar celdas por encima de `L_oro` no cambia la media. En cada punto, `w` se interpola solo
//!   con los puntos de tierra de los anillos (mezclar el mar, con `w = 1`, daría a la tierra de la
//!   costa humedad del mar); lo que a esa escala es mar cuenta con `w = 1`. Las celdas de mar
//!   tienen `w = 1`.
//!
//! `w` es casi estático: se calcula al acabar la génesis y se rehace en cada paso de erosión y en
//! cada cambio de malla. Es función pura de (relieve de referencia, viento, mar): determinista.

use std::f64::consts::{FRAC_PI_2, PI};

use crate::config::{ClimateCfg, TerrenoConfig};
use crate::erosion;
use crate::geometry::{V3, center_dir, face_n, lat_lon, split_key};
use crate::systems::{oro_slope_e, precip_no_season, wind_east};
use crate::world::World;

/// Máximo de muestras por lado al promediar `w` sobre una celda gruesa (8×8 = 64).
const MUESTRAS_LADO_MAX: u32 = 3; // 2^3 = 8

/// `Λ` (m) de un punto: cuánto ha subido el aire que le llega, medido como su **altura de
/// saturación** sobre el nivel del mar (nota de sombra pluviométrica §7, decisión S3).
///
/// `camino` da `(h, es_mar)`: el primer elemento es el propio punto y cada uno de los siguientes
/// está un paso (`ds_km`) más contra el viento. Se recorre a favor del viento desde el extremo (el
/// primer punto de mar, o `max_tramos` pasos atrás) hasta el punto:
/// - sobre el mar el aire está fresco: su altura de saturación `z` es el nivel del mar;
/// - si el terreno sube por encima de `z`, el aire suelta vapor y `z` sube con él;
/// - si baja o sigue llano, el aire conserva su `z` (bajar lo calienta, pero no le devuelve el
///   agua) y se recarga sobre tierra: `z` se acerca al suelo como `e^(−paso/L_recarga)`.
///
/// Así, una loma que no sube por encima de lo que el aire ya subió no lo vuelve a secar, y sobre una
/// meseta alta el aire lleva menos vapor. Si el camino se corta por `max_tramos` sin llegar al mar,
/// el aire de su extremo empieza fresco a la altura de su suelo. `buf` se reutiliza entre llamadas.
pub fn lambda_camino<I: Iterator<Item = (f64, bool)>>(
    camino: I,
    ds_km: f64,
    l_recarga_km: f64,
    nivel_mar: f64,
    max_tramos: usize,
    buf: &mut Vec<(f64, bool)>,
) -> f64 {
    buf.clear();
    for (h, mar) in camino.take(max_tramos + 1) {
        buf.push((h, mar));
        if mar {
            break;
        }
    }
    let Some(&(_, mar_punto)) = buf.first() else {
        return 0.0;
    };
    if mar_punto {
        return 0.0;
    }
    let decae = (-ds_km / l_recarga_km).exp();
    // Extremo del camino: el mar (aire fresco al nivel del mar) o el corte por longitud.
    let (mut z, desde) = match buf.last() {
        Some(&(_, true)) => (nivel_mar, buf.len() - 1),
        Some(&(h, false)) => (h.max(nivel_mar), buf.len() - 1),
        None => return 0.0,
    };
    for &(h, _) in buf[..desde].iter().rev() {
        z = h + (z - h).max(0.0) * decae;
    }
    (z - nivel_mar).max(0.0)
}

/// Fracción de humedad que le queda al aire tras `Λ` metros de subida: `w = e^(−Λ/H_v)`.
#[inline]
pub fn w_de_lambda(lambda_m: f64, c: &ClimateCfg) -> f64 {
    (-lambda_m / c.h_v_m).exp()
}

/// Anillos del camino del viento: paralelos a paso `d_oro` con puntos a paso `d_oro`.
#[derive(Debug, Clone)]
pub struct Anillos {
    /// Paso en latitud (rad) y latitud de cada anillo.
    d_lat: f64,
    lat: Vec<f64>,
    /// Primer punto de cada anillo (`off[k]..off[k+1]`).
    off: Vec<usize>,
    /// Paso a lo largo de cada anillo (km) y sentido del viento (+1 hacia el este, −1 al oeste).
    ds_km: Vec<f64>,
    viento: Vec<f64>,
    /// Dirección de cada punto y su relieve de nacimiento al nivel `L_oro` (puro).
    dir: Vec<V3>,
    h0: Vec<f64>,
    /// Relieve de referencia actual (con la erosión vista a `L_oro`), mar y `w`.
    pub h: Vec<f64>,
    pub mar: Vec<bool>,
    pub w: Vec<f64>,
}

impl Anillos {
    /// Construye los anillos y el relieve de nacimiento de sus puntos (función pura de la config).
    pub fn nuevos(world: &World, cfg: &TerrenoConfig) -> Anillos {
        let r_km = cfg.planet.r_km;
        let d = cfg.climate.d_oro_km;
        let l_oro = world.statgen.oro_level();
        let n_anillos = ((PI * r_km / d).round() as usize).max(2);
        let d_lat = PI / n_anillos as f64;
        let mut an = Anillos {
            d_lat,
            lat: Vec::with_capacity(n_anillos),
            off: Vec::with_capacity(n_anillos + 1),
            ds_km: Vec::with_capacity(n_anillos),
            viento: Vec::with_capacity(n_anillos),
            dir: Vec::new(),
            h0: Vec::new(),
            h: Vec::new(),
            mar: Vec::new(),
            w: Vec::new(),
        };
        an.off.push(0);
        for k in 0..n_anillos {
            let lat = -FRAC_PI_2 + (k as f64 + 0.5) * d_lat;
            let circ = 2.0 * PI * r_km * lat.cos();
            let n = ((circ / d).round() as usize).max(3);
            an.lat.push(lat);
            an.ds_km.push(circ / n as f64);
            an.viento.push(wind_east(lat.to_degrees().abs()));
            let (sl, cl) = lat.sin_cos();
            for i in 0..n {
                let lon = -PI + (i as f64 + 0.5) * 2.0 * PI / n as f64;
                let (so, co) = lon.sin_cos();
                let p = [cl * co, cl * so, sl];
                an.dir.push(p);
                an.h0.push(world.statgen.h_at(p, l_oro));
            }
            an.off.push(an.dir.len());
        }
        let n = an.dir.len();
        an.h = an.h0.clone();
        an.mar = vec![false; n];
        an.w = vec![1.0; n];
        an
    }

    /// Nº de puntos de todos los anillos.
    pub fn len(&self) -> usize {
        self.dir.len()
    }

    /// `true` si no hay puntos.
    pub fn is_empty(&self) -> bool {
        self.dir.is_empty()
    }

    /// Rehace el relieve de referencia de los puntos (con la erosión actual), el mar y `w`.
    pub fn actualizar(&mut self, world: &World, cfg: &TerrenoConfig) {
        let sea = world.statgen.sea_level();
        let l_oro = world.statgen.oro_level();
        if world.e.iter().all(|&x| x == 0.0) {
            self.h.copy_from_slice(&self.h0);
        } else {
            let means = erosion::ancestor_means(world, l_oro);
            for p in 0..self.dir.len() {
                self.h[p] = self.h0[p] + erosion::e_ref_at(world, &means, l_oro, self.dir[p]);
            }
        }
        for (m, &h) in self.mar.iter_mut().zip(&self.h) {
            *m = h < sea;
        }
        let c = &cfg.climate;
        let mut buf = Vec::new();
        for k in 0..self.lat.len() {
            let (a, b) = (self.off[k], self.off[k + 1]);
            let n = b - a;
            let ds = self.ds_km[k];
            // Sin dar más de una vuelta al anillo (cerca de los polos es corto).
            let tramos = ((c.l_traj_km / ds).floor() as usize).min(n - 1);
            // Hacia atrás = contra el viento: hacia el oeste si sopla hacia el este, y al revés.
            let atras = if self.viento[k] > 0.0 { n - 1 } else { 1 };
            let (h, mar) = (&self.h[a..b], &self.mar[a..b]);
            for i in 0..n {
                let camino = (0..=tramos).map(|j| {
                    let q = (i + j * atras) % n;
                    (h[q], mar[q])
                });
                let lam = lambda_camino(camino, ds, c.l_recarga_km, sea, tramos, &mut buf);
                self.w[a + i] = if mar[i] { 1.0 } else { w_de_lambda(lam, c) };
            }
        }
    }

    /// Los cuatro puntos de los anillos que rodean una dirección, con sus pesos bilineales: lineal a
    /// lo largo de los dos anillos más cercanos y lineal entre ellos.
    fn vecinos(&self, dir: V3) -> [(usize, f64); 4] {
        let (lat, lon) = lat_lon(dir);
        let f = (lat + FRAC_PI_2) / self.d_lat - 0.5;
        let k0 = f.floor();
        let t = f - k0;
        let ultimo = self.lat.len() as i64 - 1;
        let en_anillo = |k: i64| -> [(usize, f64); 2] {
            let k = k.clamp(0, ultimo) as usize;
            let (a, b) = (self.off[k], self.off[k + 1]);
            let n = b - a;
            let g = (lon + PI) / (2.0 * PI / n as f64) - 0.5;
            let i0 = g.floor();
            let s = g - i0;
            let i0 = (i0 as i64).rem_euclid(n as i64) as usize;
            [(a + i0, 1.0 - s), (a + (i0 + 1) % n, s)]
        };
        let k0 = k0 as i64;
        let [p0, p1] = en_anillo(k0);
        let [p2, p3] = en_anillo(k0 + 1);
        [
            (p0.0, (1.0 - t) * p0.1),
            (p1.0, (1.0 - t) * p1.1),
            (p2.0, t * p2.1),
            (p3.0, t * p3.1),
        ]
    }

    /// `w` en una dirección, interpolado **solo con los puntos de tierra**: el mar tiene `w = 1`,
    /// y mezclarlo daría a la tierra de la costa humedad del mar (y haría depender la lluvia de
    /// cuánto mar cae dentro de cada celda). Devuelve `(w, peso de tierra)`; sin tierra
    /// alrededor, `(1, 0)`.
    pub fn w_tierra_en(&self, dir: V3) -> (f64, f64) {
        let (mut acc, mut peso) = (0.0, 0.0);
        for (p, b) in self.vecinos(dir) {
            if !self.mar[p] {
                acc += b * self.w[p];
                peso += b;
            }
        }
        if peso > 0.0 {
            (acc / peso, peso)
        } else {
            (1.0, 0.0)
        }
    }
}

/// Al acabar la génesis: activa el agotamiento (nota §3: `w` se calcula después del prerrodaje de
/// erosión) y fija `c_P` (nota §2).
pub fn activar(world: &mut World, cfg: &TerrenoConfig) {
    world.humedad_activa = true;
    refrescar(world, cfg);
    let (desvio, c_p) = factor_conservacion(world, cfg);
    world.desvio_lluvia = desvio;
    world.c_p = c_p;
}

/// Rehace `w` de todas las celdas (tras un paso de erosión o un cambio de malla). Durante la
/// génesis geológica aún no está activo.
pub fn refrescar(world: &mut World, cfg: &TerrenoConfig) {
    if !world.humedad_activa {
        return;
    }
    let mut an = match world.anillos.take() {
        Some(a) => a,
        None => Box::new(Anillos::nuevos(world, cfg)),
    };
    an.actualizar(world, cfg);
    let l_oro = world.statgen.oro_level();
    let num = world.topo.num_cells;
    world.w_hum.resize(num, 1.0);
    for s in 0..num {
        world.w_hum[s] = if world.es_oceano(s) {
            1.0
        } else if u32::from(world.topo.level[s]) >= l_oro {
            an.w_tierra_en(world.topo.dir[s]).0
        } else {
            w_media_celda(&an, world, s, l_oro)
        };
    }
    world.anillos = Some(an);
}

/// Media de `w` sobre la huella de una celda más gruesa que `L_oro`: en los centros de sus
/// descendientes de nivel `L_oro` (o de un nivel intermedio, si serían más de 8×8). Lo que a esa
/// escala es mar cuenta con `w = 1`, porque la celda gruesa representa también ese trozo.
fn w_media_celda(an: &Anillos, world: &World, s: usize, l_oro: u32) -> f64 {
    let (c, l, i, j) = split_key(world.topo.keys[s]);
    let d = (l_oro - l).min(MUESTRAS_LADO_MAX);
    let m = 1u32 << d;
    let n = face_n(world.topo.n0, l + d);
    let mut acc = 0.0;
    for jj in 0..m {
        for ii in 0..m {
            acc += an.w_tierra_en(center_dir(c, i * m + ii, j * m + jj, n)).0;
        }
    }
    acc / f64::from(m * m)
}

/// Desviación de la lluvia media anual sobre tierra y factor `c_P` que la corrige (nota §2). La
/// referencia es la lluvia sin efecto orográfico ni agotamiento (franja y distancia a la costa): la
/// orografía de antes de la nota (`k_oro = 0,5`) la dejaba a un 0,3 %, así que es "la anterior".
/// `c_P = 1` si la desviación cabe en `tol_lluvia`; si no, `c_P = lluvia de referencia / lluvia
/// actual`, que recupera el total. Es un apaño provisional (CLAUDE §2.8): ver la nota §7.
pub fn factor_conservacion(world: &World, cfg: &TerrenoConfig) -> (f64, f64) {
    let (mut referencia, mut actual) = (0.0, 0.0);
    for s in 0..world.topo.num_cells {
        if world.es_oceano(s) {
            continue;
        }
        let (lat, dist, a) = (
            world.topo.lat[s],
            world.dist_ocean_km[s],
            world.topo.area[s],
        );
        referencia += precip_no_season(cfg, lat, dist, 0.0) * a;
        actual += precip_no_season(cfg, lat, dist, oro_slope_e(world, s)) * world.w_hum[s] * a;
    }
    if !(referencia > 0.0 && actual > 0.0) {
        return (0.0, 1.0);
    }
    let desvio = actual / referencia - 1.0;
    let c = &cfg.climate;
    let c_p = if desvio.abs() > c.tol_lluvia {
        referencia / actual
    } else {
        1.0
    };
    (desvio, c_p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Perfil de juguete de la nota (§4): mar, llanura costera, cordillera norte-sur de 3 000 m con
    /// vientos del oeste y llanura de sotavento. Devuelve (x en km, h en m, es_mar), con el viento
    /// soplando hacia `x` creciente.
    fn perfil(ds: f64) -> Vec<(f64, f64, bool)> {
        let n = (3000.0 / ds) as usize;
        (0..n)
            .map(|k| {
                let x = k as f64 * ds - 500.0; // el mar hasta x = 0
                let h = if x < 0.0 {
                    -2000.0
                } else if x < 500.0 {
                    200.0 // llanura de barlovento
                } else if x < 575.0 {
                    200.0 + 2800.0 * (x - 500.0) / 75.0 // ladera de barlovento
                } else if x < 650.0 {
                    3000.0 - 2500.0 * (x - 575.0) / 75.0 // ladera de sotavento
                } else {
                    500.0 // llanura de sotavento (meseta, como la estepa patagónica)
                };
                (x, h, x < 0.0)
            })
            .collect()
    }

    #[test]
    fn el_aire_se_seca_al_subir_y_se_recarga_sobre_tierra() {
        let mut buf = Vec::new();
        let mut lam = |camino: &[(f64, bool)], ds: f64, l: f64, tramos: usize| {
            lambda_camino(camino.iter().copied(), ds, l, 0.0, tramos, &mut buf)
        };
        // Llano a 300 m tras la costa: el aire subió 300 m y, sobre el llano, ya no baja de ahí.
        let llano = [
            (300.0, false),
            (300.0, false),
            (300.0, false),
            (-50.0, true),
        ];
        assert!((lam(&llano, 10.0, 1000.0, 10) - 300.0).abs() < 1e-9);
        // Una loma de 500 m: el aire sigue con la altura que alcanzó y se recarga despacio.
        let loma = [(0.0, false), (500.0, false), (0.0, false), (0.0, true)];
        let esperado = 500.0 * (-10.0 / 1000.0f64).exp();
        assert!((lam(&loma, 10.0, 1000.0, 10) - esperado).abs() < 1e-9);
        // Lomas repetidas a la misma altura no vuelven a secar el aire (antes sumaban 600 m).
        let lomas = [
            (0.0, false),
            (300.0, false),
            (0.0, false),
            (300.0, false),
            (0.0, false),
            (0.0, true),
        ];
        assert!((lam(&lomas, 10.0, 1e12, 10) - 300.0).abs() < 1e-6);
        // Si el camino se corta antes del mar, el aire empieza fresco a la altura de su suelo.
        assert!((lam(&loma, 10.0, 1e12, 1) - 500.0).abs() < 1e-6);
        // En el mar no hay agotamiento.
        assert_eq!(lam(&[(-10.0, true)], 10.0, 1e3, 5), 0.0);
    }

    #[test]
    fn escenario_de_juguete_de_la_nota() {
        // Nota §4: cordillera norte-sur de 3 000 m con vientos del oeste. Lluvia relativa a la de
        // la llanura de barlovento: P ∝ m_oro · w (la franja y la costa no cambian en el perfil).
        let c = ClimateCfg::default();
        let ds = c.d_oro_km;
        let p = perfil(ds);
        let tramos = (c.l_traj_km / ds).floor() as usize;
        let mut buf = Vec::new();
        let lluvia: Vec<f64> = (0..p.len())
            .map(|i| {
                if p[i].2 {
                    return f64::NAN;
                }
                let camino = (0..=tramos.min(i)).map(|j| (p[i - j].1, p[i - j].2));
                let lam = lambda_camino(camino, ds, c.l_recarga_km, 0.0, tramos, &mut buf);
                let w = w_de_lambda(lam, &c);
                // Pendiente con diferencias centrales a ±d_oro, y el mar plano.
                let (hm, hp) = (p[i - 1].1.max(0.0), p[(i + 1).min(p.len() - 1)].1.max(0.0));
                let grad = (hp - hm) / (2.0 * ds * 1000.0);
                let m_oro = (1.0 + c.k_oro * grad).clamp(c.m_oro_min, c.m_oro_max);
                m_oro * w
            })
            .collect();
        let en = |x: f64| lluvia[((x + 500.0) / ds).round() as usize];
        let llanura = en(250.0);
        // Barlovento: el pie (último punto llano y primero de la ladera) y lo alto de la ladera.
        let barlovento: Vec<f64> = [500.0, 525.0, 550.0, 575.0]
            .iter()
            .map(|&x| en(x) / llanura)
            .collect();
        // Llanura de sotavento, de 200 a 500 km de la cresta (x = 575 km).
        let sotavento: Vec<f64> = [775.0, 875.0, 975.0, 1075.0]
            .iter()
            .map(|&x| en(x) / llanura)
            .collect();
        let max_barlovento = barlovento.iter().copied().fold(0.0, f64::max);
        let contraste = max_barlovento / sotavento[0];
        println!(
            "barlovento (pie → cresta) {barlovento:.2?} · sotavento a 200–500 km {sotavento:.2?} · contraste {contraste:.1}"
        );
        // Medido con la config por defecto (nota de sombra §7.3), con valores físicos (k_oro = 40,
        // H_v = 2 500 m, L_recarga = 3 000 km): pie de barlovento 1,75 (la nota esperaba ~2–3: la
        // propia ladera ya seca el aire y el paso de 25 km suaviza el pie); al subir la ladera
        // llueve menos (1,72 · 1,18 · cresta 0,35); sotavento a 200–500 km 0,35–0,38 (la nota:
        // 0,2–0,4); contraste 5,1 (la nota: 5–20).
        assert!((1.6..=3.2).contains(&barlovento[0]) && (1.6..=3.2).contains(&barlovento[1]));
        assert!(barlovento.windows(2).all(|v| v[1] <= v[0] + 1e-12));
        // Sotavento: entre el 20 y el 40 % de la llanura de barlovento, y algo más al alejarse.
        assert!(sotavento.iter().all(|&x| (0.2..0.45).contains(&x)));
        assert!(sotavento.windows(2).all(|v| v[1] >= v[0]));
        assert!((5.0..=20.0).contains(&contraste), "contraste {contraste}");
    }
}
