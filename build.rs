//! Script de compilación: calcula una huella del código del terreno para la caché de la génesis.
//!
//! La génesis geológica depende de la semilla, de la config y del **código** que la calcula. La
//! caché (`src/terreno/cache.rs`) usa esta huella como parte de su clave: si cambia cualquier fuente
//! de `src/terreno/`, el `Cargo.lock` o el `rust-toolchain.toml`, la huella cambia y la caché vieja
//! se ignora. Así una caché nunca puede devolver un mundo calculado con otra versión del código.

use std::fs;
use std::path::{Path, PathBuf};

/// FNV-1a de 64 bits (sin dependencias; solo identifica contenidos, no es criptográfico).
fn fnv1a(mut h: u64, bytes: &[u8]) -> u64 {
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    h
}

/// Ficheros de un directorio, recursivamente, en orden estable.
fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            files(&p, out);
        } else {
            out.push(p);
        }
    }
}

fn main() {
    let mut paths = Vec::new();
    files(Path::new("src/terreno"), &mut paths);
    paths.push(PathBuf::from("Cargo.lock"));
    paths.push(PathBuf::from("rust-toolchain.toml"));

    let mut h = 0xcbf2_9ce4_8422_2325_u64;
    for p in &paths {
        h = fnv1a(h, p.to_string_lossy().as_bytes());
        h = fnv1a(h, &fs::read(p).unwrap_or_default());
    }
    println!("cargo:rustc-env=ABM_TERRENO_FINGERPRINT={h:016x}");
    println!("cargo:rerun-if-changed=src/terreno");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=rust-toolchain.toml");
}
