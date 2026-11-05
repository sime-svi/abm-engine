//! Caché de la génesis (incremento 3, revisión del 2026-11-05).
//!
//! La génesis (refinado inicial del LOD + un millón de años de erosión) depende solo de la semilla,
//! de la config y del **código**. Se calcula una vez, se guarda en disco y se reutiliza: evita unos
//! 25 s por ejecución. La clave del fichero combina la huella del código del terreno (calculada por
//! `build.rs`: cambia con cualquier fuente de `src/terreno/`, el `Cargo.lock` o el toolchain) con la
//! config entera, salvo los campos que solo afectan a la corrida (nombre, ticks, salida, caché).
//!
//! El mundo se guarda en binario crudo (little-endian, columna a columna): al cargarlo se recupera
//! **bit a bit**, así que la salida es idéntica se use o no la caché (CLAUDE §6). El generador de
//! estáticos no se guarda: es función pura de la config y se reconstruye. Un fichero dañado, a medio
//! escribir o de otra versión se ignora y se recalcula.

use std::path::PathBuf;

use crate::config::{PersistenceCfg, TerrenoConfig};
use crate::erosion::GeoSummary;
use crate::error::TerrenoError;
use crate::statics::StaticGen;
use crate::world::{Biome, Pair, Surface, Topology, World};

const MAGIC: &[u8; 8] = b"ABMGEN03";
const FINGERPRINT: &str = env!("ABM_TERRENO_FINGERPRINT");

fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// Clave de la génesis de una config: huella del código + config sin los campos de la corrida.
pub fn key(cfg: &TerrenoConfig) -> Result<u64, TerrenoError> {
    let mut c = cfg.clone();
    c.run.name = String::new();
    c.run.total_ticks = 0;
    c.run.output_path = String::new();
    c.run.dump_world = false;
    c.run.dump_climate = false;
    c.run.genesis_cache = String::new();
    c.persistence = PersistenceCfg::default();
    let json = serde_json::to_string(&c)?;
    let h = fnv1a(0xcbf2_9ce4_8422_2325, FINGERPRINT.as_bytes());
    Ok(fnv1a(h, json.as_bytes()))
}

/// Ruta del fichero de caché de una config (`None` si la caché está desactivada).
pub fn path(cfg: &TerrenoConfig) -> Result<Option<PathBuf>, TerrenoError> {
    if cfg.run.genesis_cache.is_empty() {
        return Ok(None);
    }
    let name = format!("genesis_{:016x}.bin", key(cfg)?);
    Ok(Some(PathBuf::from(&cfg.run.genesis_cache).join(name)))
}

/// Carga la génesis de la caché, si existe y es válida para esta config y este código.
pub fn load(cfg: &TerrenoConfig) -> Result<Option<(World, GeoSummary)>, TerrenoError> {
    let Some(p) = path(cfg)? else {
        return Ok(None);
    };
    let Ok(bytes) = std::fs::read(&p) else {
        return Ok(None);
    };
    Ok(decode(&bytes, cfg, key(cfg)?))
}

/// Guarda la génesis en la caché (escritura atómica: a un temporal y luego renombrado).
pub fn save(cfg: &TerrenoConfig, world: &World, geo: &GeoSummary) -> Result<(), TerrenoError> {
    let Some(p) = path(cfg)? else {
        return Ok(());
    };
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = encode(world, geo, key(cfg)?)?;
    let tmp = p.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, &p)?;
    Ok(())
}

// --- Codificación -------------------------------------------------------------------------------

#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn u64(&mut self, x: u64) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn u32(&mut self, x: u32) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn f64(&mut self, x: f64) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn vf64(&mut self, v: &[f64]) {
        self.u64(v.len() as u64);
        v.iter().for_each(|&x| self.f64(x));
    }
    fn vu64(&mut self, v: &[u64]) {
        self.u64(v.len() as u64);
        v.iter().for_each(|&x| self.u64(x));
    }
    fn vu32(&mut self, v: &[u32]) {
        self.u64(v.len() as u64);
        v.iter().for_each(|&x| self.u32(x));
    }
    fn vi64(&mut self, v: &[i64]) {
        self.u64(v.len() as u64);
        v.iter()
            .for_each(|&x| self.0.extend_from_slice(&x.to_le_bytes()));
    }
    fn vu8(&mut self, v: &[u8]) {
        self.u64(v.len() as u64);
        self.0.extend_from_slice(v);
    }
    fn vv3(&mut self, v: &[[f64; 3]]) {
        self.u64(v.len() as u64);
        v.iter().flatten().for_each(|&x| self.f64(x));
    }
}

struct Reader<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.p.checked_add(n)?;
        let s = self.b.get(self.p..end)?;
        self.p = end;
        Some(s)
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn len(&mut self) -> Option<usize> {
        usize::try_from(self.u64()?).ok()
    }
    fn vf64(&mut self) -> Option<Vec<f64>> {
        let n = self.len()?;
        let raw = self.take(n.checked_mul(8)?)?;
        raw.chunks_exact(8)
            .map(|c| c.try_into().ok().map(f64::from_le_bytes))
            .collect()
    }
    fn vu64(&mut self) -> Option<Vec<u64>> {
        let n = self.len()?;
        let raw = self.take(n.checked_mul(8)?)?;
        raw.chunks_exact(8)
            .map(|c| c.try_into().ok().map(u64::from_le_bytes))
            .collect()
    }
    fn vu32(&mut self) -> Option<Vec<u32>> {
        let n = self.len()?;
        let raw = self.take(n.checked_mul(4)?)?;
        raw.chunks_exact(4)
            .map(|c| c.try_into().ok().map(u32::from_le_bytes))
            .collect()
    }
    fn vi64(&mut self) -> Option<Vec<i64>> {
        let n = self.len()?;
        let raw = self.take(n.checked_mul(8)?)?;
        raw.chunks_exact(8)
            .map(|c| c.try_into().ok().map(i64::from_le_bytes))
            .collect()
    }
    fn vu8(&mut self) -> Option<Vec<u8>> {
        let n = self.len()?;
        Some(self.take(n)?.to_vec())
    }
    fn vv3(&mut self) -> Option<Vec<[f64; 3]>> {
        let n = self.len()?;
        let raw = self.take(n.checked_mul(24)?)?;
        raw.chunks_exact(24)
            .map(|c| {
                let x = f64::from_le_bytes(c.get(0..8)?.try_into().ok()?);
                let y = f64::from_le_bytes(c.get(8..16)?.try_into().ok()?);
                let z = f64::from_le_bytes(c.get(16..24)?.try_into().ok()?);
                Some([x, y, z])
            })
            .collect()
    }
}

fn surface_code(s: Surface) -> u8 {
    match s {
        Surface::Tierra => 0,
        Surface::Oceano => 1,
        Surface::Lago => 2,
        Surface::Rio => 3,
    }
}

fn surface_from(c: u8) -> Option<Surface> {
    Some(match c {
        0 => Surface::Tierra,
        1 => Surface::Oceano,
        2 => Surface::Lago,
        3 => Surface::Rio,
        _ => return None,
    })
}

/// Nº de columnas `f64` del mundo que se guardan.
const N_F64: usize = 31;

/// Columnas `f64` del mundo, en orden fijo (el mismo al guardar y al cargar).
fn f64_columns(w: &World) -> [&Vec<f64>; N_F64] {
    [
        &w.h0,
        &w.grad0_e,
        &w.grad0_n,
        &w.w_cap0,
        &w.k_perm,
        &w.fert,
        &w.resource,
        &w.e,
        &w.z_soil,
        &w.h,
        &w.slope,
        &w.aspect,
        &w.grad_e,
        &w.grad_n,
        &w.w_cap,
        &w.filled_h,
        &w.drainage_area,
        &w.dist_ocean_km,
        &w.w,
        &w.temp,
        &w.precip,
        &w.swe,
        &w.veg,
        &w.temp_sum,
        &w.precip_sum,
        &w.oro0_e,
        &w.oro0_n,
        &w.oro_e,
        &w.oro_n,
        &w.oro_air0_e,
        &w.w_hum,
    ]
}

fn encode(world: &World, geo: &GeoSummary, key: u64) -> Result<Vec<u8>, TerrenoError> {
    let mut w = Writer::default();
    w.0.extend_from_slice(MAGIC);
    w.u64(key);
    w.vu8(&serde_json::to_vec(geo)?);
    let t = &world.topo;
    w.u32(t.n0);
    w.u32(t.l_min);
    w.u32(t.l_max);
    w.u64(t.num_cells as u64);
    w.f64(t.r_planet);
    w.vu64(&t.keys);
    w.vu8(&t.level);
    w.vf64(&t.lat);
    w.vf64(&t.lon);
    w.vf64(&t.area);
    w.vv3(&t.east);
    w.vv3(&t.north);
    w.vv3(&t.dir);
    w.vu32(&t.nb_off);
    w.vu32(&t.nb);
    w.vu32(
        &t.pairs
            .iter()
            .flat_map(|p| [p.a, p.b])
            .collect::<Vec<u32>>(),
    );
    w.vf64(&t.pairs.iter().map(|p| p.g).collect::<Vec<f64>>());
    for col in f64_columns(world) {
        w.vf64(col);
    }
    w.vi64(&world.receiver);
    w.vu32(&world.flow_order);
    w.vu8(
        &world
            .surface
            .iter()
            .map(|&s| surface_code(s))
            .collect::<Vec<u8>>(),
    );
    w.vu8(
        &world
            .biome
            .iter()
            .map(|b| b.index() as u8)
            .collect::<Vec<u8>>(),
    );
    w.vu8(
        &world
            .hydro_lock
            .iter()
            .map(|&x| x as u8)
            .collect::<Vec<u8>>(),
    );
    w.u64(world.sample_count);
    w.f64(world.c_p);
    w.f64(world.desvio_lluvia);
    Ok(w.0)
}

fn decode(bytes: &[u8], cfg: &TerrenoConfig, key: u64) -> Option<(World, GeoSummary)> {
    let mut r = Reader { b: bytes, p: 0 };
    if r.take(8)? != MAGIC || r.u64()? != key {
        return None;
    }
    let geo: GeoSummary = serde_json::from_slice(&r.vu8()?).ok()?;
    let (n0, l_min, l_max) = (r.u32()?, r.u32()?, r.u32()?);
    let num = r.len()?;
    let r_planet = r.f64()?;
    let keys = r.vu64()?;
    let level = r.vu8()?;
    let (lat, lon, area) = (r.vf64()?, r.vf64()?, r.vf64()?);
    let (east, north, dir) = (r.vv3()?, r.vv3()?, r.vv3()?);
    let (nb_off, nb) = (r.vu32()?, r.vu32()?);
    let ab = r.vu32()?;
    let g = r.vf64()?;
    if ab.len() != 2 * g.len() {
        return None;
    }
    let pairs = ab
        .chunks_exact(2)
        .zip(&g)
        .map(|(c, &g)| Pair {
            a: c[0],
            b: c[1],
            g,
        })
        .collect();
    let mut cols: Vec<Vec<f64>> = Vec::with_capacity(N_F64);
    for _ in 0..N_F64 {
        cols.push(r.vf64()?);
    }
    let receiver = r.vi64()?;
    let flow_order = r.vu32()?;
    let surface: Vec<Surface> = r
        .vu8()?
        .into_iter()
        .map(surface_from)
        .collect::<Option<_>>()?;
    let biome: Vec<Biome> = r
        .vu8()?
        .into_iter()
        .map(|i| Biome::ALL.get(i as usize).copied())
        .collect::<Option<_>>()?;
    let hydro_lock: Vec<bool> = r.vu8()?.into_iter().map(|x| x != 0).collect();
    let sample_count = r.u64()?;
    let (c_p, desvio_lluvia) = (r.f64()?, r.f64()?);
    if r.p != bytes.len() {
        return None;
    }

    // Coherencia de tamaños antes de construir nada.
    let per_cell = [
        keys.len(),
        level.len(),
        lat.len(),
        lon.len(),
        area.len(),
        east.len(),
        north.len(),
        dir.len(),
        receiver.len(),
        surface.len(),
        biome.len(),
        hydro_lock.len(),
    ];
    if per_cell.iter().any(|&x| x != num)
        || cols.iter().any(|c| c.len() != num)
        || nb_off.len() != num + 1
        || nb_off.last().copied().map(|x| x as usize) != Some(nb.len())
        || flow_order.len() > num
    {
        return None;
    }

    let mut it = cols.into_iter();
    let mut next = || it.next().unwrap_or_default();
    let world = World {
        topo: Topology {
            n0,
            l_min,
            l_max,
            num_cells: num,
            r_planet,
            keys,
            level,
            lat,
            lon,
            area,
            east,
            north,
            dir,
            nb_off,
            nb,
            pairs,
        },
        statgen: StaticGen::new(cfg),
        h0: next(),
        grad0_e: next(),
        grad0_n: next(),
        w_cap0: next(),
        k_perm: next(),
        fert: next(),
        resource: next(),
        e: next(),
        z_soil: next(),
        h: next(),
        slope: next(),
        aspect: next(),
        grad_e: next(),
        grad_n: next(),
        w_cap: next(),
        filled_h: next(),
        receiver,
        drainage_area: next(),
        flow_order,
        dist_ocean_km: next(),
        surface,
        w: next(),
        temp: next(),
        precip: next(),
        swe: next(),
        veg: next(),
        biome,
        hydro_lock,
        temp_sum: next(),
        precip_sum: next(),
        oro0_e: next(),
        oro0_n: next(),
        oro_e: next(),
        oro_n: next(),
        oro_air0_e: next(),
        w_hum: next(),
        c_p,
        desvio_lluvia,
        // La caché guarda mundos ya envejecidos: el agotamiento está activo si la config lo pide.
        humedad_activa: cfg.climate.agotamiento_humedad,
        anillos: None,
        sample_count,
    };
    Some((world, geo))
}
