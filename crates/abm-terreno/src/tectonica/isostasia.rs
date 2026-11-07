//! Altura isostática y nivel del mar (§6.7 del diseño, con la corrección de realismo C1).
//!
//! - Corteza continental (isostasia de Airy con la relación empírica terrestre): la flotación de una
//!   columna es `B = (ρ_m − ρ_c)·(H − H_nivel) + (ρ_m − ρ_s)·s` (s = sedimento). Por encima del nivel
//!   del mar sube `B/ρ_m`; por debajo, con agua encima, `B/(ρ_m − ρ_w)` (C1).
//! - Corteza oceánica: profundidad por edad (Parsons y Sclater, 1977) **más** la flotación del
//!   espesor extra de los arcos (decisión del responsable), con el mismo tratamiento del agua.
//! - Fosas: sobreprofundidad gaussiana en el lado de la placa que subduce.
//! - Nivel del mar: el que llena las cuencas con el volumen de agua (bisección determinista).

use super::deriva::Corteza;
use crate::config::TectonicaCfg;

/// Profundidad (m, positiva) del fondo oceánico de edad `t` (Myr).
pub fn profundidad_por_edad(c: &TectonicaCfg, t: f64) -> f64 {
    let t = t.max(0.0);
    if t <= c.t_x_myr {
        c.d_r_m + c.c_hs * t.sqrt()
    } else {
        c.d_inf_m - c.a_pl_m * (-t / c.tau_pl_myr).exp()
    }
}

/// Altura (m) de una columna que parte de `z0` (m) y tiene una flotación extra `b` (km·kg/m³,
/// positiva hacia arriba): bajo el agua sube `b/(ρ_m − ρ_w)` y, si asoma, el resto `/ρ_m`. Con
/// `b < 0` baja: al aire `/ρ_m` y, cuando se sumerge, `/(ρ_m − ρ_w)`.
pub fn con_flotacion(c: &TectonicaCfg, z0_m: f64, b: f64) -> f64 {
    let agua = c.rho_m - c.rho_w;
    let aire = c.rho_m;
    if b >= 0.0 {
        if z0_m >= 0.0 {
            return z0_m + 1000.0 * b / aire;
        }
        let z = z0_m + 1000.0 * b / agua;
        if z <= 0.0 {
            return z;
        }
        // Lo que sobra al asomar: la flotación que no hizo falta para llegar al nivel del mar.
        let resto = b - (-z0_m / 1000.0) * agua;
        1000.0 * resto / aire
    } else {
        if z0_m <= 0.0 {
            return z0_m + 1000.0 * b / agua;
        }
        let z = z0_m + 1000.0 * b / aire;
        if z >= 0.0 {
            return z;
        }
        let resto = b + (z0_m / 1000.0) * aire;
        1000.0 * resto / agua
    }
}

/// Altura isostática (m, respecto al dato) de una columna de corteza, sin fosa ni detalle.
pub fn altura(c: &TectonicaCfg, k: &Corteza) -> f64 {
    let sed = (c.rho_m - c.rho_s) * k.sed;
    if k.cont {
        con_flotacion(c, 0.0, (c.rho_m - c.rho_c) * (k.h - c.h_nivel_km) + sed)
    } else {
        let z0 = -profundidad_por_edad(c, k.edad_oc);
        con_flotacion(c, z0, (c.rho_m - c.rho_oc) * (k.h - c.h_oc_km) + sed)
    }
}

/// Sobreprofundidad (m, positiva) de la fosa a una distancia firmada `s` (km) del borde: solo en el
/// lado de la placa que subduce (`s ≥ 0`).
#[inline]
pub fn fosa_m(c: &TectonicaCfg, s_km: f64) -> f64 {
    if s_km >= 0.0 && s_km < 6.0 * c.w_fosa_km {
        c.d_fosa_m * (-(s_km / c.w_fosa_km).powi(2)).exp()
    } else {
        0.0
    }
}

/// Espesor de sedimento (km) que lleva una columna de corteza continental hasta la altura `z_obj`
/// (m) desde la suya `z` (m), las dos bajo el agua.
pub fn sedimento_hasta(c: &TectonicaCfg, z_m: f64, z_obj_m: f64) -> f64 {
    if z_obj_m <= z_m {
        return 0.0;
    }
    (z_obj_m - z_m) / 1000.0 * (c.rho_m - c.rho_w) / (c.rho_m - c.rho_s)
}

/// Nivel del mar (m, respecto al dato) que llena las cuencas con `volumen_km3` de agua:
/// `Σ max(0, ζ − z)·A = V`, por bisección (determinista, 200 iteraciones como mucho).
pub fn nivel_del_mar(z_m: &[f64], area_km2: &[f64], volumen_km3: f64) -> f64 {
    let vol = |zeta: f64| -> f64 {
        z_m.iter()
            .zip(area_km2)
            .map(|(&z, &a)| ((zeta - z) / 1000.0).max(0.0) * a)
            .sum::<f64>()
    };
    let (mut lo, mut hi) = (-12_000.0, 12_000.0);
    for _ in 0..200 {
        let mid = 0.5 * (lo + hi);
        if vol(mid) < volumen_km3 {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-6 {
            break;
        }
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cont(h: f64) -> Corteza {
        Corteza {
            edad_orog: 0.0,
            ..Corteza::continental(h)
        }
    }

    #[test]
    fn la_curva_de_placa_es_continua_y_se_aplana_como_el_fondo_real() {
        // Raíz de Parsons y Sclater hasta t_x y placa ajustada a la mediana del zócalo real
        // (incremento-4-diseno.md §46): 4 727 m a 40–50 Myr, ~5,2–5,3 km desde los 80.
        let c = TectonicaCfg::default();
        assert_eq!(profundidad_por_edad(&c, 0.0), 2500.0);
        let antes = profundidad_por_edad(&c, 20.0);
        let despues = profundidad_por_edad(&c, 20.0 + 1e-9);
        assert!(
            (antes - despues).abs() < 1.0,
            "salto de {} m",
            antes - despues
        );
        assert!((profundidad_por_edad(&c, 45.0) - 4727.0).abs() < 50.0);
        assert!((profundidad_por_edad(&c, 150.0) - 5290.0).abs() < 30.0);
        let aplanado = profundidad_por_edad(&c, 150.0) - profundidad_por_edad(&c, 80.0);
        assert!(
            aplanado < 200.0,
            "de 80 a 150 Myr baja {aplanado:.0} m (Parsons y Sclater: 556)"
        );
    }

    #[test]
    fn isostasia_continental_al_aire_y_bajo_el_agua() {
        let c = TectonicaCfg::default();
        // Al aire, 0,152 km por km (§6.7): 65 km → +5,0 km; 35 km → +0,45 km.
        assert!((altura(&c, &cont(65.0)) - 5000.0).abs() < 5.0);
        assert!((altura(&c, &cont(35.0)) - 454.5).abs() < 1.0);
        assert!(altura(&c, &cont(32.0)).abs() < 1e-9);
        // Bajo el agua, 0,220 km por km (C1): 25 km → −1,54 km (el documento daba −1,06 al aire).
        assert!((altura(&c, &cont(25.0)) + 1541.9).abs() < 1.0);
        // El sedimento rellena: lleva el margen hasta la plataforma sin pasarse.
        let mut m = cont(25.0);
        m.sed = sedimento_hasta(&c, altura(&c, &m), -130.0);
        assert!((altura(&c, &m) + 130.0).abs() < 1e-6);
    }

    #[test]
    fn la_flotacion_es_continua_al_asomar() {
        let c = TectonicaCfg::default();
        // Un arco oceánico que engorda sube de forma continua al cruzar el nivel del mar.
        let mut prev = f64::NEG_INFINITY;
        for i in 0..400 {
            let h = 7.0 + i as f64 * 0.1;
            let k = Corteza {
                h,
                hc: h - 7.0,
                arco: 1.0,
                edad_oc: 30.0,
                ..Corteza::oceanica(7.0)
            };
            let z = altura(&c, &k);
            assert!(z > prev && (prev == f64::NEG_INFINITY || z - prev < 30.0));
            prev = z;
        }
    }

    #[test]
    fn el_nivel_del_mar_llena_las_cuencas() {
        let z = [-4000.0, -2000.0, 500.0, 1000.0];
        let a = [1.0, 1.0, 1.0, 1.0];
        // 3 km³ de agua: llena la cuenca honda hasta −2000 (2 km³) y luego las dos hasta −1500.
        let zeta = nivel_del_mar(&z, &a, 3.0);
        assert!((zeta + 1500.0).abs() < 1e-3);
    }
}
