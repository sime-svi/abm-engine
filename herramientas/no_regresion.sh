#!/usr/bin/env bash
# No regresión byte a byte frente a un commit anterior (CLAUDE §6 y §10).
#
# Compila el binario del commit de referencia en un worktree temporal, lo ejecuta con su config, ejecuta
# el binario actual con la suya (con los interruptores nuevos apagados, si los hay) y compara los dos
# JSON de salida byte a byte, salvo la línea del nombre de la corrida.
#
# Uso:
#   herramientas/no_regresion.sh REF BINARIO CONFIG_REF CONFIG_ACTUAL
#     REF            commit, rama o etiqueta de referencia (p. ej. f6e34e6)
#     BINARIO        agentes | terreno | abm
#     CONFIG_REF     config para el binario de referencia, como ruta dentro del repo en REF
#     CONFIG_ACTUAL  config para el binario actual (la misma, o una con los interruptores nuevos apagados)
#
# Ejemplos:
#   herramientas/no_regresion.sh main agentes config/agentes.toml config/agentes.toml
#   herramientas/no_regresion.sh 6031705 terreno config/terreno.toml config/experiments/terreno_sin_sombra.toml
#
# Sale con 0 si las salidas son idénticas y con 1 si no. Cada binario corre en su propia carpeta temporal
# (sus `output/` y sus cachés no se mezclan con las del repositorio).
set -euo pipefail

if [ $# -ne 4 ]; then
  sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'
  exit 2
fi
REF="$1"; BIN="$2"; CONFIG_REF="$3"; CONFIG_ACTUAL="$4"
RAIZ="$(git rev-parse --show-toplevel)"
CONFIG_ACTUAL="$(cd "$RAIZ" && realpath "$CONFIG_ACTUAL")"
TMP="$(mktemp -d)"
limpiar() { git -C "$RAIZ" worktree remove --force "$TMP/ref" 2>/dev/null || true; rm -rf "$TMP"; }
trap limpiar EXIT

echo "== Referencia: $REF ($BIN, $CONFIG_REF)"
git -C "$RAIZ" worktree add --detach --quiet "$TMP/ref" "$REF"
(cd "$TMP/ref" && cargo build --release --quiet --bin "$BIN")
(cd "$TMP/ref" && ./target/release/"$BIN" "$CONFIG_REF" > "$TMP/ref.log")

echo "== Actual: $(git -C "$RAIZ" rev-parse --short HEAD) ($BIN, ${CONFIG_ACTUAL#"$RAIZ"/})"
(cd "$RAIZ" && cargo build --release --quiet --bin "$BIN")
mkdir -p "$TMP/actual"
(cd "$TMP/actual" && "$RAIZ/target/release/$BIN" "$CONFIG_ACTUAL" > "$TMP/actual.log")

# La salida principal es output/<nombre>_<binario>.json (en el motor de forrajeo, la ruta de su config).
# El informe de tiempos (`_tiempos.json`) cambia en cada corrida y no forma parte de la salida.
salida() { ls -t "$1"/output/*.json 2>/dev/null | grep -v -e '_world\.json$' -e '_tiempos\.json$' | head -1; }
A="$(salida "$TMP/ref")"; B="$(salida "$TMP/actual")"
if [ -z "$A" ] || [ -z "$B" ]; then
  echo "No encuentro las salidas JSON (¿la config escribe en output/?)."; exit 2
fi
grep -v '^    "name": ' "$A" > "$TMP/a.json"
grep -v '^    "name": ' "$B" > "$TMP/b.json"
if cmp -s "$TMP/a.json" "$TMP/b.json"; then
  echo "IDÉNTICAS byte a byte (salvo el nombre): $(wc -c < "$A") bytes."
  exit 0
fi
echo "DIFERENTES. Primera diferencia:"
cmp "$TMP/a.json" "$TMP/b.json" || true
diff <(head -c 200000 "$TMP/a.json") <(head -c 200000 "$TMP/b.json") | head -20 || true
exit 1
