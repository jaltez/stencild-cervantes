/* epd_rt.h — freestanding runtime for the Cervantes 2 (kernel 2.6.28, ARMv5TE).
 * Raw EABI syscalls only: no libc, no glibc-vs-old-kernel startup issues.
 * Programs using this never return from main(); the kernel zeroes .bss.
 */
#ifndef EPD_RT_H
#define EPD_RT_H

typedef unsigned int u32;
typedef unsigned char u8;

#define NULL ((void *)0)

/* ---- syscalls (numbers from asm/unistd.h, ARM EABI) ---- */
#define __NR_exit 1
#define __NR_read 3
#define __NR_write 4
#define __NR_open 5
#define __NR_close 6
#define __NR_ioctl 54

static long sys(long n, long a, long b, long c, long d, long e, long f)
{
	register long r7 __asm__("r7") = n;
	register long r0 __asm__("r0") = a;
	register long r1 __asm__("r1") = b;
	register long r2 __asm__("r2") = c;
#define __NR_nanosleep 162
	register long r3 __asm__("r3") = d;
	register long r4 __asm__("r4") = e;
	register long r5 __asm__("r5") = f;
	__asm__ volatile("svc 0"
			 : "+r"(r0)
			 : "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5), "r"(r7)
			 : "memory");
	return r0;
}

static long s_open(const char *p, int flags, int mode) { return sys(__NR_open, (long)p, flags, mode, 0, 0, 0); }
static long s_read(int fd, void *b, long n) { return sys(__NR_read, fd, (long)b, n, 0, 0, 0); }
static long s_write(int fd, const void *b, long n) { return sys(__NR_write, fd, (long)b, n, 0, 0, 0); }
static long s_ioctl(int fd, unsigned long req, long arg) { return sys(__NR_ioctl, fd, (long)req, arg, 0, 0, 0); }
static void s_exit(int code) { sys(__NR_exit, code, 0, 0, 0, 0, 0); for (;;) ; }

static void s_sleep(int seg)
{
	struct { long sec; long nsec; } ts = { seg, 0 };
	sys(__NR_nanosleep, (long)&ts, 0, 0, 0, 0, 0);
}

/* public names gcc/libgcc may call even freestanding */
void *memset(void *d, int c, unsigned long n)
{
	u8 *p = d;
	while (n--) *p++ = (u8)c;
	return d;
}

void *memcpy(void *d, const void *s, unsigned long n)
{
	u8 *dd = d; const u8 *ss = s;
	while (n--) *dd++ = *ss++;
	return d;
}

int raise(int sig)
{
	sys(__NR_exit, 128 + sig, 0, 0, 0, 0, 0);
	return 0;
}

/* ---- tiny libc bits ---- */
static void *rt_memcpy(void *d, const void *s, long n)
{
	u8 *dd = d; const u8 *ss = s;
	while (n--) *dd++ = *ss++;
	return d;
}

static int rt_memcmp(const void *a, const void *b, long n)
{
	const u8 *x = a, *y = b;
	while (n--) { if (*x != *y) return *x - *y; x++; y++; }
	return 0;
}

static int rt_streq(const char *a, const char *b)
{
	while (*a && *a == *b) { a++; b++; }
	return *a == *b;
}

static long rt_strlen(const char *s) { long n = 0; while (*s++) n++; return n; }

static void rt_puts(int fd, const char *s) { s_write(fd, s, rt_strlen(s)); }

/* decimal/hex into buf, returns length (buf big enough) */
static int rt_dec(char *buf, long v)
{
	char t[12]; int n = 0, i;
	unsigned long u = v < 0 ? -(unsigned long)v : (unsigned long)v;
	if (v < 0) *buf++ = '-';
	do { t[n++] = (char)('0' + u % 10); u /= 10; } while (u);
	for (i = 0; i < n; i++) buf[i] = t[n - 1 - i];
	return n + (v < 0 ? 1 : 0);
}

/* ---- entry ---- */
extern int main(int argc, char **argv);

__asm__(
".global _start\n"
"_start:\n"
"	ldr r0, [sp]\n"      /* argc */
"	add r1, sp, #4\n"    /* argv */
"	mov r2, #0\n"        /* envp (unused) */
"	bic sp, sp, #7\n"    /* align */
"	b main\n"
);

#endif /* EPD_RT_H */
