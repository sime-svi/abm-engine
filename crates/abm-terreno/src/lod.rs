//! LOD dinámico (incremento 2): vecindad multinivel, operadores de reparto conservativos, driver de
//! refinamiento, equilibrio 2:1 y paso de topología.
//!
//! Diseño: `docs/design/terreno/incremento-2-diseno.md` §5 (vecindad y 2:1), §6 (operadores),
//! §8 (driver) y §9 (paso de topología). Todo es función determinista del estado, iterada en orden
//! de clave. Las únicas asignaciones de memoria ocurren en los pasos de topología (cada `n_lod`
//! ticks), nunca en el bucle por tick.

use std::f64::consts::FRAC_1_SQRT_2;

use crate::config::TerrenoConfig;
use crate::erosion;
use crate::geometry::{
    MOORE8, child_key, es_diagonal, face_n, key, neighbor_addr, parent_key, split_key,
};
use crate::hydro;
use crate::statics::CellGeom;
use crate::systems::whittaker;
use crate::world::{Biome, Pair, Surface, Topology, World};

/// Orden fijo de las 4 hijas de una celda, como desplazamientos `(di, dj)`.
pub const CHILDREN: [(u32, u32); 4] = [(0, 0), (1, 0), (0, 1), (1, 1)];

// =================================================================================================
// Vecindad multinivel (§5)
// =================================================================================================

/// Construye la vecindad de todas las hojas: pares únicos ordenados con su peso y la lista CSR.
///
/// Para cada hoja se calcula su **anillo** al nivel `L+1` (los vecinos de Moore de sus 4 hijas
/// virtuales que no son hijas suyas), marcando cada celda del anillo como *lado* (alcanzada en
/// ortogonal) o *esquina* (solo en diagonal), y se resuelve cada una a la hoja que la contiene: ella
/// misma (vecino más fino), su padre (mismo nivel) o su abuelo (más grueso). Los pares se recogen
/// desde todas las hojas y se fusionan, así que la relación es **simétrica por construcción**.
pub fn build_neighbors(topo: &mut Topology) {
    let num = topo.num_cells;
    let mut raw: Vec<(u32, u32, bool)> = Vec::with_capacity(num * 9);
    let mut ring: Vec<(u64, bool)> = Vec::with_capacity(16);
    let mut found: Vec<(u32, bool)> = Vec::with_capacity(16);

    for s in 0..num {
        let (c, l, i, j) = split_key(topo.keys[s]);
        let n1 = face_n(topo.n0, l + 1);
        ring.clear();
        for &(di, dj) in CHILDREN.iter() {
            let (ci, cj) = (2 * i + di, 2 * j + dj);
            for (k, &(mi, mj)) in MOORE8.iter().enumerate() {
                let Some((c2, i2, j2)) = neighbor_addr(c, ci, cj, n1, mi, mj) else {
                    continue; // esquina del cubo
                };
                if c2 == c && i2 / 2 == i && j2 / 2 == j {
                    continue; // otra hija de la propia hoja
                }
                let rk = key(c2, l + 1, i2, j2);
                let lado = !es_diagonal(k);
                if let Some(e) = ring.iter_mut().find(|e| e.0 == rk) {
                    e.1 |= lado;
                } else {
                    ring.push((rk, lado));
                }
            }
        }

        found.clear();
        for &(rk, lado) in ring.iter() {
            let Some(b) = resolve(topo, rk, l) else {
                debug_assert!(false, "árbol desequilibrado: anillo sin hoja");
                continue;
            };
            if b == s {
                continue;
            }
            let b = b as u32;
            if let Some(e) = found.iter_mut().find(|e| e.0 == b) {
                e.1 |= lado;
            } else {
                found.push((b, lado));
            }
        }
        let a = s as u32;
        for &(b, lado) in found.iter() {
            raw.push((a.min(b), a.max(b), lado));
        }
    }

    // Pares únicos: cada par aparece desde sus dos lados; se fusionan (lado si cualquiera lo ve así).
    raw.sort_unstable_by_key(|p| (p.0, p.1));
    let mut pairs: Vec<Pair> = Vec::with_capacity(raw.len() / 2 + 1);
    for (a, b, lado) in raw {
        if let Some(last) = pairs.last_mut()
            && last.a == a
            && last.b == b
        {
            if lado {
                last.g = 1.0;
            }
            continue;
        }
        pairs.push(Pair {
            a,
            b,
            g: if lado { 1.0 } else { FRAC_1_SQRT_2 },
        });
    }

    // Lista CSR (offsets + vecinos ordenados).
    let mut deg = vec![0u32; num];
    for p in &pairs {
        deg[p.a as usize] += 1;
        deg[p.b as usize] += 1;
    }
    let mut off = Vec::with_capacity(num + 1);
    let mut acc = 0u32;
    off.push(0u32);
    for &d in &deg {
        acc += d;
        off.push(acc);
    }
    let mut cursor = off.clone();
    let mut nb = vec![0u32; acc as usize];
    for p in &pairs {
        for (x, y) in [(p.a, p.b), (p.b, p.a)] {
            let c = &mut cursor[x as usize];
            nb[*c as usize] = y;
            *c += 1;
        }
    }
    for w in off.windows(2) {
        nb[w[0] as usize..w[1] as usize].sort_unstable();
    }

    topo.nb_off = off;
    topo.nb = nb;
    topo.pairs = pairs;
}

/// Hoja que contiene la celda `rk` del anillo (nivel `l+1`) de una hoja de nivel `l`.
fn resolve(topo: &Topology, rk: u64, l: u32) -> Option<usize> {
    if let Some(s) = topo.slot_of(rk) {
        return Some(s); // vecino más fino
    }
    let p = parent_key(rk);
    if let Some(s) = topo.slot_of(p) {
        return Some(s); // vecino del mismo nivel
    }
    if l > topo.l_min {
        return topo.slot_of(parent_key(p)); // vecino más grueso
    }
    None
}

// =================================================================================================
// Operadores de reparto del estado dinámico (§6)
// =================================================================================================

/// Estado extensivo de una celda que el LOD reparte: agua del suelo y nieve (mm) y biomasa.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaterState {
    pub w: f64,
    pub swe: f64,
    pub veg: f64,
}

impl WaterState {
    pub const ZERO: WaterState = WaterState {
        w: 0.0,
        swe: 0.0,
        veg: 0.0,
    };
}

/// Celda de origen de un reparto: su estado, su área (m²) y si es océano.
#[derive(Debug, Clone, Copy)]
pub struct CellIn {
    pub state: WaterState,
    pub area: f64,
    pub ocean: bool,
}

/// Celda de destino de un reparto: área (m²), capacidad de campo (mm) y si es océano.
#[derive(Debug, Clone, Copy)]
pub struct CellCap {
    pub area: f64,
    pub w_cap: f64,
    pub ocean: bool,
}

/// Lo que un reparto saca del balance: agua al mar (m³) y biomasa perdida/creada (densidad·m²).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Transfer {
    pub to_ocean_vol: f64,
    pub veg_lost: f64,
    pub veg_created: f64,
}

/// Subdivisión (§6.1): reparte el estado de la madre entre sus 4 hijas.
///
/// Solo las hijas de **tierra** reciben estado. Nieve y biomasa conservan `Σ X·a` (densidad
/// reescalada al área de tierra); el agua se reparte por capacidad (misma saturación en todas) y lo
/// que no cabe va al mar. Madre de océano => tierra nueva (`W = SWE = 0`, `V = v_init`).
pub fn split_state(
    mother: &CellIn,
    kids: &[CellCap; 4],
    v_init: f64,
) -> ([WaterState; 4], Transfer) {
    let mut out = [WaterState::ZERO; 4];
    let mut tr = Transfer::default();
    if mother.ocean {
        for (o, k) in out.iter_mut().zip(kids) {
            if !k.ocean {
                o.veg = v_init;
                tr.veg_created += v_init * k.area;
            }
        }
        return (out, tr);
    }
    let m = mother.state;
    let (mut a_t, mut cap) = (0.0, 0.0);
    for k in kids.iter().filter(|k| !k.ocean) {
        a_t += k.area;
        cap += k.w_cap * k.area;
    }
    if a_t <= 0.0 {
        // Ninguna hija es tierra: el almacenamiento de la madre va al mar.
        tr.to_ocean_vol = (m.w + m.swe) / 1000.0 * mother.area;
        tr.veg_lost = m.veg * mother.area;
        return (out, tr);
    }
    // Hay tierra: nieve y biomasa van a las hijas de tierra aunque no admitan agua del suelo (roca
    // desnuda, `W_cap = 0`, incr. 3 §7); solo el agua del suelo que no cabe va al mar.
    let vw = m.w * mother.area; // mm·m²
    let sat = if cap <= 0.0 {
        tr.to_ocean_vol = vw / 1000.0;
        0.0
    } else if vw <= cap {
        vw / cap
    } else {
        tr.to_ocean_vol = (vw - cap) / 1000.0;
        1.0
    };
    let swe = m.swe * mother.area / a_t;
    let veg = m.veg * mother.area / a_t;
    for (o, k) in out.iter_mut().zip(kids) {
        if !k.ocean {
            *o = WaterState {
                w: k.w_cap * sat,
                swe,
                veg,
            };
        }
    }
    (out, tr)
}

/// Colapso (§6.2): agrega el estado de 4 hermanas en su madre (media por área de las de tierra).
/// Lo que rebasa la capacidad de la madre, o todo si la madre es océano, va al mar.
pub fn merge_state(kids: &[CellIn; 4], mother: &CellCap, v_init: f64) -> (WaterState, Transfer) {
    let mut tr = Transfer::default();
    if mother.ocean {
        for k in kids.iter().filter(|k| !k.ocean) {
            tr.to_ocean_vol += (k.state.w + k.state.swe) / 1000.0 * k.area;
            tr.veg_lost += k.state.veg * k.area;
        }
        return (WaterState::ZERO, tr);
    }
    if kids.iter().all(|k| k.ocean) {
        tr.veg_created = v_init * mother.area;
        return (
            WaterState {
                veg: v_init,
                ..WaterState::ZERO
            },
            tr,
        );
    }
    let (mut vw, mut vs, mut vv) = (0.0, 0.0, 0.0);
    for k in kids.iter().filter(|k| !k.ocean) {
        vw += k.state.w * k.area;
        vs += k.state.swe * k.area;
        vv += k.state.veg * k.area;
    }
    let mut w = vw / mother.area;
    if w > mother.w_cap {
        tr.to_ocean_vol = (w - mother.w_cap) * mother.area / 1000.0;
        w = mother.w_cap;
    }
    (
        WaterState {
            w,
            swe: vs / mother.area,
            veg: vv / mother.area,
        },
        tr,
    )
}

// =================================================================================================
// Driver de refinamiento (§8)
// =================================================================================================

/// Marcas del driver para un paso de topología.
pub struct Marks {
    /// La hoja pide subdividirse.
    pub want_sub: Vec<bool>,
    /// La familia de la hoja puede colapsarse (calma con histéresis + anticipación). El veto 2:1 lo
    /// aplica [`apply`].
    pub may_collapse: Vec<bool>,
    /// Verificar los colapsos con un ensayo en seco antes de aplicarlos (driver: sí; marcas forzadas
    /// de los tests de mecánica: no).
    pub verify_collapses: bool,
    /// Urgencia de cada petición de subdivisión, para el tope por revisión: infinita para los
    /// rasgos, y si no la mayor razón variación/umbral. Vacía en las marcas forzadas.
    pub urgency: Vec<f64>,
}

impl Marks {
    /// Marcas forzadas (tests de la mecánica): sin ensayo en seco ni urgencias.
    pub fn forced(want_sub: Vec<bool>, may_collapse: Vec<bool>) -> Marks {
        Marks {
            want_sub,
            may_collapse,
            verify_collapses: false,
            urgency: Vec::new(),
        }
    }
}

/// Tope de subdivisiones por revisión (protección contra el afinado desbocado): si piden
/// subdividirse más de `frac · hojas`, se quedan las más urgentes (desempate por clave).
pub fn cap_subdivisions(m: &mut Marks, frac: f64) {
    let n = m.want_sub.len();
    if frac <= 0.0 || m.urgency.len() != n {
        return;
    }
    let cap = ((frac * n as f64).ceil() as usize).max(1);
    let mut wanting: Vec<usize> = (0..n).filter(|&s| m.want_sub[s]).collect();
    if wanting.len() <= cap {
        return;
    }
    wanting.sort_by(|&a, &b| m.urgency[b].total_cmp(&m.urgency[a]).then(a.cmp(&b)));
    for &s in &wanting[cap..] {
        m.want_sub[s] = false;
    }
}

/// Familia (4 hermanas hojas, en el orden de [`CHILDREN`]) cuya primera hija es la hoja `s`.
fn family_of(topo: &Topology, s: usize) -> Option<[usize; 4]> {
    let (c, l, i, j) = split_key(topo.keys[s]);
    if l <= topo.l_min || i % 2 != 0 || j % 2 != 0 {
        return None;
    }
    let mut fam = [s; 4];
    for (f, &(di, dj)) in fam.iter_mut().zip(CHILDREN.iter()).skip(1) {
        *f = topo.slot_of(key(c, l, i + di, j + dj))?;
    }
    Some(fam)
}

/// Calcula las marcas del driver sobre el estado actual. Es el único punto de entrada de criterios:
/// los futuros disparadores de agentes y eventos se añadirán aquí.
pub fn marks(world: &World, cfg: &TerrenoConfig) -> Marks {
    let topo = &world.topo;
    let num = topo.num_cells;
    let lod = &cfg.lod;
    let ocean: Vec<bool> = (0..num).map(|s| world.es_oceano(s)).collect();
    let theta: Vec<f64> = (0..num).map(|s| world.theta(s)).collect();

    // Rasgos (costa/río/lago) y variaciones máximas con los vecinos de tierra: [Δh, ΔT, Δθ].
    let mut feature = vec![false; num];
    let mut var = vec![[0.0_f64; 3]; num];
    for s in 0..num {
        let nbs = topo.neighbors(s);
        if lod.refine_features {
            feature[s] = matches!(world.surface[s], Surface::Rio | Surface::Lago)
                || nbs.iter().any(|&b| ocean[b as usize] != ocean[s]);
        }
        if !ocean[s] {
            let v = &mut var[s];
            for &b in nbs {
                let b = b as usize;
                if ocean[b] {
                    continue;
                }
                v[0] = v[0].max((world.h[b] - world.h[s]).abs());
                v[1] = v[1].max((world.temp[b] - world.temp[s]).abs());
                v[2] = v[2].max((theta[b] - theta[s]).abs());
            }
        }
    }
    let thr = [lod.dh_sub, lod.dt_sub, lod.dtheta_sub];
    let exceeds =
        |v: &[f64; 3], factor: f64| v.iter().zip(thr.iter()).any(|(x, t)| *x > factor * t);

    let want_sub: Vec<bool> = (0..num)
        .map(|s| (topo.level[s] as u32) < topo.l_max && (feature[s] || exceeds(&var[s], 1.0)))
        .collect();
    let urgency: Vec<f64> = (0..num)
        .map(|s| {
            if !want_sub[s] {
                0.0
            } else if feature[s] {
                f64::INFINITY
            } else {
                var[s]
                    .iter()
                    .zip(thr.iter())
                    .map(|(x, t)| x / t)
                    .fold(0.0_f64, f64::max)
            }
        })
        .collect();

    // Calma con histéresis: no es rasgo, no lleva la marca hidrológica (fue río o lago: pegajosa)
    // y sus variaciones bajan de `h_hist · umbral`.
    let calm: Vec<bool> = (0..num)
        .map(|s| !feature[s] && !world.hydro_lock[s] && !exceeds(&var[s], lod.h_hist))
        .collect();

    let mut may_collapse = vec![false; num];
    for s in 0..num {
        let Some(fam) = family_of(topo, s) else {
            continue;
        };
        if fam.iter().all(|&m| calm[m]) && !mother_would_subdivide(world, cfg, &fam, &ocean, &theta)
        {
            for &m in &fam {
                may_collapse[m] = true;
            }
        }
    }
    Marks {
        want_sub,
        may_collapse,
        verify_collapses: true,
        urgency,
    }
}

/// Anticipación (§8): ¿la madre que saldría del colapso volvería a pedir subdividirse frente a los
/// vecinos externos de la familia? Si sí, no se colapsa (evita el ciclo colapso ↔ subdivisión).
fn mother_would_subdivide(
    world: &World,
    cfg: &TerrenoConfig,
    fam: &[usize; 4],
    ocean: &[bool],
    theta: &[f64],
) -> bool {
    let topo = &world.topo;
    let lod = &cfg.lod;
    let (c, l, i, j) = split_key(topo.keys[fam[0]]);
    let (ml, mi, mj) = (l - 1, i / 2, j / 2);
    let g = world.statgen.geom(c, ml, mi, mj);
    let st = world.statgen.statics(c, ml, mi, mj, &g);

    // Estado anticipado de la madre (como en el colapso): elevación h0 + e medio, T media por área,
    // y θ del agua agregada sobre la capacidad que le daría su suelo medio.
    let (mut t_area, mut area, mut vw, mut e_vol) = (0.0, 0.0, 0.0, 0.0);
    for &m in fam {
        let a = topo.area[m];
        t_area += world.temp[m] * a;
        area += a;
        e_vol += world.e[m] * a;
        if !ocean[m] {
            vw += world.w[m] * a;
        }
    }
    let h_m = st.h + e_vol / g.area;
    let m_ocean = world.statgen.is_ocean_h(h_m);
    let t_m = t_area / area;
    let z_m =
        area_mean(fam.map(|m| (world.z_soil[m], topo.area[m]))).max(erosion::soil_floor(cfg, ml));
    let w_cap_m = st.w_cap * erosion::w_cap_factor(cfg, z_m, ml);
    let theta_m = if w_cap_m > 0.0 {
        (vw / g.area / w_cap_m).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let thr = [lod.dh_sub, lod.dt_sub, lod.dtheta_sub];
    for &m in fam {
        for &b in topo.neighbors(m) {
            let b = b as usize;
            if fam.contains(&b) {
                continue;
            }
            if lod.refine_features && ocean[b] != m_ocean {
                return true; // la madre sería costa
            }
            if !m_ocean && !ocean[b] {
                let v = [
                    (world.h[b] - h_m).abs(),
                    (world.temp[b] - t_m).abs(),
                    (theta[b] - theta_m).abs(),
                ];
                if v.iter().zip(thr.iter()).any(|(x, t)| x > t) {
                    return true;
                }
            }
        }
    }
    false
}

// =================================================================================================
// Paso de topología (§9)
// =================================================================================================

/// Resumen de un paso de topología.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LodReport {
    /// Hojas subdivididas (cada una da 4 hijas).
    pub subdivisions: u64,
    /// Familias colapsadas (cada una da 1 madre).
    pub collapses: u64,
    /// Agua (suelo + nieve) enviada al mar por el reparto (m³): término `lod_al_mar` del balance.
    pub to_ocean_vol: f64,
    /// Biomasa perdida en el mar y creada en tierra nueva (densidad·m²).
    pub veg_lost: f64,
    pub veg_created: f64,
    /// Material que el acondicionamiento hidrológico (incr. 3 §3) no pudo compensar con las vecinas
    /// (m³; normalmente 0). Término propio del balance de material.
    pub fill_m3: f64,
    /// Material movido por el acondicionamiento: lo que subió en las hondonadas rellenadas y bajó en
    /// sus vecinas (m³, informativo; no cambia el balance).
    pub conditioned_m3: f64,
}

impl LodReport {
    /// `true` si la topología cambió.
    pub fn changed(&self) -> bool {
        self.subdivisions + self.collapses > 0
    }

    /// Acumula otro resumen (para varias pasadas seguidas).
    pub fn absorb(&mut self, o: &LodReport) {
        self.subdivisions += o.subdivisions;
        self.collapses += o.collapses;
        self.to_ocean_vol += o.to_ocean_vol;
        self.veg_lost += o.veg_lost;
        self.veg_created += o.veg_created;
        self.fill_m3 += o.fill_m3;
        self.conditioned_m3 += o.conditioned_m3;
    }

    fn add(&mut self, t: &Transfer) {
        self.to_ocean_vol += t.to_ocean_vol;
        self.veg_lost += t.veg_lost;
        self.veg_created += t.veg_created;
    }
}

/// Un paso de topología completo: marcas del driver + aplicación.
pub fn step(world: &mut World, cfg: &TerrenoConfig) -> LodReport {
    let mut m = marks(world, cfg);
    cap_subdivisions(&mut m, cfg.lod.max_sub_frac);
    apply(world, cfg, &m)
}

/// Refinamiento inicial de la génesis (§9): pasos de topología hasta que no haya cambios (como
/// mucho `8·(l_max − l_min) + 8`; los ríos tardan unos pasos en asentarse). Devuelve el resumen
/// acumulado de todas las pasadas.
pub fn refine_initial(world: &mut World, cfg: &TerrenoConfig) -> LodReport {
    let mut total = LodReport::default();
    if !cfg.lod.enabled() {
        return total;
    }
    // Solo criterios estáticos (relieve y rasgos): T y θ aún no tienen dinámica, y el reparto de
    // las pasadas daría humedades artificiales (la tierra nueva nace seca).
    let mut estatico = cfg.clone();
    estatico.lod.dt_sub = f64::INFINITY;
    estatico.lod.dtheta_sub = f64::INFINITY;
    estatico.lod.max_sub_frac = 0.0; // la génesis debe converger: sin tope por revisión
    let max = 8 * (cfg.lod.l_max - cfg.lod.l_min) + 8;
    for _ in 0..max {
        let rep = step(world, &estatico);
        total.absorb(&rep);
        if !rep.changed() {
            break;
        }
    }
    total
}

/// Aplica unas marcas: subdivisiones con cascada 2:1, colapsos con veto 2:1 y reconstrucción.
pub fn apply(world: &mut World, cfg: &TerrenoConfig, mk: &Marks) -> LodReport {
    let topo = &world.topo;
    let num = topo.num_cells;
    let l_max = topo.l_max as u8;

    // 1) Subdivisiones + cascada 2:1: si un par acabaría a ≥2 niveles, se subdivide el más grueso.
    let mut sub: Vec<bool> = (0..num)
        .map(|s| mk.want_sub[s] && topo.level[s] < l_max)
        .collect();
    let mut fin: Vec<u8> = topo
        .level
        .iter()
        .zip(&sub)
        .map(|(&l, &x)| l + x as u8)
        .collect();
    loop {
        let mut changed = false;
        for p in &topo.pairs {
            let (a, b) = (p.a as usize, p.b as usize);
            if fin[a] >= fin[b] + 2 && !sub[b] {
                sub[b] = true;
                fin[b] += 1;
                changed = true;
            } else if fin[b] >= fin[a] + 2 && !sub[a] {
                sub[a] = true;
                fin[a] += 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    // 2) Colapsos: familias permitidas por el driver, sin hijas que se subdividan, y sin ningún
    //    vecino externo que acabe más fino que las hijas (veto 2:1).
    let mut families: Vec<[usize; 4]> = Vec::new();
    for s in 0..num {
        let Some(fam) = family_of(topo, s) else {
            continue;
        };
        if !fam.iter().all(|&m| mk.may_collapse[m] && !sub[m]) {
            continue;
        }
        let lv = topo.level[s];
        let ok = fam.iter().all(|&m| {
            topo.neighbors(m)
                .iter()
                .all(|&b| fam.contains(&(b as usize)) || fin[b as usize] <= lv)
        });
        if ok {
            families.push(fam);
        }
    }

    let any_sub = sub.iter().any(|&x| x);
    if !any_sub && families.is_empty() {
        return LodReport::default();
    }
    if families.is_empty() || !mk.verify_collapses {
        return rebuild(world, cfg, &sub, &families);
    }

    // 3) Ensayo en seco de los colapsos (§8, anticipación exacta): se aplica el paso en una copia,
    //    se recalculan los derivados globales y se vetan las familias cuya madre volvería a pedir
    //    subdividirse (p. ej. porque al juntar las 4 hijas recoge el drenaje de un río). Así no hay
    //    ciclo colapso ↔ subdivisión. Si nada se veta, la copia ES el resultado.
    let mut trial = world.clone();
    let rep = rebuild(&mut trial, cfg, &sub, &families);
    let m2 = marks(&trial, cfg);
    let veto: Vec<bool> = families
        .iter()
        .map(|fam| {
            let k = parent_key(world.topo.keys[fam[0]]);
            trial.topo.slot_of(k).is_some_and(|s| m2.want_sub[s])
        })
        .collect();
    if !veto.iter().any(|&v| v) {
        *world = trial;
        return rep;
    }
    let kept: Vec<[usize; 4]> = families
        .iter()
        .zip(&veto)
        .filter(|(_, v)| !**v)
        .map(|(f, _)| *f)
        .collect();
    if !any_sub && kept.is_empty() {
        return LodReport::default();
    }
    rebuild(world, cfg, &sub, &kept)
}

/// Origen de cada hoja de la malla nueva.
enum Src {
    /// Hoja que no cambia (posición antigua).
    Same(usize),
    /// Hija `idx` de la madre subdividida nº `fam`.
    Child {
        mother: usize,
        idx: usize,
        fam: usize,
    },
    /// Madre de la familia colapsada nº `fam`.
    Parent { fam: usize },
}

/// Columnas de la malla nueva, en construcción.
#[derive(Default)]
struct Cols {
    keys: Vec<u64>,
    level: Vec<u8>,
    lat: Vec<f64>,
    lon: Vec<f64>,
    area: Vec<f64>,
    east: Vec<[f64; 3]>,
    north: Vec<[f64; 3]>,
    dir: Vec<[f64; 3]>,
    h: Vec<f64>,
    slope: Vec<f64>,
    aspect: Vec<f64>,
    grad_e: Vec<f64>,
    grad_n: Vec<f64>,
    w_cap: Vec<f64>,
    k_perm: Vec<f64>,
    fert: Vec<f64>,
    resource: Vec<f64>,
    w: Vec<f64>,
    swe: Vec<f64>,
    veg: Vec<f64>,
    temp: Vec<f64>,
    precip: Vec<f64>,
    temp_sum: Vec<f64>,
    precip_sum: Vec<f64>,
    biome: Vec<Biome>,
    hydro_lock: Vec<bool>,
    h0: Vec<f64>,
    e: Vec<f64>,
    grad0_e: Vec<f64>,
    grad0_n: Vec<f64>,
    w_cap0: Vec<f64>,
    z_soil: Vec<f64>,
    oro0_e: Vec<f64>,
    oro0_n: Vec<f64>,
    oro_e: Vec<f64>,
    oro_n: Vec<f64>,
    oro_air0_e: Vec<f64>,
    w_hum: Vec<f64>,
}

/// Lo que una hoja nueva hereda: clima intensivo `[T, P, ΣT, ΣP]`, marca hidrológica, erosión
/// acumulada `e` (m) y profundidad de suelo `z` (m) (incr. 3 §3 y §7).
struct NewCell {
    clima: [f64; 4],
    lock: bool,
    e: f64,
    z: f64,
}

impl Cols {
    /// Copia íntegra de una hoja que no cambia.
    fn push_same(&mut self, w: &World, s: usize) {
        let t = &w.topo;
        self.keys.push(t.keys[s]);
        self.level.push(t.level[s]);
        self.lat.push(t.lat[s]);
        self.lon.push(t.lon[s]);
        self.area.push(t.area[s]);
        self.east.push(t.east[s]);
        self.north.push(t.north[s]);
        self.dir.push(t.dir[s]);
        self.h.push(w.h[s]);
        self.slope.push(w.slope[s]);
        self.aspect.push(w.aspect[s]);
        self.grad_e.push(w.grad_e[s]);
        self.grad_n.push(w.grad_n[s]);
        self.w_cap.push(w.w_cap[s]);
        self.k_perm.push(w.k_perm[s]);
        self.fert.push(w.fert[s]);
        self.resource.push(w.resource[s]);
        self.w.push(w.w[s]);
        self.swe.push(w.swe[s]);
        self.veg.push(w.veg[s]);
        self.temp.push(w.temp[s]);
        self.precip.push(w.precip[s]);
        self.temp_sum.push(w.temp_sum[s]);
        self.precip_sum.push(w.precip_sum[s]);
        self.biome.push(w.biome[s]);
        self.hydro_lock.push(w.hydro_lock[s]);
        self.h0.push(w.h0[s]);
        self.e.push(w.e[s]);
        self.grad0_e.push(w.grad0_e[s]);
        self.grad0_n.push(w.grad0_n[s]);
        self.w_cap0.push(w.w_cap0[s]);
        self.z_soil.push(w.z_soil[s]);
        self.oro0_e.push(w.oro0_e[s]);
        self.oro0_n.push(w.oro0_n[s]);
        self.oro_e.push(w.oro_e[s]);
        self.oro_n.push(w.oro_n[s]);
        self.oro_air0_e.push(w.oro_air0_e[s]);
        self.w_hum.push(w.w_hum[s]);
    }

    /// Hoja nueva: geometría y estáticos recalculados (función pura); clima, marca, erosión y suelo
    /// heredados; agua, nieve y biomasa a cero (los rellenan los operadores de reparto). El
    /// gradiente dinámico se deja en el puro y se completa con `∇e` tras construir la vecindad.
    fn push_new(&mut self, w: &World, k: u64, g: &CellGeom, nc: NewCell, cfg: &TerrenoConfig) {
        let (c, l, i, j) = split_key(k);
        let st = w.statgen.statics(c, l, i, j, g);
        let NewCell { clima, lock, e, z } = nc;
        self.keys.push(k);
        self.level.push(l as u8);
        self.lat.push(g.lat);
        self.lon.push(g.lon);
        self.area.push(g.area);
        self.east.push(g.east);
        self.north.push(g.north);
        self.dir.push(g.dir);
        self.h.push(st.h + e);
        self.slope.push(st.slope);
        self.aspect.push(st.aspect);
        self.grad_e.push(st.grad_e);
        self.grad_n.push(st.grad_n);
        self.w_cap.push(st.w_cap * erosion::w_cap_factor(cfg, z, l));
        self.k_perm.push(st.k_perm);
        self.fert.push(st.fert);
        self.resource.push(st.resource);
        self.w.push(0.0);
        self.swe.push(0.0);
        self.veg.push(0.0);
        self.temp.push(clima[0]);
        self.precip.push(clima[1]);
        self.temp_sum.push(clima[2]);
        self.precip_sum.push(clima[3]);
        self.biome.push(Biome::Desierto); // se recalcula al final
        self.hydro_lock.push(lock);
        self.h0.push(st.h);
        self.e.push(e);
        self.grad0_e.push(st.grad_e);
        self.grad0_n.push(st.grad_n);
        self.w_cap0.push(st.w_cap);
        self.z_soil.push(z);
        // Gradiente orográfico: parte pura (relieve de referencia, función de la posición); la de
        // la erosión se rehace tras construir la vecindad si el relieve está vivo.
        let (oe, on, oa) = w.statgen.oro_gradient0_air(g, l);
        self.oro0_e.push(oe);
        self.oro0_n.push(on);
        self.oro_e.push(oe);
        self.oro_n.push(on);
        // Relieve que ve el aire (mar plano): a la escala de referencia o a la de la celda.
        self.oro_air0_e.push(if w.statgen.uses_oro_reference(l) {
            oa
        } else {
            st.grad_air_e
        });
        // Agotamiento de humedad: provisional; se rehace tras construir la malla si está activo.
        self.w_hum.push(1.0);
    }

    fn set_state(&mut self, ns: usize, st: WaterState) {
        self.w[ns] = st.w;
        self.swe[ns] = st.swe;
        self.veg[ns] = st.veg;
    }
}

/// Baja por igual a las `donors` (sin pasar de `floor`) hasta retirar `vol` m³, o todo lo que se
/// pueda; devuelve el volumen retirado. Como el desplazamiento es el mismo para todas (salvo las que
/// tocan el suelo), su orden de alturas no cambia.
fn lower_to_floor(world: &mut World, donors: &[usize], floor: f64, vol: f64) -> f64 {
    if donors.is_empty() || vol <= 0.0 {
        return 0.0;
    }
    // (holgura sobre el suelo, área, posición), de menor a mayor holgura.
    let mut d: Vec<(f64, f64, usize)> = donors
        .iter()
        .map(|&j| (world.h[j] - floor, world.topo.area[j], j))
        .collect();
    d.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.2.cmp(&y.2)));
    // Desplazamiento δ tal que Σ a_j·min(δ, holgura_j) = vol (o la holgura máxima si no alcanza).
    let mut delta = d.iter().fold(0.0_f64, |m, x| m.max(x.0));
    let mut a_active: f64 = d.iter().map(|x| x.1).sum();
    let (mut prev, mut acc) = (0.0, 0.0);
    for &(room, a, _) in &d {
        let seg = a_active * (room - prev);
        if acc + seg >= vol {
            delta = prev + (vol - acc) / a_active;
            break;
        }
        acc += seg;
        prev = room;
        a_active -= a;
    }
    let mut removed = 0.0;
    for &(room, a, j) in &d {
        let dj = delta.min(room);
        if dj > 0.0 {
            world.h[j] -= dj;
            world.e[j] -= dj;
            removed += dj * a;
        }
    }
    removed
}

/// Media ponderada por área de un intensivo; si todos los valores coinciden, ese valor exacto (así
/// el suelo intacto sigue siendo `z_init` bit a bit tras un colapso).
fn area_mean(vals: [(f64, f64); 4]) -> f64 {
    let first = vals[0].0;
    if vals.iter().all(|&(v, _)| v == first) {
        return first;
    }
    let (num, den) = vals
        .iter()
        .fold((0.0, 0.0), |(n, d), &(v, a)| (n + v * a, d + a));
    num / den
}

/// Clima intensivo de una hoja antigua: `[T, P, ΣT, ΣP]`.
fn clima_of(w: &World, s: usize) -> [f64; 4] {
    [w.temp[s], w.precip[s], w.temp_sum[s], w.precip_sum[s]]
}

/// Estado repartible de una hoja antigua.
fn cell_in(w: &World, s: usize) -> CellIn {
    CellIn {
        state: WaterState {
            w: w.w[s],
            swe: w.swe[s],
            veg: w.veg[s],
        },
        area: w.topo.area[s],
        ocean: w.es_oceano(s),
    }
}

/// Reconstruye la malla: nueva lista de claves ordenada, columnas (copiando o recalculando),
/// reparto del estado, vecindad y derivados globales.
fn rebuild(
    world: &mut World,
    cfg: &TerrenoConfig,
    sub: &[bool],
    families: &[[usize; 4]],
) -> LodReport {
    let old: &World = world;
    let num = old.topo.num_cells;

    let mut fam_of = vec![usize::MAX; num];
    for (f, fam) in families.iter().enumerate() {
        for &m in fam {
            fam_of[m] = f;
        }
    }

    // Nueva lista de hojas (con su origen), ordenada por clave.
    let n_sub = sub.iter().filter(|&&x| x).count();
    let mut entries: Vec<(u64, Src)> = Vec::with_capacity(num + 3 * n_sub);
    let mut mothers: Vec<usize> = Vec::with_capacity(n_sub);
    for s in 0..num {
        let k = old.topo.keys[s];
        if sub[s] {
            let fam = mothers.len();
            mothers.push(s);
            for (idx, &(di, dj)) in CHILDREN.iter().enumerate() {
                entries.push((
                    child_key(k, di, dj),
                    Src::Child {
                        mother: s,
                        idx,
                        fam,
                    },
                ));
            }
        } else if fam_of[s] != usize::MAX {
            if families[fam_of[s]][0] == s {
                entries.push((parent_key(k), Src::Parent { fam: fam_of[s] }));
            }
        } else {
            entries.push((k, Src::Same(s)));
        }
    }
    entries.sort_unstable_by_key(|e| e.0);
    let n_new = entries.len();

    // Columnas nuevas: copia de lo que no cambia; recálculo (función pura) de lo nuevo.
    let mut cols = Cols::default();
    let mut child_slots = vec![[0usize; 4]; mothers.len()];
    let mut parent_slot = vec![0usize; families.len()];
    let mut is_new = vec![false; n_new];
    // ¿Estaba en depresión el origen de cada hoja nueva? (para el acondicionamiento, incr. 3 §3)
    let in_dep_old = |s: usize| !old.es_oceano(s) && old.h[s] < old.filled_h[s];
    let mut origin_dep = vec![false; n_new];
    for (ns, (k, src)) in entries.iter().enumerate() {
        match *src {
            Src::Same(s) => {
                cols.push_same(old, s);
                origin_dep[ns] = in_dep_old(s);
            }
            Src::Child { mother, idx, fam } => {
                // Las hijas heredan la marca hidrológica, la erosión acumulada y el suelo de la
                // madre (intensivos: copia, incr. 3 §3).
                let (c, l, i, j) = split_key(*k);
                let g = old.statgen.geom(c, l, i, j);
                let nc = NewCell {
                    clima: clima_of(old, mother),
                    lock: old.hydro_lock[mother],
                    e: old.e[mother],
                    z: old.z_soil[mother],
                };
                cols.push_new(old, *k, &g, nc, cfg);
                child_slots[fam][idx] = ns;
                is_new[ns] = true;
                origin_dep[ns] = in_dep_old(mother);
            }
            Src::Parent { fam } => {
                // Intensivos climáticos: media ponderada por área de las 4 hermanas.
                let mut acc = [0.0; 4];
                let mut a = 0.0;
                for &m in &families[fam] {
                    let ar = old.topo.area[m];
                    for (x, v) in acc.iter_mut().zip(clima_of(old, m)) {
                        *x += v * ar;
                    }
                    a += ar;
                }
                // Erosión: media por área que conserva el volumen `Σ e·a` exactamente; suelo:
                // media por área (si las 4 hermanas coinciden, ese valor exacto). Solo se colapsan
                // familias sin marca: la madre nace sin ella.
                let (c, l, i, j) = split_key(*k);
                let g = old.statgen.geom(c, l, i, j);
                let members = &families[fam];
                let e_vol: f64 = members.iter().map(|&m| old.e[m] * old.topo.area[m]).sum();
                let nc = NewCell {
                    clima: acc.map(|x| x / a),
                    lock: false,
                    e: e_vol / g.area,
                    z: area_mean(members.map(|m| (old.z_soil[m], old.topo.area[m])))
                        .max(erosion::soil_floor(cfg, l)),
                };
                cols.push_new(old, *k, &g, nc, cfg);
                parent_slot[fam] = ns;
                is_new[ns] = true;
                origin_dep[ns] = members.iter().any(|&m| in_dep_old(m));
            }
        }
    }

    // Reparto conservativo del estado (§6).
    let v_init = cfg.vegetation.v_init;
    let mut rep = LodReport {
        subdivisions: mothers.len() as u64,
        collapses: families.len() as u64,
        ..LodReport::default()
    };
    for (f, &m) in mothers.iter().enumerate() {
        let slots = child_slots[f];
        let kids = slots.map(|ns| CellCap {
            area: cols.area[ns],
            w_cap: cols.w_cap[ns],
            ocean: old.statgen.is_ocean_h(cols.h[ns]),
        });
        let (out, tr) = split_state(&cell_in(old, m), &kids, v_init);
        for (st, &ns) in out.iter().zip(&slots) {
            cols.set_state(ns, *st);
        }
        rep.add(&tr);
    }
    for (f, fam) in families.iter().enumerate() {
        let ns = parent_slot[f];
        let mother = CellCap {
            area: cols.area[ns],
            w_cap: cols.w_cap[ns],
            ocean: old.statgen.is_ocean_h(cols.h[ns]),
        };
        let (st, tr) = merge_state(&fam.map(|m| cell_in(old, m)), &mother, v_init);
        cols.set_state(ns, st);
        rep.add(&tr);
    }

    // Topología nueva + vecindad.
    let mut topo = Topology {
        n0: old.topo.n0,
        l_min: old.topo.l_min,
        l_max: old.topo.l_max,
        num_cells: n_new,
        r_planet: old.topo.r_planet,
        keys: cols.keys,
        level: cols.level,
        lat: cols.lat,
        lon: cols.lon,
        area: cols.area,
        east: cols.east,
        north: cols.north,
        dir: cols.dir,
        nb_off: Vec::new(),
        nb: Vec::new(),
        pairs: Vec::new(),
    };
    build_neighbors(&mut topo);

    // Instalar las columnas en el mundo.
    world.topo = topo;
    world.h = cols.h;
    world.slope = cols.slope;
    world.aspect = cols.aspect;
    world.grad_e = cols.grad_e;
    world.grad_n = cols.grad_n;
    world.w_cap = cols.w_cap;
    world.k_perm = cols.k_perm;
    world.fert = cols.fert;
    world.resource = cols.resource;
    world.w = cols.w;
    world.swe = cols.swe;
    world.veg = cols.veg;
    world.temp = cols.temp;
    world.precip = cols.precip;
    world.temp_sum = cols.temp_sum;
    world.precip_sum = cols.precip_sum;
    world.biome = cols.biome;
    world.hydro_lock = cols.hydro_lock;
    world.h0 = cols.h0;
    world.e = cols.e;
    world.grad0_e = cols.grad0_e;
    world.grad0_n = cols.grad0_n;
    world.w_cap0 = cols.w_cap0;
    world.z_soil = cols.z_soil;
    world.oro0_e = cols.oro0_e;
    world.oro0_n = cols.oro0_n;
    world.oro_e = cols.oro_e;
    world.oro_n = cols.oro_n;
    world.oro_air0_e = cols.oro_air0_e;
    world.w_hum = cols.w_hum;

    // Derivados globales sobre la malla nueva (§4), marca hidrológica y biomas de las hojas nuevas.
    hydro::recompute(world, cfg);
    // Acondicionamiento hidrológico (incr. 3 §3). Una vez que el relieve ha sido erosionado, el
    // detalle fino de una hoja nueva no lo está y sus hondonadas son un artefacto de bajar de escala
    // (y un colapso puede dejar encerrada a una vecina). Regla: un cambio de malla no puede crear
    // hondonadas; una hoja solo queda en depresión si su origen lo estaba. Cada grupo conexo de
    // hondonadas nuevas se rellena hacia su cota de derrame, y ese material se toma de su borde (las
    // vecinas de tierra por encima de la cota, bajadas por igual sin pasar de ella): se conserva
    // exactamente y no aparecen hondonadas nuevas, porque toda vecina rebajada sigue tocando al grupo
    // rellenado. Si el borde no alcanza, el grupo se rellena solo en la proporción que el borde puede
    // pagar (queda como un lago poco hondo): nunca se crea material. `fill_m3` recoge solo el redondeo.
    // Antes de erosionar (relieve en bruto) no se toca nada: esas hondonadas las rellena el propio
    // sedimento de la génesis. Sin relieve vivo, tampoco (incremento 2).
    if cfg.erosion.enabled() && world.e.iter().any(|&x| x != 0.0) {
        let mut is_target = vec![false; n_new];
        let mut any = false;
        for (s, &dep_antes) in origin_dep.iter().enumerate() {
            if !dep_antes && !world.es_oceano(s) && world.h[s] < world.filled_h[s] {
                is_target[s] = true;
                any = true;
            }
        }
        if any {
            rellenar_grupos(world, &is_target, &mut rep);
            hydro::recompute_after_erosion(world, cfg);
        }
    }
    update_hydro_lock(world);
    // Con relieve vivo, la pendiente incluye `∇e`, que depende de los vecinos nuevos (incr. 3 §3).
    // Sin erosión (`e ≡ 0`) no se toca nada: el resultado es el del incremento 2, bit a bit.
    if world.e.iter().any(|&x| x != 0.0) {
        erosion::refresh_gradients(world);
        erosion::refresh_orography(world);
    }
    // Agotamiento de humedad (nota de sombra pluviométrica §3): las hojas nuevas toman su `w` de
    // los anillos, y el acondicionamiento puede haber cambiado el relieve de referencia.
    crate::humedad::refrescar(world, cfg);
    for (s, &nuevo) in is_new.iter().enumerate() {
        if nuevo {
            world.biome[s] = biome_for(world, cfg, s);
        }
    }
    rep
}

/// Rellena hacia su cota de derrame cada grupo conexo de celdas `objetivo` en depresión, con el
/// material de su borde (las vecinas de tierra por encima de la cota, bajadas por igual sin pasar de
/// ella): se conserva exactamente y no aparecen hondonadas nuevas. Si el borde no alcanza, el grupo
/// se rellena en proporción. Necesita `filled_h` al día; no recalcula los derivados.
fn rellenar_grupos(world: &mut World, is_target: &[bool], rep: &mut LodReport) {
    let n_new = world.topo.num_cells;
    let mut seen = vec![false; n_new];
    for start in 0..n_new {
        if !is_target[start] || seen[start] {
            continue;
        }
        // Grupo conexo de hondonadas (en orden de posición: determinista).
        let mut group = vec![start];
        seen[start] = true;
        let mut k = 0;
        while k < group.len() {
            let c = group[k];
            k += 1;
            for &b in world.topo.neighbors(c) {
                let b = b as usize;
                if is_target[b] && !seen[b] {
                    seen[b] = true;
                    group.push(b);
                }
            }
        }
        group.sort_unstable();
        let floor = group
            .iter()
            .fold(f64::NEG_INFINITY, |m, &c| m.max(world.filled_h[c]));
        let mut donors: Vec<usize> = group
            .iter()
            .flat_map(|&c| world.topo.neighbors(c).iter().map(|&b| b as usize))
            .filter(|&b| !is_target[b] && !world.es_oceano(b) && world.h[b] > floor)
            .collect();
        donors.sort_unstable();
        donors.dedup();
        // Lo que pide el grupo y lo que puede pagar su borde.
        let need: f64 = group
            .iter()
            .map(|&c| (world.filled_h[c] - world.h[c]).max(0.0) * world.topo.area[c])
            .sum();
        let room: f64 = donors
            .iter()
            .map(|&j| (world.h[j] - floor) * world.topo.area[j])
            .sum();
        if need <= 0.0 || room <= 0.0 {
            continue;
        }
        let scale = (room / need).min(1.0);
        let mut raised = 0.0;
        for &c in &group {
            let d = (world.filled_h[c] - world.h[c]).max(0.0) * scale;
            if d > 0.0 {
                world.h[c] += d;
                world.e[c] += d;
                raised += d * world.topo.area[c];
            }
        }
        let removed = lower_to_floor(world, &donors, floor, raised);
        rep.conditioned_m3 += raised;
        rep.fill_m3 += raised - removed;
    }
}

/// Acondicionamiento hidrológico del relieve tectónico (G6 del diseño): el ruido de detalle tiene la
/// rugosidad del relieve real pero no su red de drenaje, y deja hoyos que un millón de años de
/// erosión no llega a rellenar. Se rellenan (con el material de su borde, balance exacto) las
/// hondonadas que no existen en la base tectónica sin ruido; las cuencas de la tectónica (rifts,
/// depresiones entre cordilleras) se conservan. Varias pasadas, porque rellenar un grupo puede
/// unirlo a otro.
pub fn acondicionar_detalle(world: &mut World, cfg: &TerrenoConfig) -> LodReport {
    let mut rep = LodReport::default();
    let n = world.topo.num_cells;
    let base: Vec<f64> = (0..n)
        .map(|s| {
            world
                .statgen
                .h_base_at(world.topo.dir[s])
                .unwrap_or(world.h[s])
        })
        .collect();
    // Hondonadas de la base: priority-flood sobre la base con la misma malla.
    let filled_full = world.filled_h.clone();
    let h_full = world.h.clone();
    world.h.copy_from_slice(&base);
    hydro::recompute_after_erosion(world, cfg);
    let base_en_hoyo: Vec<bool> = (0..n)
        .map(|s| !world.es_oceano(s) && base[s] < world.filled_h[s])
        .collect();
    world.h.copy_from_slice(&h_full);
    world.filled_h.copy_from_slice(&filled_full);
    hydro::recompute_after_erosion(world, cfg);
    for _ in 0..4 {
        let target: Vec<bool> = (0..n)
            .map(|s| !base_en_hoyo[s] && !world.es_oceano(s) && world.h[s] < world.filled_h[s])
            .collect();
        if !target.iter().any(|&x| x) {
            break;
        }
        rellenar_grupos(world, &target, &mut rep);
        hydro::recompute_after_erosion(world, cfg);
    }
    if world.e.iter().any(|&x| x != 0.0) {
        erosion::refresh_gradients(world);
        erosion::refresh_orography(world);
    }
    update_hydro_lock(world);
    rep
}

/// Marca hidrológica pegajosa (§8): toda hoja que es río o lago queda marcada para siempre (sus
/// hijas la heredan) y una familia con alguna hija marcada no se colapsa. Los ríos dependen de la
/// red de drenaje de toda la malla, y la malla de los ríos: sin esta memoria, ese acoplamiento
/// produce un vaivén sin fin (un colapso desvía un río, que obliga a subdividir en otra parte...).
/// Con el relieve estático el conjunto marcado solo crece y está acotado => la malla converge.
pub fn update_hydro_lock(world: &mut World) {
    for (lock, surf) in world.hydro_lock.iter_mut().zip(&world.surface) {
        *lock |= matches!(surf, Surface::Rio | Surface::Lago);
    }
}

/// Bioma de una hoja nueva: océano, o Whittaker de sus acumuladores (desierto antes de la 1ª media).
fn biome_for(world: &World, cfg: &TerrenoConfig, s: usize) -> Biome {
    if world.es_oceano(s) {
        Biome::Oceano
    } else if world.sample_count == 0 {
        Biome::Desierto
    } else {
        let n = world.sample_count as f64;
        whittaker(
            world.temp_sum[s] / n,
            world.precip_sum[s] / n * cfg.climate.t_year,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generador congruencial mínimo para los tests (determinista, sin dependencias).
    struct Lcg(u64);
    impl Lcg {
        fn next(&mut self) -> f64 {
            self.0 = self
                .0
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    fn kids_random(rng: &mut Lcg, area_madre: f64, ocean: [bool; 4]) -> [CellCap; 4] {
        let mut a = [0.0; 4];
        for x in a.iter_mut() {
            *x = 0.5 + rng.next();
        }
        let s: f64 = a.iter().sum();
        let mut out = [CellCap {
            area: 0.0,
            w_cap: 0.0,
            ocean: false,
        }; 4];
        for (k, o) in out.iter_mut().enumerate() {
            *o = CellCap {
                area: a[k] / s * area_madre,
                w_cap: 50.0 + 250.0 * rng.next(),
                ocean: ocean[k],
            };
        }
        out
    }

    fn vol(st: &WaterState, a: f64) -> f64 {
        (st.w + st.swe) / 1000.0 * a
    }

    #[test]
    fn ida_y_vuelta_es_identidad() {
        // Madre de tierra con hijas de tierra y capacidad suficiente: subdividir y colapsar devuelve
        // exactamente el estado (§6.3), a épsilon de máquina.
        let mut rng = Lcg(7);
        for _ in 0..500 {
            let area = 1e9 * (1.0 + rng.next());
            let kids = kids_random(&mut rng, area, [false; 4]);
            let cap: f64 = kids.iter().map(|k| k.w_cap * k.area).sum::<f64>() / area;
            let madre = CellIn {
                state: WaterState {
                    w: cap * rng.next(),
                    swe: 80.0 * rng.next(),
                    veg: 100.0 * rng.next(),
                },
                area,
                ocean: false,
            };
            let (hijas, tr) = split_state(&madre, &kids, 1.0);
            assert_eq!(tr, Transfer::default());
            let vuelta: [CellIn; 4] = std::array::from_fn(|k| CellIn {
                state: hijas[k],
                area: kids[k].area,
                ocean: false,
            });
            let cap_madre = CellCap {
                area,
                w_cap: 1e9, // sin tope: la identidad no depende de la capacidad de la madre
                ocean: false,
            };
            let (m2, tr2) = merge_state(&vuelta, &cap_madre, 1.0);
            assert_eq!(tr2, Transfer::default());
            for (x, y) in [
                (madre.state.w, m2.w),
                (madre.state.swe, m2.swe),
                (madre.state.veg, m2.veg),
            ] {
                assert!((x - y).abs() <= 1e-12 * x.abs().max(1.0), "{x} != {y}");
            }
        }
    }

    #[test]
    fn reparto_conserva_con_mar_y_exceso() {
        // Con hijas de mar y exceso de capacidad, el agua no se crea ni se destruye: lo que no queda
        // en tierra está en `to_ocean_vol`.
        let mut rng = Lcg(11);
        for caso in 0..400 {
            let area = 1e9;
            let ocean = [caso % 2 == 0, caso % 3 == 0, caso % 5 == 0, caso % 7 == 0];
            let kids = kids_random(&mut rng, area, ocean);
            let madre = CellIn {
                state: WaterState {
                    w: 400.0 * rng.next(), // a veces por encima de la capacidad de las hijas
                    swe: 50.0 * rng.next(),
                    veg: 10.0,
                },
                area,
                ocean: false,
            };
            let (hijas, tr) = split_state(&madre, &kids, 1.0);
            let despues: f64 = hijas
                .iter()
                .zip(&kids)
                .filter(|(_, k)| !k.ocean)
                .map(|(h, k)| vol(h, k.area))
                .sum();
            let antes = vol(&madre.state, area);
            assert!(
                (antes - despues - tr.to_ocean_vol).abs() <= 1e-9 * antes.max(1.0),
                "subdivisión no conserva: {antes} vs {despues} + {}",
                tr.to_ocean_vol
            );
            for (h, k) in hijas.iter().zip(&kids) {
                if k.ocean {
                    assert_eq!(*h, WaterState::ZERO, "una hija de mar guarda estado");
                } else {
                    assert!(h.w <= k.w_cap * (1.0 + 1e-12), "agua sobre la capacidad");
                }
            }
            // Colapso hacia una madre con poca capacidad: el exceso va al mar.
            let vuelta: [CellIn; 4] = std::array::from_fn(|k| CellIn {
                state: hijas[k],
                area: kids[k].area,
                ocean: kids[k].ocean,
            });
            let m = CellCap {
                area,
                w_cap: 60.0,
                ocean: false,
            };
            let (m2, tr2) = merge_state(&vuelta, &m, 1.0);
            assert!(m2.w <= 60.0);
            let fin = vol(&m2, area);
            assert!(
                (despues - fin - tr2.to_ocean_vol).abs() <= 1e-9 * despues.max(1.0),
                "colapso no conserva"
            );
        }
    }

    #[test]
    fn reparto_con_capacidad_cero_conserva_nieve_y_biomasa() {
        // Hijas de tierra sin capacidad para agua del suelo (roca desnuda, incr. 3 §7): la nieve y
        // la biomasa siguen en tierra; solo el agua del suelo va al mar.
        let kids = [CellCap {
            area: 1.0,
            w_cap: 0.0,
            ocean: false,
        }; 4];
        let madre = CellIn {
            state: WaterState {
                w: 100.0,
                swe: 40.0,
                veg: 7.0,
            },
            area: 4.0,
            ocean: false,
        };
        let (h, tr) = split_state(&madre, &kids, 1.0);
        for x in h {
            assert_eq!(
                x,
                WaterState {
                    w: 0.0,
                    swe: 40.0,
                    veg: 7.0
                }
            );
        }
        assert!((tr.to_ocean_vol - 100.0 / 1000.0 * 4.0).abs() < 1e-15);
        assert_eq!(tr.veg_lost, 0.0);
    }

    #[test]
    fn madre_de_mar_crea_tierra_seca() {
        let kids = [
            CellCap {
                area: 1.0,
                w_cap: 100.0,
                ocean: true,
            },
            CellCap {
                area: 1.0,
                w_cap: 100.0,
                ocean: false,
            },
            CellCap {
                area: 1.0,
                w_cap: 100.0,
                ocean: true,
            },
            CellCap {
                area: 1.0,
                w_cap: 100.0,
                ocean: true,
            },
        ];
        let madre = CellIn {
            state: WaterState::ZERO,
            area: 4.0,
            ocean: true,
        };
        let (h, tr) = split_state(&madre, &kids, 2.5);
        assert_eq!(
            h[1],
            WaterState {
                w: 0.0,
                swe: 0.0,
                veg: 2.5
            }
        );
        assert_eq!(tr.to_ocean_vol, 0.0);
        assert_eq!(tr.veg_created, 2.5);
    }
}
