/* inkd-fs — freestanding live e-ink push daemon for bq Cervantes 2 (E60M00).
 * Raw-syscall build: no libc, immune to glibc-vs-kernel-2.6.28 issues.
 *
 * Protocol (LE, raw tty): "EPD1" | u32 size | u8 waveform | u8 invert | payload
 * size==0 -> ping. Replies: "OK\n" "PONG\n" "ERR ...\n".
 * Simulation: argv of the form "--sim:<path>" writes the ioctl struct to <path>
 * instead of touching /dev/fb0 (host-side QEMU test).
 *
 * Build: arm-linux-gnueabi-gcc -march=armv5te -nostdlib -static -Os \
 *        -o inkd inkd.c -lgcc
 */
#include "epd_rt.h"

#define PANEL_W 800              /* DC del kernel = 800x600 paisaje (fake_s1d13522.c:809) */
#define PANEL_H 600
#define DATA_SZ (800 * 600)
#define IOCTL_S1D13521_DISPLAY 0x4539
#define HDR_SZ 10
#define TCSETS 0x5402
#define O_RDWR 2
#define O_WCT 0x241 /* O_WRONLY|O_CREAT|O_TRUNC */

struct raw_tio {                 /* kernel struct termios, raw settings */
	u32 c_iflag, c_oflag, c_cflag, c_lflag;
	u8 c_line;
	u8 c_cc[19];
};

struct __attribute__((packed)) image_pgm {
	u32 StartX, StartY, Width, Height, WaveForm, LUT_NO;
	u8 Data[DATA_SZ];
};

static struct image_pgm img;     /* .bss; kernel ELF loader zeroes it */

static int readn(int fd, void *buf, long n)
{
	u8 *p = buf;
	long got = 0;
	while (got < n) {
		long r = s_read(fd, p + got, n - got);
		if (r <= 0) return -1;
		got += r;
	}
	return 0;
}

static int writen(int fd, const char *buf, long n)
{
	long put = 0;
	while (put < n) {
		long w = s_write(fd, buf + put, n - put);
		if (w <= 0) return -1;
		put += w;
	}
	return 0;
}

int main(int argc, char **argv)
{
	const char *port = "/dev/ttyGS0";
	const char *sim = NULL;
	int i, tty, fb = -1;
	struct raw_tio tio;

	for (i = 1; i < argc; i++) {
		if (rt_streq(argv[i] + 0, argv[i]) && argv[i][0] == '-' &&
		    rt_strlen(argv[i]) > 6 && rt_memcmp(argv[i], "--sim:", 6) == 0)
			sim = argv[i] + 6;
		else
			port = argv[i];
	}

	tty = (int)s_open(port, O_RDWR, 0);
	if (tty < 0) {
		rt_puts(2, "inkd: cannot open ");
		rt_puts(2, port);
		rt_puts(2, "\n");
		s_exit(1);
	}

	for (i = 0; i < (int)sizeof(tio); i++) ((u8 *)&tio)[i] = 0;
	tio.c_cflag = 0x000002bd;   /* CS8|CREAD|CLOCAL */
	tio.c_cc[6 /*VMIN*/] = 1;
	tio.c_cc[5 /*VTIME*/] = 0;
	s_ioctl(tty, TCSETS, (long)&tio);

	if (!sim) {
		fb = (int)s_open("/dev/fb0", O_RDWR, 0);
		if (fb < 0) { rt_puts(2, "inkd: cannot open /dev/fb0\n"); s_exit(1); }
	}
	rt_puts(2, "inkd: ready\n");

	for (;;) {
		u8 hdr[HDR_SZ];
		u32 size, j;
		u8 wf, inv;

		if (readn(tty, hdr, HDR_SZ) < 0) {
			/* USB desconectado (o error del tty): reabrir y seguir.
			 * La imagen queda en pantalla (e-ink) y al reconectar el
			 * host vuelve a tener el canal sin reiniciar nada. */
			if (sim) break;
			rt_puts(2, "inkd: tty perdido, esperando reconexion\n");
			sys(6, tty, 0, 0, 0, 0, 0);
			for (;;) {
				s_sleep(2);
				tty = (int)s_open(port, O_RDWR, 0);
				if (tty >= 0) break;
			}
			s_ioctl(tty, TCSETS, (long)&tio);
			rt_puts(2, "inkd: reconectado\n");
			continue;
		}
		if (rt_memcmp(hdr, "EPD1", 4) != 0) {
			writen(tty, "ERR bad magic\n", 14);
			continue;
		}
		size = hdr[4] | hdr[5] << 8 | hdr[6] << 16 | (u32)hdr[7] << 24;
		wf = hdr[8]; inv = hdr[9];

		if (size == 0) { writen(tty, "PONG\n", 5); continue; }
		if (size > PANEL_W * PANEL_H / 2) {
			writen(tty, "ERR size\n", 9);
			while (size--) { u8 sink; if (readn(tty, &sink, 1) < 0) goto out; }
			continue;
		}
		if (readn(tty, img.Data, size) < 0) break;

		if (inv)
			for (j = 0; j < size; j++)
				img.Data[j] = (u8)(((img.Data[j] >> 4) | (img.Data[j] << 4)) & 0xff);

		img.StartX = 0; img.StartY = 0;
		img.Width = PANEL_W; img.Height = PANEL_H;
		img.WaveForm = wf; img.LUT_NO = 0;

		if (sim) {
			int o = (int)s_open(sim, O_WCT, 0644);
			if (o >= 0) {
				s_write(o, &img, (long)sizeof(img));
				sys(6 /*close*/, o, 0, 0, 0, 0, 0);
			}
			writen(tty, "OK\n", 3);
			continue;
		}
		{
			long rc = s_ioctl(fb, IOCTL_S1D13521_DISPLAY, (long)&img);
			if (rc < 0) {
				char msg[40];
				const char *pre = "ERR ioctl ";
				int n = 0;
				while (pre[n]) { msg[n] = pre[n]; n++; }
				n += rt_dec(msg + n, rc);
				msg[n++] = '\n';
				writen(tty, msg, n);
			} else {
				writen(tty, "OK\n", 3);
			}
		}
	}
out:
	s_exit(0);
	return 0;
}
