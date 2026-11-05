//! Azar determinista por contador y sumas exactas (diseño de biología §6.11; incremento 1, D12–D13).
//!
//! Cada número aleatorio es una función pura `H(semilla, id, tick, propósito, índice)`: no hay un
//! estado del generador que avance, así que el resultado no depende del orden en que se procesen las
//! entidades. Las sumas que vuelven al modelo o a la salida se hacen en punto fijo (`SumaExacta`),
//! que es asociativa: tampoco dependen del orden.

/// Para qué se usa cada número aleatorio (dos decisiones distintas nunca comparten número).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum Proposito {
    Concepcion = 1,
    Mortalidad = 2,
    SexoCria = 3,
    Camada = 4,
    Orfandad = 5,
    EdadInicial = 6,
    SexoInicial = 7,
    IdInicial = 8,
    IdCria = 9,
    Barajado = 10,
    /// Edad de esterilidad de cada hembra (revisión 1 §4).
    Esterilidad = 11,
    /// Muerte de las crías jóvenes en un destete forzoso (revisión 1 §3.2).
    Abandono = 12,
}

/// Mezclador SplitMix64 (función biyectiva de 64 bits con buena difusión).
#[inline]
pub fn mezclar(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// `H(semilla, id, tick, propósito, índice)`: cadena de mezcladores, un campo cada vez.
#[inline]
pub fn hash(semilla: u64, id: u64, tick: u64, proposito: Proposito, indice: u64) -> u64 {
    let mut h = mezclar(semilla);
    h = mezclar(h ^ id);
    h = mezclar(h ^ tick);
    h = mezclar(h ^ proposito as u64);
    mezclar(h ^ indice)
}

/// Uniforme en `[0, 1)` con 53 bits de mantisa.
#[inline]
pub fn uniforme(semilla: u64, id: u64, tick: u64, proposito: Proposito, indice: u64) -> f64 {
    (hash(semilla, id, tick, proposito, indice) >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

/// Escala del punto fijo: 2³² unidades por kcal (resolución ~2,3·10⁻¹⁰ kcal).
const ESCALA: f64 = 4_294_967_296.0;

/// Suma en punto fijo (`i128`): exacta y asociativa, así que no depende del orden de los sumandos.
/// Cabe de sobra: 10¹⁸ kcal · 2³² ≈ 4·10²⁷, frente a un máximo de ~1,7·10³⁸.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SumaExacta(i128);

impl SumaExacta {
    /// Suma `x` redondeado a la resolución del punto fijo.
    #[inline]
    pub fn sumar(&mut self, x: f64) {
        debug_assert!(x.is_finite(), "sumando no finito: {x}");
        self.0 += (x * ESCALA).round() as i128;
    }

    /// Suma otra suma exacta.
    #[inline]
    pub fn juntar(&mut self, otra: SumaExacta) {
        self.0 += otra.0;
    }

    /// Valor en `f64`.
    #[inline]
    pub fn valor(&self) -> f64 {
        self.0 as f64 / ESCALA
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn azar_por_contador_determinista_y_uniforme() {
        let a = uniforme(42, 7, 100, Proposito::Mortalidad, 0);
        assert_eq!(a, uniforme(42, 7, 100, Proposito::Mortalidad, 0));
        assert_ne!(a, uniforme(42, 7, 100, Proposito::Concepcion, 0));
        assert_ne!(a, uniforme(42, 7, 101, Proposito::Mortalidad, 0));
        assert_ne!(a, uniforme(43, 7, 100, Proposito::Mortalidad, 0));
        // Media y varianza de 100 000 uniformes: 1/2 y 1/12.
        let n = 100_000;
        let xs: Vec<f64> = (0..n)
            .map(|i| uniforme(1, i, 0, Proposito::Mortalidad, 0))
            .collect();
        let media = xs.iter().sum::<f64>() / n as f64;
        let var = xs.iter().map(|x| (x - media).powi(2)).sum::<f64>() / n as f64;
        assert!((media - 0.5).abs() < 0.005, "media {media}");
        assert!((var - 1.0 / 12.0).abs() < 0.002, "varianza {var}");
        assert!(xs.iter().all(|&x| (0.0..1.0).contains(&x)));
    }

    #[test]
    fn la_suma_exacta_no_depende_del_orden() {
        let xs: Vec<f64> = (0..10_000)
            .map(|i| (uniforme(3, i, 0, Proposito::Barajado, 0) - 0.3) * 1e6)
            .collect();
        let mut a = SumaExacta::default();
        xs.iter().for_each(|&x| a.sumar(x));
        let mut b = SumaExacta::default();
        xs.iter().rev().for_each(|&x| b.sumar(x));
        let mut c = SumaExacta::default();
        let mut idx: Vec<usize> = (0..xs.len()).collect();
        idx.sort_by_key(|&i| hash(9, i as u64, 0, Proposito::Barajado, 0));
        idx.iter().for_each(|&i| c.sumar(xs[i]));
        assert_eq!(a, b);
        assert_eq!(a, c);
        let directa: f64 = xs.iter().sum();
        assert!((a.valor() - directa).abs() < 1e-5);
    }
}
