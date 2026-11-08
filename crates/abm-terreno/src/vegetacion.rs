//! Producción vegetal (adenda 1 al modo Tierra §2): el modelo de Miami (Lieth 1975), la única
//! producción del motor. La usan el terreno (la vegetación del modo Tierra y el diagnóstico) y los
//! agentes (el forraje de su hábitat), con las mismas unidades: gramos de materia seca por m² y año.

/// Fracción de carbono de la materia seca vegetal (IPCC 2006, valor por defecto): para comparar la
/// producción de Miami (materia seca) con la de MODIS (carbono).
pub const CARBONO_MATERIA_SECA: f64 = 0.47;

/// Producción primaria neta anual de Miami (g de materia seca/m²/año): la limita el calor o el agua,
/// el que sea más escaso. Temperatura media anual en °C y lluvia anual en mm; los coeficientes son los
/// del modelo publicado.
pub fn ppn_miami(t_media: f64, p_media_mm: f64) -> f64 {
    let f_t = 3000.0 / (1.0 + (1.315 - 0.119 * t_media).exp());
    let f_p = 3000.0 * (1.0 - (-0.000664 * p_media_mm).exp());
    f_t.min(f_p).max(0.0)
}

/// Reparto de la producción de un año entre sus meses (adenda 1 §2.2): cada mes pesa lo que daría
/// Miami con su temperatura y su lluvia multiplicada por 12, y los pesos suman 1. Así la vegetación no
/// crece en los meses fríos ni en los secos. Si ningún mes produce nada, el reparto es uniforme.
pub fn reparto_mensual(t: &[f64; 12], p_mes: &[f64; 12]) -> [f64; 12] {
    let mut w = [0.0; 12];
    for m in 0..12 {
        w[m] = ppn_miami(t[m], 12.0 * p_mes[m]);
    }
    let s: f64 = w.iter().sum();
    if s > 0.0 {
        w.iter_mut().for_each(|x| *x /= s);
    } else {
        w = [1.0 / 12.0; 12];
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn miami_da_los_valores_publicados() {
        // Con 25 °C y 3 000 mm limita el calor: 3 000 / (1 + e^(1,315 − 2,975)) = 2 520,7; con 0 °C,
        // 3 000 / (1 + e^1,315) = 635,0; con 500 mm y 25 °C limita el agua: 3 000 (1 − e^−0,332) = 847,5.
        assert!((ppn_miami(25.0, 3000.0) - 2520.71).abs() < 0.01);
        assert!((ppn_miami(0.0, 2000.0) - 634.95).abs() < 0.01);
        assert!((ppn_miami(25.0, 500.0) - 847.54).abs() < 0.01);
        assert_eq!(ppn_miami(20.0, 0.0), 0.0);
    }

    #[test]
    fn el_reparto_suma_uno_y_no_da_nada_a_los_meses_secos() {
        let t = [20.0; 12];
        let mut p = [100.0; 12];
        p[5] = 0.0;
        let w = reparto_mensual(&t, &p);
        assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        assert_eq!(w[5], 0.0);
        assert!((w[0] - 1.0 / 11.0).abs() < 1e-12);
        assert_eq!(reparto_mensual(&t, &[0.0; 12]), [1.0 / 12.0; 12]);
    }
}
