#!/usr/bin/env bash
# Verificación de una tarea antes de darla por hecha (CLAUDE §10), en un solo comando:
#   1. formato (`cargo fmt --check`) y linter (`cargo clippy -- -D warnings`);
#   2. tests (`cargo test --workspace`, con los de determinismo);
#   3. batería de escenarios de referencia (`herramientas/escenarios.py`);
#   4. opcional: no regresión byte a byte de `agentes` y `terreno` frente a un commit, para los cambios
#      que no deben alterar la salida (`herramientas/no_regresion.sh`).
#
# Uso:
#   herramientas/verificar.sh                   # 1 a 3
#   herramientas/verificar.sh --no-regresion main   # 1 a 4, comparando con `main`
#
# Los documentos vivos (CLAUDE §8) no se pueden comprobar solos: al final lo recuerda.
set -euo pipefail
RAIZ="$(git rev-parse --show-toplevel)"
cd "$RAIZ"
REF=""
if [ "${1:-}" = "--no-regresion" ]; then
  REF="${2:?falta el commit de referencia}"
elif [ $# -gt 0 ]; then
  sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
fi

paso() { printf '\n== %s\n' "$1"; }
paso "1/4 Formato y clippy"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --quiet -- -D warnings
paso "2/4 Tests"
LOG="$(mktemp)"
trap 'rm -f "$LOG"' EXIT
if cargo test --workspace --quiet > "$LOG" 2>&1; then
  awk '/^test result:/ { p += $4; i += $8 } END { printf "%d tests en verde (%d ignorados)\n", p, i }' "$LOG"
else
  grep -E "FAILED|panicked|^error" -A6 "$LOG" | head -60
  echo "Hay tests en rojo: cargo test --workspace"
  exit 1
fi
paso "3/4 Escenarios de referencia"
python3 herramientas/escenarios.py
if [ -n "$REF" ]; then
  paso "4/4 No regresión frente a $REF"
  herramientas/no_regresion.sh "$REF" agentes config/agentes.toml config/agentes.toml
  herramientas/no_regresion.sh "$REF" terreno config/terreno.toml config/terreno.toml
else
  paso "4/4 No regresión: omitida (--no-regresion REF para compararla)"
fi
printf '\nTodo en verde. Falta lo que no se comprueba solo: MEMORY.md, TREE.md, DATABASE_SCHEMA.md y las notas (CLAUDE §8).\n'
