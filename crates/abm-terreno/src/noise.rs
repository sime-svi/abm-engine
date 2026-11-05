//! Ruido coherente 3D (gradiente tipo Perlin) sembrado y determinista.
//!
//! Se evalúa sobre la posición 3D del centro de la celda (no 2D por cara), de modo que la elevación
//! es **continua al cruzar aristas y esquinas** del cubo sin costuras (diseño §4a). La permutación
//! se deriva de la semilla con `ChaCha8Rng` (misma secuencia entre plataformas, CLAUDE §6).

use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

/// Tabla de permutación 512 (256 duplicada) para el ruido de gradiente.
#[derive(Clone)]
pub struct Perlin3 {
    perm: [u8; 512],
}

impl Perlin3 {
    /// Construye el ruido a partir de una semilla. Semillas distintas => campos distintos.
    pub fn new(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let mut p: [u8; 256] = std::array::from_fn(|i| i as u8);
        p.shuffle(&mut rng);
        let mut perm = [0u8; 512];
        for i in 0..512 {
            perm[i] = p[i & 255];
        }
        Self { perm }
    }

    #[inline]
    fn grad(hash: u8, x: f64, y: f64, z: f64) -> f64 {
        // 12 gradientes hacia las aristas de un cubo (esquema clásico de Perlin).
        match hash & 15 {
            0 => x + y,
            1 => -x + y,
            2 => x - y,
            3 => -x - y,
            4 => x + z,
            5 => -x + z,
            6 => x - z,
            7 => -x - z,
            8 => y + z,
            9 => -y + z,
            10 => y - z,
            11 => -y - z,
            12 => x + y,
            13 => -y + z,
            14 => -x + y,
            _ => -y - z,
        }
    }

    /// Ruido en `(x, y, z)`, resultado en `[-1, 1]` aproximado, continuo y derivable.
    pub fn get(&self, x: f64, y: f64, z: f64) -> f64 {
        let xi = x.floor();
        let yi = y.floor();
        let zi = z.floor();
        let (xf, yf, zf) = (x - xi, y - yi, z - zi);
        let (xb, yb, zb) = (
            (xi as i64 & 255) as usize,
            (yi as i64 & 255) as usize,
            (zi as i64 & 255) as usize,
        );
        let fade = |t: f64| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
        let lerp = |a: f64, b: f64, t: f64| a + t * (b - a);
        let (u, v, w) = (fade(xf), fade(yf), fade(zf));

        let p = &self.perm;
        let aaa = p[p[p[xb] as usize + yb] as usize + zb];
        let aba = p[p[p[xb] as usize + yb + 1] as usize + zb];
        let aab = p[p[p[xb] as usize + yb] as usize + zb + 1];
        let abb = p[p[p[xb] as usize + yb + 1] as usize + zb + 1];
        let baa = p[p[p[xb + 1] as usize + yb] as usize + zb];
        let bba = p[p[p[xb + 1] as usize + yb + 1] as usize + zb];
        let bab = p[p[p[xb + 1] as usize + yb] as usize + zb + 1];
        let bbb = p[p[p[xb + 1] as usize + yb + 1] as usize + zb + 1];

        let x1 = lerp(
            Self::grad(aaa, xf, yf, zf),
            Self::grad(baa, xf - 1.0, yf, zf),
            u,
        );
        let x2 = lerp(
            Self::grad(aba, xf, yf - 1.0, zf),
            Self::grad(bba, xf - 1.0, yf - 1.0, zf),
            u,
        );
        let y1 = lerp(x1, x2, v);
        let x3 = lerp(
            Self::grad(aab, xf, yf, zf - 1.0),
            Self::grad(bab, xf - 1.0, yf, zf - 1.0),
            u,
        );
        let x4 = lerp(
            Self::grad(abb, xf, yf - 1.0, zf - 1.0),
            Self::grad(bbb, xf - 1.0, yf - 1.0, zf - 1.0),
            u,
        );
        let y2 = lerp(x3, x4, v);
        lerp(y1, y2, w)
    }

    /// Suma multi-octava (fractal browniano) **sin normalizar**: `Σ_{k<oct} p^k · ruido(λ^k · x)`.
    ///
    /// Es la pieza del LOD (incremento 2 §3.1): al dividir por una normalización **fija**, añadir
    /// octavas en los niveles finos solo suma detalle, sin reescalar las octavas anteriores.
    pub fn fbm_sum(
        &self,
        x: f64,
        y: f64,
        z: f64,
        oct: u32,
        persistencia: f64,
        lacunaridad: f64,
    ) -> f64 {
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut sum = 0.0;
        for _ in 0..oct {
            sum += amp * self.get(x * freq, y * freq, z * freq);
            amp *= persistencia;
            freq *= lacunaridad;
        }
        sum
    }

    /// Suma de amplitudes `Σ_{k<oct} p^k` (normalización de [`Self::fbm_sum`]).
    pub fn fbm_norm(oct: u32, persistencia: f64) -> f64 {
        let mut amp = 1.0;
        let mut norm = 0.0;
        for _ in 0..oct {
            norm += amp;
            amp *= persistencia;
        }
        norm
    }

    /// Suma multi-octava normalizada por sus propias amplitudes, en `[-1, 1]` aprox.
    pub fn fbm(
        &self,
        x: f64,
        y: f64,
        z: f64,
        oct: u32,
        persistencia: f64,
        lacunaridad: f64,
    ) -> f64 {
        let norm = Self::fbm_norm(oct, persistencia);
        if norm > 0.0 {
            self.fbm_sum(x, y, z, oct, persistencia, lacunaridad) / norm
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn determinista_y_acotado() {
        let a = Perlin3::new(7);
        let b = Perlin3::new(7);
        for k in 0..1000 {
            let t = k as f64 * 0.123;
            let va = a.get(t, t * 0.7, t * 1.9);
            assert_eq!(va, b.get(t, t * 0.7, t * 1.9));
            assert!(va.abs() <= 1.5);
        }
    }
}
