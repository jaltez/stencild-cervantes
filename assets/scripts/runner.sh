#!/bin/sh
# NetronixInc_MFG.sh — runner v3.1 (mínimo, para la SD del panel)
# Contrato (ver /bin/g del aparato): si este archivo existe en la raíz de la SD
# (/mnt/u-disk), se ejecuta COMO ROOT en cada arranque y ebrmain NO arranca.
# v3.1: guarda /tmp para ejecutarse UNA sola vez por arranque aunque /bin/g
#       se relance, y las órdenes llevan su propia guarda.
# Ejecuta ordenes/[0-9]*.sh en orden léxico (los .hecho se saltan).
# Si existe /mnt/u-disk/LEER, arranca ebrmain al terminar (modo lector).
L=/mnt/u-disk/runner.log
if [ -e /tmp/panel_runner_done ]; then
	echo "== runner v3.1 ya ejecutado este arranque, fuera" >> $L
	exit 0
fi
: > /tmp/panel_runner_done
echo "== runner v3.1 $(date)" >> $L
uname -a >> $L
for f in /mnt/u-disk/ordenes/[0-9]*.sh; do
	[ -e "$f" ] || continue
	echo "-- orden $(basename $f) inicio $(date)" >> $L
	sh "$f" >> $L 2>&1
	echo "-- orden $(basename $f) fin rc=$?" >> $L
done
if [ -e /mnt/u-disk/LEER ]; then
	echo "== modo lector (LEER presente): arrancando ebrmain" >> $L
	ebrmain > /dev/console &
else
	echo "== modo panel: sin ebrmain (crear LEER para volver al lector)" >> $L
fi
sync
exit 0
