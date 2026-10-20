/* epdraw-fs — freestanding one-shot e-ink painter for bq Cervantes 2 (E60M00).
 * Raw-syscall build: no libc (the static-glibc build segfaults on kernel 2.6.28).
 *
 * usage: epdraw [-t] <image> [-i] [-W waveform]
 *   image: 240000 B packed-4bpp or 480000 B 8bpp gray, 600x800 portrait
 *   -t: built-in 16-bar test pattern; -i: invert; -W n: waveform (default 2)
 *   --sim:<path>: write the ioctl struct to <path> instead (host test)
 *
 * Build: arm-linux-gnueabi-gcc -march=armv5te -nostdlib -static -Os \
 *        -o epdraw epdraw.c -lgcc
 */
#include "epd_rt.h"

#define PANEL_W 800              /* DC del kernel = 800x600 paisaje (fake_s1d13522.c:809) */
#define PANEL_H 600
#define DATA_SZ (800 * 600)
#define IOCTL_S1D13521_DISPLAY 0x4539
#define O_RDWR 2
#define O_WCT 0x241

struct __attribute__((packed)) image_pgm {
	u32 StartX, StartY, Width, Height, WaveForm, LUT_NO;
	u8 Data[DATA_SZ];
};

static struct image_pgm img;
static u8 raw[DATA_SZ];

static u32 dec_atoi(const char *s, u32 def)
{
	u32 v = 0;
	if (!s || !*s) return def;
	while (*s >= '0' && *s <= '9') v = v * 10 + (u32)(*s++ - '0');
	return v;
}

int main(int argc, char **argv)
{
	const char *file = NULL, *sim = NULL;
	int i, invert = 0, testpat = 0, fb;
	u32 wf = 2, w, h;

	for (i = 1; i < argc; i++) {
		if (argv[i][0] == '-' && argv[i][1] != '\0') {
			if (rt_memcmp(argv[i], "--sim:", 6) == 0) sim = argv[i] + 6;
			else if (rt_streq(argv[i], "-i")) invert = 1;
			else if (rt_streq(argv[i], "-t")) testpat = 1;
			else if (rt_streq(argv[i], "-W") && i + 1 < argc) wf = dec_atoi(argv[++i], 2);
			/* unknown -flags (e.g. -f, -v) are ignored */
		} else {
			file = argv[i];
		}
	}
	if (!testpat && !file) { rt_puts(2, "usage: epdraw [-t] <img.4bpp> [-i] [-W n]\n"); s_exit(2); }

	img.Width = w = PANEL_W;
	img.Height = h = PANEL_H;
	img.WaveForm = wf;

	if (testpat) {
		u32 bx, by;
		for (by = 0; by < h; by++)
			for (bx = 0; bx < w; bx++) {
				u32 g = bx * 16 / w;
				if (invert) g = 15 - g;
				if (bx & 1)
					img.Data[(bx >> 1) + by * (w >> 1)] |= (u8)(g << 4);
				else
					img.Data[(bx >> 1) + by * (w >> 1)] |= (u8)g;
			}
	} else {
		int f = (int)s_open(file, 0 /*O_RDONLY*/, 0);
		long got;
		u32 d;
		if (f < 0) { rt_puts(2, "epdraw: cannot open file\n"); s_exit(1); }
		got = s_read(f, raw, (long)sizeof(raw));
		sys(6, f, 0, 0, 0, 0, 0);
		if (got == (long)(w * h)) {          /* 8bpp gray -> pack */
			for (d = 0; d < w * h / 2; d++) {
				u32 lo = (raw[2 * d] >> 4) & 0xf, hi = (raw[2 * d + 1] >> 4) & 0xf;
				if (invert) { lo = 15 - lo; hi = 15 - hi; }
				img.Data[d] = (u8)(hi << 4 | lo);
			}
		} else if (got >= (long)(w * h / 2)) { /* 4bpp packed */
			for (d = 0; d < w * h / 2; d++)
				img.Data[d] = invert
				    ? (u8)(((raw[d] >> 4) | (raw[d] << 4)) & 0xff)
				    : raw[d];
		} else {
			rt_puts(2, "epdraw: short file\n");
			s_exit(1);
		}
	}

	if (sim) {
		int o = (int)s_open(sim, O_WCT, 0644);
		if (o < 0) { rt_puts(2, "epdraw: sim write failed\n"); s_exit(1); }
		s_write(o, &img, (long)sizeof(img));
		sys(6, o, 0, 0, 0, 0, 0);
		rt_puts(1, "epdraw: simulated\n");
		s_exit(0);
	}

	fb = (int)s_open("/dev/fb0", O_RDWR, 0);
	if (fb < 0) { rt_puts(2, "epdraw: cannot open /dev/fb0\n"); s_exit(1); }
	if (s_ioctl(fb, IOCTL_S1D13521_DISPLAY, (long)&img) < 0) {
		rt_puts(2, "epdraw: ioctl DISPLAY failed\n");
		sys(6, fb, 0, 0, 0, 0, 0);
		s_exit(1);
	}
	sys(6, fb, 0, 0, 0, 0, 0);
	rt_puts(1, "epdraw: displayed\n");
	s_exit(0);
	return 0;
}
