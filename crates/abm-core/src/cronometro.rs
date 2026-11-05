//! Cronómetro de rendimiento por sistema (punto 5 de escalabilidad: saber qué sistema se lleva el
//! tiempo antes de que haya cien).
//!
//! CLAUDE §6: el reloj de pared solo mide el rendimiento. Lo que anota este cronómetro no lo lee
//! ninguna regla y no va a la salida del modelo, sino a un informe aparte (`<nombre>_tiempos.json`),
//! así que la salida sigue siendo idéntica byte a byte entre corridas.

use std::time::{Duration, Instant};

/// Tiempo acumulado de una etapa (un sistema o un grupo de pasos).
#[derive(Debug, Clone)]
pub struct Etapa {
    pub nombre: &'static str,
    pub total: Duration,
    pub veces: u64,
}

/// Acumula el tiempo de cada etapa, en el orden en que aparecen por primera vez.
#[derive(Debug, Clone)]
pub struct Cronometro {
    marca: Instant,
    etapas: Vec<Etapa>,
}

impl Default for Cronometro {
    fn default() -> Self {
        Self {
            marca: Instant::now(),
            etapas: Vec::new(),
        }
    }
}

impl Cronometro {
    /// Pone la marca en este instante: el principio de lo siguiente que se mida.
    pub fn marcar(&mut self) {
        self.marca = Instant::now();
    }

    /// Suma a la etapa `nombre` el tiempo desde la última marca y pone la marca aquí.
    pub fn anotar(&mut self, nombre: &'static str) {
        let ahora = Instant::now();
        let dt = ahora.saturating_duration_since(self.marca);
        self.marca = ahora;
        match self.etapas.iter_mut().find(|e| e.nombre == nombre) {
            Some(e) => {
                e.total += dt;
                e.veces += 1;
            }
            None => self.etapas.push(Etapa {
                nombre,
                total: dt,
                veces: 1,
            }),
        }
    }

    /// Las etapas medidas, en el orden en que aparecieron por primera vez.
    pub fn etapas(&self) -> &[Etapa] {
        &self.etapas
    }

    /// Tiempo total medido (la suma de todas las etapas).
    pub fn total(&self) -> Duration {
        self.etapas.iter().map(|e| e.total).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acumula_por_etapa_en_orden_de_aparicion() {
        let mut c = Cronometro::default();
        for _ in 0..3 {
            c.marcar();
            c.anotar("b");
            c.anotar("a");
        }
        let nombres: Vec<_> = c.etapas().iter().map(|e| (e.nombre, e.veces)).collect();
        assert_eq!(nombres, vec![("b", 3), ("a", 3)]);
        assert_eq!(c.total(), c.etapas()[0].total + c.etapas()[1].total);
    }
}
