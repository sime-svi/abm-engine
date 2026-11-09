#!/usr/bin/env bash
# Descarga las fuentes del modo Tierra (docs/design/terreno/modo-tierra-diseno.md §1.3) a
# output/referencias/ (ignorado por git). Solo baja lo que falta; se puede repetir sin miedo.
#
#   ETOPO 2022 (NOAA NCEI, dominio público): superficie y roca cada 2', por OPeNDAP (lo hace
#     herramientas/analisis/referencia_tierra.py; aquí solo los tres archivos que usa el motor).
#   CHELSA V2.1 1981–2010 (CC0 1.0): 24 GeoTIFF mensuales (tas y pr), ~4,3 GB.
#   CRU TS 4.09 (Open Government Licence v3): tmp y pre 1901–2024, NetCDF-3 con gzip, ~1,1 GB.
#   HydroSHEDS v1 a 15″ (licencia de HydroSHEDS: uso científico, educativo y comercial, con cita):
#     direcciones de flujo (308 MB), acumulación (818 MB) y máscara (45 MB) globales; la red de ríos
#     del modo Tierra. La máscara distingue el fondo de una cuenca cerrada de la salida al mar: en las
#     direcciones de la versión 1.1 los dos valen 0.
#   HydroLAKES v1.0 (CC BY 4.0), polígonos, 820 MB: los lagos.
#   HydroRIVERS v1.0 (licencia de HydroSHEDS), ~0,5 GB: solo para validar los ríos.
#   RGI 7.0 (CC BY 4.0), tabla global de atributos (centro y área de cada glaciar), 75 MB, de la
#     réplica de OGGM (Universidad de Bremen); NSIDC pide cuenta de Earthdata.
#   MODIS MOD17A3HGF v6.1 a 0,5° del ICDC (CC BY 4.0), 48 MB: solo para validar la vegetación.
#
# El servidor de datos de HydroSHEDS está detrás de Cloudflare: durante los partidos de LaLiga los
# operadores españoles bloquean esas direcciones. Fuera de esas horas baja sin problema.
#
# Uso: herramientas/tierra/descargar.sh
set -euo pipefail
RAIZ="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
REF="$RAIZ/output/referencias"
mkdir -p "$REF/chelsa_mensual" "$REF/cru_ts" "$REF/hydrosheds" "$REF/modis_npp" "$REF/rgi7"

bajar() {  # bajar URL DESTINO: con reintentos y sin repetir lo que ya está completo
  local url="$1" dest="$2" tam
  tam="$(curl -sIL "$url" | tr -d '\r' | awk 'tolower($1)=="content-length:"{t=$2} END{print t+0}')"
  if [ -f "$dest" ] && [ "$tam" -gt 0 ] && [ "$(stat -c %s "$dest")" = "$tam" ]; then
    echo "  ya está: $(basename "$dest")"; return
  fi
  echo "  bajando $(basename "$dest") ($((tam / 1000000)) MB)"
  curl -sL --retry 5 -C - -o "$dest" "$url"
}

echo "ETOPO 2022 (superficie y franjas polares de la roca, cada 2')"
python3 -I - "$RAIZ" "$REF" <<'PY'
import os, sys
sys.path.insert(0, os.path.join(sys.argv[1], 'herramientas', 'analisis'))
import referencia_tierra as rt
d = lambda n: os.path.join(sys.argv[2], n)
rt.etopo(rt.ETOPO_SUP, d('etopo2022_superficie_2m.f32be'), 0, rt.FILAS)
rt.etopo(rt.ETOPO_ROCA, d('etopo2022_roca_sur_2m.f32be'), 0, 30 * 60)
rt.etopo(rt.ETOPO_ROCA, d('etopo2022_roca_norte_2m.f32be'), 149 * 60, rt.FILAS)
PY

echo "CHELSA V2.1, climatologías mensuales 1981–2010"
for v in tas pr; do
  for m in 01 02 03 04 05 06 07 08 09 10 11 12; do
    bajar "https://os.zhdk.cloud.switch.ch/chelsav2/GLOBAL/climatologies/1981-2010/$v/CHELSA_${v}_${m}_1981-2010_V.2.1.tif" \
      "$REF/chelsa_mensual/CHELSA_${v}_${m}_1981-2010_V.2.1.tif"
  done
done

echo "CRU TS 4.09"
for v in tmp pre; do
  bajar "https://crudata.uea.ac.uk/cru/data/hrg/cru_ts_4.09/cruts.2503051245.v4.09/$v/cru_ts4.09.1901.2024.$v.dat.nc.gz" \
    "$REF/cru_ts/cru_ts4.09.1901.2024.$v.dat.nc.gz"
done

echo "HydroSHEDS v1 a 15″ (direcciones, acumulación y máscara), HydroLAKES y HydroRIVERS"
for a in hydrosheds-v1-dir/hyd_glo_dir_15s.zip hydrosheds-v1-acc/hyd_glo_acc_15s.zip \
         hydrosheds-v1-msk/hyd_glo_msk_15s.zip \
         hydrolakes/HydroLAKES_polys_v10_shp.zip HydroRIVERS/HydroRIVERS_v10_shp.zip; do
  bajar "https://data.hydrosheds.org/file/$a" "$REF/hydrosheds/$(basename "$a")" \
    || echo "  AVISO: no se pudo bajar $(basename "$a") (¿el bloqueo de Cloudflare durante un partido?)"
done
# Se descomprimen el GeoTIFF de cada zip de HydroSHEDS y el shapefile de HydroLAKES.
python3 -I - "$REF/hydrosheds" <<'PY'
import os, sys, zipfile
d = sys.argv[1]
for zipname in ('hyd_glo_dir_15s.zip', 'hyd_glo_acc_15s.zip', 'hyd_glo_msk_15s.zip', 'HydroLAKES_polys_v10_shp.zip'):
    ruta = os.path.join(d, zipname)
    if not os.path.exists(ruta):
        continue
    with zipfile.ZipFile(ruta) as z:
        for i in z.infolist():
            if i.filename.endswith('/'):
                continue
            dest = os.path.join(d, i.filename)
            if not os.path.exists(dest) or os.path.getsize(dest) != i.file_size:
                print('  descomprimiendo', i.filename)
                z.extract(i, d)
PY

echo "RGI 7.0 (tabla global de atributos)"
bajar "https://cluster.klima.uni-bremen.de/~oggm/rgi/RGI2000-v7.0-G-global-attributes.csv" \
  "$REF/rgi7/RGI2000-v7.0-G-global-attributes.csv"

echo "MODIS NPP a 0,5° (ICDC; validación de la vegetación)"
bajar "https://www.fdr.uni-hamburg.de/record/14635/files/MODIS-C061_MOD17A3HGF__NPP_GPP__Yearly__LPDAAC__GLOBAL_0.5degree__UHAM-ICDC__2001-2023__fv0.01.nc.zip?download=1" \
  "$REF/modis_npp/modis_npp_05.zip"
echo "Hecho: fuentes en $REF"
