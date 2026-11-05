//! Sistemas del tick de la biología (diseño de biología §6, orden §9; incremento 1 §6).
//!
//! Las decisiones al azar usan el generador por contador (`azar`), y todas las sumas que vuelven al
//! modelo o al libro son exactas (`SumaExacta`). Así el resultado no depende del orden en que la
//! ECS recorre las entidades (D12–D13).

use bevy_ecs::prelude::*;

use abm_core::time::Tick;

use super::azar::{Proposito, SumaExacta, hash, uniforme};
use super::componentes::{
    Celda, Dependiente, Diario, Edad, EspecieRef, EstadoRepro, IdBio, Masa, Repro, Reserva, Sexo,
};
use super::config::RespuestaForraje;
use super::especie::Especie;
use super::habitat::Habitat;
use super::recursos::{Buffers, Catalogo, Causa, CriaPendiente, Libro, MuertoInfo, PartoInfo};

/// Inicio del tick: libro a cero y reservas de partida (para el balance de energía).
pub fn bio_inicio(cat: Res<Catalogo>, mut libro: ResMut<Libro>, q: Query<&Reserva>) {
    libro.reiniciar(cat.especies.len());
    let mut s = SumaExacta::default();
    for r in &q {
        s.sumar(r.0);
    }
    libro.reservas_inicio = s;
}

/// T4. Rebrote logístico del forraje (§6.1): `V ← clamp(V + s·[g_0 + r·V·(1 − V/K)], 0, K)`.
pub fn bio_forraje(
    cat: Res<Catalogo>,
    tick: Res<Tick>,
    mut hab: ResMut<Habitat>,
    mut libro: ResMut<Libro>,
) {
    let s = cat.factor_estacional(tick.0);
    let hab = &mut *hab;
    for c in 0..hab.n_celdas {
        for k in 0..hab.n_esp {
            let i = hab.idx(c, k);
            let (kk, v) = (hab.k[i], hab.v[i]);
            let nuevo = if kk > 0.0 {
                (v + s * (hab.g0[i] + hab.r * v * (1.0 - v / kk))).clamp(0.0, kk)
            } else {
                0.0
            };
            hab.v[i] = nuevo;
            libro.esp[k].rebrote_g.sumar((nuevo - v) * hab.area_m2[c]);
        }
    }
}

type QDemanda<'a> = (
    &'a IdBio,
    &'a EspecieRef,
    &'a Edad,
    &'a Masa,
    &'a Reserva,
    &'a Repro,
    &'a Celda,
    Option<&'a Dependiente>,
    &'a mut Diario,
);

/// A3. Demanda y reparto justo (§6.4), con el gasto del día (§6.5):
/// - pasada 1: crecimiento previsto (D2), gasto propio y la leche que necesita cada cría (D3);
/// - pasada 2: leche que paga cada madre, demanda `d = min(Ap, E_max − E + G)/ε` (D1), y su suma
///   por celda y especie;
/// - reparto: `x_i = d_i · min(1, F/Σd)`, con `F = V·A·ρ_veg`.
pub fn bio_demanda(
    cat: Res<Catalogo>,
    hab: Res<Habitat>,
    mut buf: ResMut<Buffers>,
    mut q: Query<QDemanda>,
) {
    let en = &cat.en;
    let n = hab.n_celdas * hab.n_esp;
    let Buffers {
        leche,
        demanda,
        reparto,
        consumo,
        ..
    } = &mut *buf;
    leche.clear();
    for (_, esp, edad, masa, res, repro, celda, dep, mut d) in q.iter_mut() {
        let e = &cat.especies[esp.0 as usize];
        let f_ecto = hab.f_ecto[celda.0 as usize];
        let m = masa.0;
        let dm = if m < e.masa && res.0 >= en.r_crec_min * e.e_max(m) {
            e.dm_west(m)
        } else {
            0.0
        };
        let mut g = e.basal(m, f_ecto) * e.pal + en.c_crec * dm;
        if repro.estado == EstadoRepro::Gestante {
            g += en.k_gest * e.basal(e.masa, f_ecto);
        }
        *d = Diario {
            dm,
            g_propio: g,
            ..Diario::default()
        };
        if let Some(dep) = dep {
            d.leche = e.phi_leche(edad.0) * g;
            if d.leche > 0.0 {
                leche.push((dep.madre, d.leche));
            }
        }
    }
    leche.sort_unstable_by_key(|x| x.0);

    demanda.clear();
    demanda.resize(n, SumaExacta::default());
    consumo.clear();
    consumo.resize(n, SumaExacta::default());
    for (id, esp, edad, masa, res, repro, celda, dep, mut d) in q.iter_mut() {
        let e = &cat.especies[esp.0 as usize];
        let c = celda.0 as usize;
        let m = masa.0;
        if repro.estado == EstadoRepro::Lactante {
            let desde = leche.partition_point(|x| x.0 < id.0);
            let mut s = SumaExacta::default();
            for x in leche[desde..].iter().take_while(|x| x.0 == id.0) {
                s.sumar(x.1);
            }
            d.g_leche = s.valor() / en.eps_leche;
        }
        let g = d.g_propio + d.g_leche;
        let apetito = e.k_ap * e.basal(m, hab.f_ecto[c]) * e.pal;
        let falta = (e.e_max(m) - res.0 + g).max(0.0);
        let mut dem = apetito.min(falta) / e.eps;
        if dep.is_some() {
            dem *= 1.0 - e.phi_leche(edad.0);
        }
        let i = hab.idx(c, esp.0 as usize);
        if cat.forraje.respuesta == RespuestaForraje::Proporcional {
            // Propuesta D16 (fuera del diseño): se consigue comer en proporción al stock.
            dem *= if hab.k[i] > 0.0 {
                hab.v[i] / hab.k[i]
            } else {
                0.0
            };
        }
        d.demanda = dem;
        demanda[i].sumar(dem);
    }

    reparto.clear();
    for c in 0..hab.n_celdas {
        for k in 0..hab.n_esp {
            let i = hab.idx(c, k);
            let oferta = hab.v[i] * hab.area_m2[c] * cat.forraje.rho_veg;
            let pedido = demanda[i].valor();
            reparto.push(if pedido > oferta {
                oferta / pedido
            } else {
                1.0
            });
        }
    }
}

/// A4. Energética (§6.5): ingesta, leche recibida, balance de reservas y libro de energía.
pub fn bio_energetica(
    cat: Res<Catalogo>,
    hab: Res<Habitat>,
    mut buf: ResMut<Buffers>,
    mut libro: ResMut<Libro>,
    mut q: Query<(&EspecieRef, &Masa, &Celda, &mut Reserva, &mut Diario)>,
) {
    let en = &cat.en;
    let buf = &mut *buf;
    let libro = &mut *libro;
    for (esp, masa, celda, mut res, mut d) in q.iter_mut() {
        let k = esp.0 as usize;
        let e = &cat.especies[k];
        let i = hab.idx(celda.0 as usize, k);
        let x = d.demanda * buf.reparto[i];
        d.ingesta = x;
        buf.consumo[i].sumar(x);
        let asim = e.eps * x;
        let ingreso = asim + d.leche;
        let gasto = d.g_propio + d.g_leche;
        let mut nueva = res.0;
        let mut perdida = 0.0;
        if ingreso >= gasto {
            let sobra = ingreso - gasto;
            let guarda = en.eta * sobra;
            nueva += guarda;
            perdida = sobra - guarda;
        } else {
            nueva -= gasto - ingreso;
            d.deficit = true;
        }
        let tope = e.e_max(masa.0);
        let mut excedente = 0.0;
        if nueva > tope {
            excedente = nueva - tope;
            nueva = tope;
        }
        res.0 = nueva;
        libro.asimilada.sumar(asim);
        libro.leche_recibida.sumar(d.leche);
        libro.gasto_propio.sumar(d.g_propio);
        libro.leche_pagada.sumar(d.g_leche);
        libro.perdida_almacen.sumar(perdida);
        libro.excedente.sumar(excedente);
        let ce = &mut libro.esp[k];
        ce.consumo_kcal.sumar(x);
        ce.en_deficit += u64::from(d.deficit);
    }
}

/// A5. Crecimiento (§6.6): `m ← min(m + Δm, M)`, con el `Δm` decidido en A3.
pub fn bio_crecimiento(cat: Res<Catalogo>, mut q: Query<(&EspecieRef, &mut Masa, &Diario)>) {
    for (esp, mut masa, d) in q.iter_mut() {
        if d.dm > 0.0 {
            masa.0 = (masa.0 + d.dm).min(cat.especies[esp.0 as usize].masa);
        }
    }
}

type QRepro<'a> = (
    &'a IdBio,
    &'a EspecieRef,
    &'a Sexo,
    &'a Edad,
    &'a Masa,
    &'a mut Reserva,
    &'a mut Repro,
    &'a Celda,
);

/// Parto (§6.7): crías por Bernoulli de la parte fraccionaria de la camada, sexo por `ρ_sexo`,
/// reserva inicial `r_0·E_max(m0)` pagada por la madre (si no alcanza, reparte lo que tiene).
#[allow(clippy::too_many_arguments)]
fn parto(
    e: &Especie,
    r_0: f64,
    semilla: u64,
    t: u64,
    id: u64,
    esp: u16,
    celda: Celda,
    res: &mut Reserva,
    repro: &mut Repro,
    edad_madre: u32,
    nacimientos: &mut Vec<CriaPendiente>,
    libro: &mut Libro,
) {
    let extra = u32::from(uniforme(semilla, id, t, Proposito::Camada, 0) < e.camada.fract());
    let n = e.camada.floor() as u32 + extra;
    let e0 = r_0 * e.e_max(e.m0);
    let por_cria = if n == 0 {
        0.0
    } else if res.0 >= e0 * n as f64 {
        res.0 -= e0 * n as f64;
        e0
    } else {
        let x = res.0.max(0.0) / n as f64;
        res.0 = res.0.min(0.0);
        x
    };
    for j in 0..n {
        let cid = hash(
            semilla,
            id,
            repro.partos as u64,
            Proposito::IdCria,
            j as u64,
        );
        let macho = uniforme(semilla, cid, t, Proposito::SexoCria, 0) < e.rho_sexo;
        nacimientos.push(CriaPendiente {
            id: cid,
            madre: id,
            esp,
            celda,
            macho,
            reserva: por_cria,
        });
    }
    libro.partos.push(PartoInfo {
        esp,
        edad_madre,
        crias: n,
        intervalo: repro.ultimo_parto.map(|u| t - u),
    });
    libro.esp[esp as usize].nacimientos += n as u64;
    repro.partos += 1;
    repro.ultimo_parto = Some(t);
    if n > 0 {
        repro.estado = EstadoRepro::Lactante;
        repro.dias_rest = e.d_destete;
    } else {
        repro.estado = EstadoRepro::Disponible;
        repro.dias_rest = 0;
    }
}

/// A6. Reproducción sexual (§6.7): madurez, aborto o parto, fin de la lactancia, concepción y fin
/// de la vida reproductiva (D9). Las crías se crean después, en `bio_nacimientos`.
pub fn bio_reproduccion(
    cat: Res<Catalogo>,
    hab: Res<Habitat>,
    tick: Res<Tick>,
    mut buf: ResMut<Buffers>,
    mut libro: ResMut<Libro>,
    mut q: Query<QRepro>,
    qd: Query<&Dependiente>,
) {
    let en = &cat.en;
    let t = tick.0;
    let semilla = cat.semilla;
    let Buffers {
        machos,
        crias,
        nacimientos,
        ..
    } = &mut *buf;
    let libro = &mut *libro;
    machos.clear();
    machos.resize(hab.n_celdas * hab.n_esp, 0);
    for (_, esp, sexo, _, _, _, repro, celda) in q.iter() {
        if *sexo == Sexo::Macho && repro.estado == EstadoRepro::Maduro {
            machos[hab.idx(celda.0 as usize, esp.0 as usize)] += 1;
        }
    }
    crias.clear();
    crias.extend(qd.iter().map(|d| d.madre));
    crias.sort_unstable();
    nacimientos.clear();

    for (id, esp, sexo, edad, masa, mut res, mut repro, celda) in q.iter_mut() {
        let k = esp.0 as usize;
        let e = &cat.especies[k];
        let a = edad.0;
        if *sexo == Sexo::Macho {
            if repro.estado == EstadoRepro::Inmaduro && a >= e.a_rep {
                repro.estado = EstadoRepro::Maduro;
            }
            continue;
        }
        if repro.estado == EstadoRepro::Inmadura && a >= e.a_rep {
            repro.estado = if a >= e.a_fin_rep {
                EstadoRepro::Postreproductiva
            } else {
                EstadoRepro::Disponible
            };
        }
        match repro.estado {
            EstadoRepro::Gestante => {
                if res.0 < en.r_abort * e.e_max(masa.0) {
                    repro.estado = EstadoRepro::Disponible;
                    repro.dias_rest = 0;
                    libro.esp[k].abortos += 1;
                } else {
                    repro.dias_rest = repro.dias_rest.saturating_sub(1);
                    if repro.dias_rest == 0 {
                        parto(
                            e,
                            en.r_0,
                            semilla,
                            t,
                            id.0,
                            esp.0,
                            *celda,
                            &mut res,
                            &mut repro,
                            a,
                            nacimientos,
                            libro,
                        );
                        continue;
                    }
                }
            }
            EstadoRepro::Lactante => {
                repro.dias_rest = repro.dias_rest.saturating_sub(1);
                let desde = crias.partition_point(|&m| m < id.0);
                let vivas = crias[desde..].iter().take_while(|&&m| m == id.0).count();
                if repro.dias_rest == 0 {
                    repro.estado = EstadoRepro::Disponible;
                    libro.esp[k].destetes += 1;
                } else if vivas == 0 {
                    repro.estado = EstadoRepro::Disponible;
                    repro.dias_rest = 0;
                }
            }
            _ => {}
        }
        if repro.estado == EstadoRepro::Disponible {
            if a >= e.a_fin_rep {
                repro.estado = EstadoRepro::Postreproductiva;
            } else if machos[hab.idx(celda.0 as usize, k)] > 0 {
                let r = res.0 / e.e_max(masa.0);
                let p = e.p_conc * Especie::f_e(r, en.r_min, en.r_opt);
                if p > 0.0 && uniforme(semilla, id.0, t, Proposito::Concepcion, 0) < p {
                    repro.estado = EstadoRepro::Gestante;
                    repro.dias_rest = e.d_gest;
                    libro.esp[k].concepciones += 1;
                }
            }
        }
    }
}

/// A6 (final). Crea las crías del tick, en orden de id.
pub fn bio_nacimientos(world: &mut World) {
    let mut pendientes = std::mem::take(&mut world.resource_mut::<Buffers>().nacimientos);
    if !pendientes.is_empty() {
        let m0: Vec<f64> = world
            .resource::<Catalogo>()
            .especies
            .iter()
            .map(|e| e.m0)
            .collect();
        pendientes.sort_unstable_by_key(|c| c.id);
        for c in pendientes.drain(..) {
            let (sexo, estado) = if c.macho {
                (Sexo::Macho, EstadoRepro::Inmaduro)
            } else {
                (Sexo::Hembra, EstadoRepro::Inmadura)
            };
            world.spawn((
                IdBio(c.id),
                EspecieRef(c.esp),
                sexo,
                Edad(0),
                Masa(m0[c.esp as usize]),
                Reserva(c.reserva),
                Repro {
                    estado,
                    dias_rest: 0,
                    partos: 0,
                    ultimo_parto: None,
                },
                c.celda,
                Dependiente { madre: c.madre },
                Diario::default(),
            ));
        }
    }
    world.resource_mut::<Buffers>().nacimientos = pendientes;
}

type QMuerte<'a> = (
    Entity,
    &'a IdBio,
    &'a EspecieRef,
    &'a Sexo,
    &'a Edad,
    &'a Reserva,
    &'a Repro,
    Option<&'a Dependiente>,
);

/// A7. Mortalidad (§6.8), en orden: inanición (`E ≤ 0`), edad máxima, Siler y, si murió una madre
/// lactante, orfandad de sus crías (mueren con `p_huerf`; las demás pasan a comer solas).
pub fn bio_mortalidad(
    cat: Res<Catalogo>,
    tick: Res<Tick>,
    mut buf: ResMut<Buffers>,
    mut libro: ResMut<Libro>,
    q: Query<QMuerte>,
) {
    let t = tick.0;
    let semilla = cat.semilla;
    let Buffers {
        muertos,
        ids_muertos,
        madres_muertas,
        huerfanos,
        ..
    } = &mut *buf;
    let libro = &mut *libro;
    muertos.clear();
    ids_muertos.clear();
    madres_muertas.clear();
    huerfanos.clear();
    let mut anotar =
        |ent: Entity, id: u64, esp: u16, sexo: Sexo, edad: u32, causa: Causa, e: f64| {
            muertos.push((ent, id));
            libro.reservas_muertos.sumar(e);
            libro.esp[esp as usize].muertes[causa as usize] += 1;
            libro.muertos.push(MuertoInfo {
                esp,
                sexo,
                edad,
                causa,
                reserva: e,
            });
        };
    for (ent, id, esp, sexo, edad, res, repro, _) in q.iter() {
        let e = &cat.especies[esp.0 as usize];
        let causa = if res.0 <= 0.0 {
            Some(Causa::Inanicion)
        } else if edad.0 >= e.a_max {
            Some(Causa::EdadMax)
        } else if uniforme(semilla, id.0, t, Proposito::Mortalidad, 0) < e.p_muerte_dia(edad.0) {
            Some(Causa::Intrinseca)
        } else {
            None
        };
        if let Some(c) = causa {
            anotar(ent, id.0, esp.0, *sexo, edad.0, c, res.0);
            ids_muertos.push(id.0);
            if repro.estado == EstadoRepro::Lactante {
                madres_muertas.push(id.0);
            }
        }
    }
    if madres_muertas.is_empty() {
        return;
    }
    madres_muertas.sort_unstable();
    ids_muertos.sort_unstable();
    for (ent, id, esp, sexo, edad, res, _, dep) in q.iter() {
        let Some(dep) = dep else { continue };
        if madres_muertas.binary_search(&dep.madre).is_err()
            || ids_muertos.binary_search(&id.0).is_ok()
        {
            continue;
        }
        let p = cat.especies[esp.0 as usize].p_huerf;
        if uniforme(semilla, id.0, t, Proposito::Orfandad, 0) < p {
            anotar(ent, id.0, esp.0, *sexo, edad.0, Causa::Orfandad, res.0);
        } else {
            huerfanos.push(ent);
        }
    }
}

/// A7 (final). Retira los muertos y suelta a los huérfanos que sobreviven.
pub fn bio_aplicar_muertes(world: &mut World) {
    let (mut muertos, mut huerfanos) = {
        let mut b = world.resource_mut::<Buffers>();
        (
            std::mem::take(&mut b.muertos),
            std::mem::take(&mut b.huerfanos),
        )
    };
    muertos.sort_unstable_by_key(|x| x.1);
    for &(ent, _) in &muertos {
        let _ = world.despawn(ent);
    }
    for &ent in &huerfanos {
        if let Ok(mut e) = world.get_entity_mut(ent) {
            e.remove::<Dependiente>();
        }
    }
    muertos.clear();
    huerfanos.clear();
    let mut b = world.resource_mut::<Buffers>();
    b.muertos = muertos;
    b.huerfanos = huerfanos;
}

/// A8. Envejecimiento: edad + 1 día; las crías que cumplen `d_destete` dejan de depender (D10).
pub fn bio_envejecimiento(
    cat: Res<Catalogo>,
    mut commands: Commands,
    mut q: Query<(Entity, &EspecieRef, &mut Edad, Option<&Dependiente>)>,
) {
    for (ent, esp, mut edad, dep) in q.iter_mut() {
        edad.0 += 1;
        if dep.is_some() && edad.0 >= cat.especies[esp.0 as usize].d_destete {
            commands.entity(ent).remove::<Dependiente>();
        }
    }
}

/// A9. Escrituras al terreno (§6.10): el consumo de la celda se descuenta del stock.
pub fn bio_escrituras(cat: Res<Catalogo>, buf: Res<Buffers>, mut hab: ResMut<Habitat>) {
    let rho = cat.forraje.rho_veg;
    for c in 0..hab.n_celdas {
        for k in 0..hab.n_esp {
            let i = hab.idx(c, k);
            let x = buf.consumo[i].valor();
            if x > 0.0 {
                hab.v[i] = (hab.v[i] - x / (rho * hab.area_m2[c])).max(0.0);
            }
        }
    }
}
