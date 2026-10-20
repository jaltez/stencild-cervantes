#!/bin/sh
# orden 20 — pintar /mnt/u-disk/panel/imagen.4bpp en el panel (flujo runner v3:
# ebrmain no ha arrancado, no hay que matar nada). Guarda: una vez por arranque.
[ -e /tmp/orden20_done ] && exit 0
: > /tmp/orden20_done
P=/mnt/u-disk/panel
R="$P/resultado.txt"
[ -x "$P/epdraw" ] && [ -f "$P/imagen.4bpp" ] || { echo "orden20: faltan archivos en $P" >> "$R"; exit 0; }
"$P/epdraw" "$P/imagen.4bpp" >> "$R" 2>&1 \
	&& echo "orden20 OK $(date)" >> "$R" \
	|| echo "orden20 FAIL rc=$? (probar -i; o reconvertir con --rotate)" >> "$R"
exit 0
