//! Altura isostática y nivel del mar (§6.7 del diseño, con la corrección de realismo C1).
//!
//! - Corteza continental: isostasia de toda la litosfera (nota de isostasia con litosfera; regla R4
//!   del estudio de isostasia continental, con los parámetros de la comprobación de su §0). La
//!   flotación de una columna de corteza `h` (con su sedimento, que cuenta como corteza) sobre una
//!   litosfera de `L` km es
//!   `B = (ρ_a − ρ_sup)·min(h, z_t) + (ρ_a − ρ_inf)·max(0, h − z_t) − Δρ_ma·max(0, L − h)`,
//!   con `L` de la edad tectonotérmica ([`litosfera_km`]) y `Δρ_ma` menor en el manto arcaico. Sobre
//!   el mar, `H = B/ρ_a − H_0`; bajo el mar, la profundidad se multiplica por `ρ_a/(ρ_a − ρ_w)`.
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

/// Grosor de la litosfera (km) con la edad tectonotérmica `t` (Myr): la recta de Artemieva (2006),
/// `L_lin = l0 + l_pend·t`, con recuperación tras el evento que la calentó,
/// `L = L_min + (L_lin − L_min)·(1 − e^(−t/τ_L))` (forma R3b del estudio).
pub fn litosfera_km(c: &TectonicaCfg, t: f64) -> f64 {
    let t = t.max(0.0);
    let l_lin = c.l0_lit_km + c.l_pend_km_myr * t;
    c.l_min_km + (l_lin - c.l_min_km) * (1.0 - (-t / c.tau_l_myr).exp())
}

/// Contraste neto del manto litosférico con la astenosfera (kg/m³) con la edad `t` (Myr): el del
/// manto arcaico, más empobrecido, por encima de `t_arc`.
#[inline]
pub fn drho_ma(c: &TectonicaCfg, t: f64) -> f64 {
    if t > c.t_arc_myr {
        c.drho_ma_arcaico
    } else {
        c.drho_ma
    }
}

/// Flotación (km·kg/m³) de una columna continental de corteza total `h_tot` (km, con su sedimento)
/// sobre una litosfera de `l` km con contraste `drho`: la densidad de la corteza crece con la
/// profundidad (`ρ_sup` hasta `z_t`, `ρ_inf` debajo) y el manto litosférico pesa `drho` más que la
/// astenosfera.
#[inline]
pub fn flotacion_litosfera(c: &TectonicaCfg, h_tot: f64, l: f64, drho: f64) -> f64 {
    (c.rho_ast - c.rho_c_sup) * h_tot.min(c.z_t_km)
        + (c.rho_ast - c.rho_c_inf) * (h_tot - c.z_t_km).max(0.0)
        - drho * (l - h_tot).max(0.0)
}

/// Hundimiento térmico de un margen estirado (km, con agua encima; nota de hundimiento de los
/// márgenes §6.2): el primer término de McKenzie (1978),
/// `S = E_0·(β/π)·sin(π/β)·(1 − e^(−t/τ))`, con `β` su estiramiento y `t` el tiempo desde la rotura.
/// Sin estirar (`β ≤ 1`), 0.
pub fn hundimiento_mckenzie(c: &TectonicaCfg, beta: f64, t: f64) -> f64 {
    if beta <= 1.0 {
        return 0.0;
    }
    let f = beta / std::f64::consts::PI * (std::f64::consts::PI / beta).sin();
    c.e0_mck_km * f * (1.0 - (-t.max(0.0) / c.tau_mck_myr).exp())
}

/// Lo que hunde ya R4 (km, con agua encima) desde la rotura de una columna de corteza total `h_tot`
/// cuya edad tectonotérmica es `edad`: el crecimiento del término del manto litosférico desde la
/// rotura (edad 0), `[Δρ(t)·max(0, L(t) − h) − Δρ(0)·max(0, L(0) − h)]/(ρ_a − ρ_w)` (§6.3 y §11.1).
pub fn hundimiento_r4(c: &TectonicaCfg, edad: f64, h_tot: f64) -> f64 {
    let tapa = |t: f64| drho_ma(c, t) * (litosfera_km(c, t) - h_tot).max(0.0);
    (tapa(edad) - tapa(0.0)) / (c.rho_ast - c.rho_w)
}

/// Carga del manto (km·kg/m³, en las unidades de [`flotacion_litosfera`]) que añade el hundimiento
/// térmico de McKenzie a una columna estirada: solo lo que R4 no hunde ya,
/// `max(0, S_McK − S_R4)·(ρ_a − ρ_w)`, para que la isostasia dé la altura con cualquier relleno
/// encima (agua, sedimento o aire). Sin estirar, 0.
pub fn carga_mckenzie(c: &TectonicaCfg, k: &Corteza, h_tot: f64) -> f64 {
    if k.beta <= 1.0 {
        return 0.0;
    }
    let s = hundimiento_mckenzie(c, k.beta, k.t_rot_myr) - hundimiento_r4(c, k.edad_tt, h_tot);
    s.max(0.0) * (c.rho_ast - c.rho_w)
}

/// Altura (m, respecto al dato) de una flotación de litosfera `b`: `b/ρ_a − H_0` sobre el mar y,
/// bajo el mar, la profundidad por `ρ_a/(ρ_a − ρ_w)` (el agua también pesa).
#[inline]
pub fn altura_de_flotacion(c: &TectonicaCfg, b: f64) -> f64 {
    let z = 1000.0 * b / c.rho_ast - c.h0_lit_m;
    if z < 0.0 {
        z * c.rho_ast / (c.rho_ast - c.rho_w)
    } else {
        z
    }
}

/// Altura isostática (m, respecto al dato) de una columna de corteza, sin fosa ni detalle.
pub fn altura(c: &TectonicaCfg, k: &Corteza) -> f64 {
    if k.cont {
        // El sedimento cuenta como corteza (nota de isostasia con litosfera §11.1; la variante del
        // estudio que el motor puede llevar: su reparto del sedimento rellena hasta una cota).
        let l = litosfera_km(c, k.edad_tt);
        let h_tot = k.h + k.sed;
        altura_de_flotacion(
            c,
            flotacion_litosfera(c, h_tot, l, drho_ma(c, k.edad_tt)) - carga_mckenzie(c, k, h_tot),
        )
    } else {
        let sed = (c.rho_m - c.rho_s) * k.sed;
        let z0 = -profundidad_por_edad(c, k.edad_oc);
        con_flotacion(c, z0, (c.rho_m - c.rho_oc) * (k.h - c.h_oc_km) + sed)
    }
}

/// Lo que baja la superficie de una columna al aire por km de roca que se le quita de arriba
/// (km/km): en la continental, la derivada de su flotación de litosfera; en la oceánica, la de la
/// corteza con el manto (como antes de la nota).
pub fn baja_por_km(c: &TectonicaCfg, k: &Corteza) -> f64 {
    if !k.cont {
        return (c.rho_m - c.rho_c) / c.rho_m;
    }
    let h_tot = k.h + k.sed;
    let corteza = if h_tot > c.z_t_km {
        c.rho_ast - c.rho_c_inf
    } else {
        c.rho_ast - c.rho_c_sup
    };
    let manto = if litosfera_km(c, k.edad_tt) > h_tot {
        drho_ma(c, k.edad_tt)
    } else {
        0.0
    };
    (corteza + manto) / c.rho_ast
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

/// Espesor de sedimento (km) que hay que añadir a la columna `k` para llevarla hasta la altura
/// `z_obj` (m), con su isostasia (bisección sobre [`altura`]; 0 si ya llega). Lo que no depende del
/// sedimento se calcula una vez.
pub fn sedimento_hasta_columna(c: &TectonicaCfg, k: &Corteza, z_obj: f64) -> f64 {
    // Continental: litosfera y contraste de su edad; oceánica: su profundidad y su flotación.
    let (l, drho) = if k.cont {
        (litosfera_km(c, k.edad_tt), drho_ma(c, k.edad_tt))
    } else {
        (0.0, 0.0)
    };
    let (z0, b0) = if k.cont {
        (0.0, 0.0)
    } else {
        (
            -profundidad_por_edad(c, k.edad_oc),
            (c.rho_m - c.rho_oc) * (k.h - c.h_oc_km),
        )
    };
    let (cont, h_tot, sed0, f_sed) = (k.cont, k.h + k.sed, k.sed, c.rho_m - c.rho_s);
    // El hundimiento de McKenzie no depende del sedimento; lo que R4 ya hunde, solo si la corteza
    // pasa del grosor de la litosfera (nunca en un margen fino), así que se recalcula.
    let altura_con = |ds: f64| -> f64 {
        if cont {
            altura_de_flotacion(
                c,
                flotacion_litosfera(c, h_tot + ds, l, drho) - carga_mckenzie(c, k, h_tot + ds),
            )
        } else {
            con_flotacion(c, z0, b0 + f_sed * (sed0 + ds))
        }
    };
    if altura_con(0.0) >= z_obj {
        return 0.0;
    }
    let (mut lo, mut hi) = (0.0, 1.0);
    while altura_con(hi) < z_obj && hi < 1e3 {
        hi *= 2.0;
    }
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        // Sin punto representable entre los dos, el resto de las vueltas no cambiaría nada.
        if mid <= lo || mid >= hi {
            break;
        }
        if altura_con(mid) < z_obj {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    hi
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

    fn cont(h: f64, sed: f64, edad_tt: f64) -> Corteza {
        Corteza {
            edad_orog: 0.0,
            sed,
            edad_tt,
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
    fn la_isostasia_con_litosfera_da_los_numeros_del_estudio() {
        // Nota de isostasia con litosfera §10, prueba 1: la regla dentro del motor da los mismos
        // números que la herramienta del estudio (herramientas/analisis/estudio_isostasia.py, con
        // los parámetros del §0 y el sedimento como corteza): (h, s, edad) → altura en m.
        let c = TectonicaCfg::default();
        let casos = [
            (35.0, 0.0, 1000.0, 59.8324),
            (40.0, 2.0, 3000.0, 574.7034),
            (70.0, 0.0, 30.0, 3559.0352),
            (10.0, 3.0, 0.0, -1966.4452),
            (33.0, 1.0, 50.0, 478.0659),
            (45.0, 0.5, 1800.0, 717.2533),
            (20.0, 4.0, 100.0, -695.5344),
        ];
        for (h, s, t, esperada) in casos {
            let z = altura(&c, &cont(h, s, t));
            assert!(
                (z - esperada).abs() < 0.1,
                "({h}, {s}, {t}): {z:.4} m frente a {esperada:.4} del estudio"
            );
        }
        assert!((litosfera_km(&c, 1000.0) - 133.5962).abs() < 1e-3);
        assert!((litosfera_km(&c, 0.0) - c.l_min_km).abs() < 1e-12);
    }

    #[test]
    fn un_margen_estirado_se_hunde_al_envejecer() {
        // §10, prueba 2 (la parte de la regla): una columna fina (12 km) recién estirada queda baja y
        // se hunde a medida que su litosfera se engruesa al enfriarse.
        let c = TectonicaCfg::default();
        let prof = |t: f64| -altura(&c, &cont(12.0, 0.0, t));
        let (p0, p50, p100, p300) = (prof(0.0), prof(50.0), prof(100.0), prof(300.0));
        assert!(p0 > 0.0, "nace bajo el mar: {p0:.0} m");
        assert!(
            p50 > p0 && p100 > p50 && p300 > p100,
            "se hunde con la edad: {p0:.0}, {p50:.0}, {p100:.0}, {p300:.0} m"
        );
        // Lo que baja por el manto litosférico que crece: Δρ_ma·ΔL/ρ_a, bajo el agua.
        let esperado =
            c.drho_ma * (litosfera_km(&c, 100.0) - c.l_min_km) / (c.rho_ast - c.rho_w) * 1000.0;
        assert!(
            ((p100 - p0) - esperado).abs() < 1.0,
            "en 100 Myr baja {:.0} m (esperado {esperado:.0})",
            p100 - p0
        );
    }

    #[test]
    fn sin_estirar_nada_cambia_y_un_margen_estirado_sigue_a_mckenzie() {
        // Nota de hundimiento de los márgenes §10, prueba 2: una columna sin estirar (β = 1) da la
        // altura de R4 de antes. Y una estirada (β = 3, 12 km) se hunde con agua encima lo que da
        // McKenzie, sin contar dos veces lo que R4 ya hunde: a los 100 Myr, ~2,1 km.
        let c = TectonicaCfg::default();
        let col = |beta: f64, t: f64| Corteza {
            beta,
            t_rot_myr: t,
            ..cont(12.0, 0.0, t)
        };
        let r4 = |t: f64| {
            let k = cont(12.0, 0.0, t);
            altura_de_flotacion(
                &c,
                flotacion_litosfera(&c, 12.0, litosfera_km(&c, t), drho_ma(&c, t)),
            ) - altura(&c, &k)
        };
        assert_eq!(r4(0.0), 0.0);
        assert_eq!(r4(100.0), 0.0);
        assert_eq!(
            altura(&c, &col(1.0, 100.0)),
            altura(&c, &cont(12.0, 0.0, 100.0))
        );
        let s = |t: f64| altura(&c, &col(3.0, 0.0)) - altura(&c, &col(3.0, t));
        let mck = hundimiento_mckenzie(&c, 3.0, 100.0);
        assert!(
            (mck - 2.11).abs() < 0.01,
            "McKenzie con β = 3 a los 100 Myr: {mck:.3} km"
        );
        assert!(
            (s(100.0) / 1000.0 - mck).abs() < 1e-6,
            "se hunde {:.4} km, McKenzie {mck:.4}",
            s(100.0) / 1000.0
        );
        // Más tarde, ~2,65 km; con β = 1,5, ~1,05 km a los 100 Myr.
        assert!((hundimiento_mckenzie(&c, 3.0, 1e4) - 2.65).abs() < 0.01);
        assert!((hundimiento_mckenzie(&c, 1.5, 100.0) - 1.05).abs() < 0.01);
        // Con poco estiramiento, R4 ya hunde más que McKenzie: no se añade nada.
        let poco = Corteza {
            beta: 1.02,
            t_rot_myr: 100.0,
            ..cont(12.0, 0.0, 100.0)
        };
        assert!(hundimiento_mckenzie(&c, 1.02, 100.0) < hundimiento_r4(&c, 100.0, 12.0));
        assert_eq!(altura(&c, &poco), altura(&c, &cont(12.0, 0.0, 100.0)));
    }

    #[test]
    fn el_sedimento_cuenta_como_corteza_y_rellena_hasta_la_plataforma() {
        // §11.1: el sedimento de una columna continental cuenta como corteza (su capa de arriba), así
        // que sumarlo sube la columna lo que su grosor de corteza; el reparto del sedimento puede
        // rellenar un margen hasta la plataforma sin pasarse.
        let c = TectonicaCfg::default();
        let a = altura(&c, &cont(25.0, 0.0, 200.0));
        let b = altura(&c, &cont(24.0, 1.0, 200.0));
        assert!(
            (a - b).abs() < 1e-9,
            "1 km de sedimento pesa como 1 km de corteza"
        );
        let mut m = cont(14.0, 0.0, 120.0);
        let z0 = altura(&c, &m);
        assert!(z0 < -130.0, "el margen empieza hondo: {z0:.0} m");
        m.sed = sedimento_hasta_columna(&c, &m, -130.0);
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
