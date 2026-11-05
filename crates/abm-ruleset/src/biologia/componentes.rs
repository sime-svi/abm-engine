//! Componentes de la entidad `Individuo` (diseño de biología §3). Datos puros en tablas de columnas
//! (ECS): nada de objetos con comportamiento.

use bevy_ecs::prelude::*;

/// Identificador determinista (§6.11).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdBio(pub u64);

/// Especie (índice en el catálogo).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct EspecieRef(pub u16);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sexo {
    Hembra,
    Macho,
}

/// Edad `a` (días).
#[derive(Component, Debug, Clone, Copy)]
pub struct Edad(pub u32);

/// Masa `m` (kg).
#[derive(Component, Debug, Clone, Copy)]
pub struct Masa(pub f64);

/// Reserva energética `E` (kcal, extensivo).
#[derive(Component, Debug, Clone, Copy)]
pub struct Reserva(pub f64);

/// Estado reproductivo (§4): hembras y machos tienen máquinas distintas (§6.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoRepro {
    Inmadura,
    Disponible,
    Gestante,
    Lactante,
    Postreproductiva,
    Inmaduro,
    Maduro,
}

/// Máquina de estados reproductiva + contadores.
#[derive(Component, Debug, Clone, Copy)]
pub struct Repro {
    pub estado: EstadoRepro,
    /// Días hasta el parto (gestante) o el destete (lactante).
    pub dias_rest: u32,
    /// Partos hasta ahora (forma parte del id de las crías).
    pub partos: u32,
    /// Tick del último parto (intervalo entre partos).
    pub ultimo_parto: Option<u64>,
}

impl Repro {
    /// Estado inicial por sexo y edad (§6.0), sin gestación ni lactancia.
    pub fn por_edad(macho: bool, a: u32, a_rep: u32, a_fin_rep: u32) -> Repro {
        let estado = match (macho, a >= a_rep, a >= a_fin_rep) {
            (true, false, _) => EstadoRepro::Inmaduro,
            (true, true, _) => EstadoRepro::Maduro,
            (false, false, _) => EstadoRepro::Inmadura,
            (false, true, false) => EstadoRepro::Disponible,
            (false, true, true) => EstadoRepro::Postreproductiva,
        };
        Repro {
            estado,
            dias_rest: 0,
            partos: 0,
            ultimo_parto: None,
        }
    }
}

/// Cría dependiente de una madre lactante (`id` de la madre).
#[derive(Component, Debug, Clone, Copy)]
pub struct Dependiente {
    pub madre: u64,
}

/// Celda del hábitat donde está.
#[derive(Component, Debug, Clone, Copy)]
pub struct Celda(pub u32);

/// Cuentas del día de cada individuo (se rehacen cada tick).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Diario {
    /// Crecimiento previsto hoy (kg), decidido con la condición de partida (D2).
    pub dm: f64,
    /// Gasto propio: mantenimiento y actividad + crecimiento + gestación (kcal).
    pub g_propio: f64,
    /// Leche pagada a sus crías, `Σ φ·G_c / ε_leche` (kcal).
    pub g_leche: f64,
    /// Leche recibida de su madre, `φ·G` (kcal).
    pub leche: f64,
    /// Demanda de forraje bruto (kcal) y lo que le toca en el reparto.
    pub demanda: f64,
    pub ingesta: f64,
    /// `true` si hoy gastó más de lo que ingirió.
    pub deficit: bool,
}
