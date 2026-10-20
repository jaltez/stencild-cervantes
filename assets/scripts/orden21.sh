#!/bin/sh
# orden 21 — panel e-ink EN VIVO por USB (runner v3: ebrmain no arranca, UDC libre).
# v2: busybox-safe (sin `grep -w`, que busybox 1.15 no soporta).
P=/mnt/u-disk/panel/usb
R="$P/resultado.txt"
echo "orden21 inicio $(date)" > "$R" 2>/dev/null || true
[ -f "$P/g_serial.ko" ] && [ -x "$P/stencild" ] || { echo "orden21: faltan archivos en $P" >> "$R"; exit 0; }

rmmod g_file_storage 2>/dev/null
insmod "$P/g_serial.ko" >> "$R" 2>&1 || { echo "orden21: insmod g_serial FAIL" >> "$R"; dmesg | tail -5 >> "$R"; exit 0; }
echo "orden21: g_serial cargado $(date)" >> "$R"

i=0
while [ $i -lt 10 ] && [ ! -e /dev/ttyGS0 ]; do sleep 1; i=$((i+1)); done
if [ ! -e /dev/ttyGS0 ]; then
	maj=$(awk '$2=="ttyGS"{print $1}' /proc/devices)
	if [ -n "$maj" ]; then
		mknod /dev/ttyGS0 c "$maj" 0 2>>"$R"
		echo "orden21: mknod ttyGS0 major=$maj" >> "$R"
	else
		echo "orden21: ttyGS no esta en /proc/devices" >> "$R"
		dmesg | tail -10 >> "$R"
		exit 0
	fi
fi
"$P/stencild" /dev/ttyGS0 > "$P/stencild.log" 2>&1 &
echo "orden21: stencild activo pid $! $(date)" >> "$R"
sleep 5
echo "--- dmesg tras configuración USB:" >> "$R"
dmesg | tail -15 >> "$R"
exit 0
