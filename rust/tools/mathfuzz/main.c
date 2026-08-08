// Differential fuzzer: extra/ufbx_math.c vs the Rust port (native/math.rs,
// exported as rs_* by this directory's staticlib). Every result must match
// BIT-FOR-BIT — the scene-hash oracle depends on it.
//
// Deterministic (fixed-seed splitmix64), so a failure reproduces exactly.
// Build & run: see ci.sh next to this file.

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../../../extra/ufbx_math.h"

// In ufbx_math.c but not its public header (extra/ufbx_math.c:142-143).
double ufbx_scalbn(double x, int n);
double ufbx_frexp(double x, int *eptr);

// Rust exports (rust/tools/mathfuzz/src/lib.rs)
double rs_fabs(double);
double rs_floor(double);
double rs_ceil(double);
double rs_rint(double);
double rs_sqrt(double);
double rs_sin(double);
double rs_cos(double);
double rs_tan(double);
double rs_asin(double);
double rs_acos(double);
double rs_atan(double);
double rs_copysign(double, double);
double rs_atan2(double, double);
double rs_pow(double, double);
double rs_fmin(double, double);
double rs_fmax(double, double);
double rs_nextafter(double, double);
double rs_scalbn(double, int);
double rs_frexp(double, int *);
int rs_isnan(double);

static uint64_t rng_state;
static uint64_t rng_next(void) {
	uint64_t z = (rng_state += 0x9e3779b97f4a7c15ull);
	z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ull;
	z = (z ^ (z >> 27)) * 0x94d049bb133111ebull;
	return z ^ (z >> 31);
}

static double bits_to_f64(uint64_t b) {
	double d;
	memcpy(&d, &b, 8);
	return d;
}
static uint64_t f64_to_bits(double d) {
	uint64_t b;
	memcpy(&b, &d, 8);
	return b;
}

// Realistic-range sample: the value classes ufbx actually feeds these
// functions (angles, unit-range trig inputs, degree scales, huge Payne-Hanek
// arguments, tiny epsilons).
static double rng_realistic(void) {
	double u = (double)(rng_next() >> 11) * (1.0 / 9007199254740992.0); // [0,1)
	double s = (rng_next() & 1) ? -1.0 : 1.0;
	switch (rng_next() % 6u) {
	case 0: return s * u * 12.566370614359172; // +-4pi
	case 1: return s * u;                      // +-1 (asin/acos domain)
	case 2: return s * u * 360.0;              // degrees
	case 3: return s * u * 2e18;               // Payne-Hanek territory
	case 4: return s * u * 1e6;
	default: return s * u * 1e-8;
	}
}

static const uint64_t edge_bits[] = {
	0x0000000000000000ull, 0x8000000000000000ull, // +-0
	0x0000000000000001ull, 0x8000000000000001ull, // +-min subnormal
	0x000fffffffffffffull, 0x800fffffffffffffull, // +-max subnormal
	0x0010000000000000ull, 0x8010000000000000ull, // +-DBL_MIN
	0x3ff0000000000000ull, 0xbff0000000000000ull, // +-1
	0x3fe0000000000000ull, 0xbfe0000000000000ull, // +-0.5
	0x3ff8000000000000ull, 0xbff8000000000000ull, // +-1.5
	0x400921fb54442d18ull, 0xc00921fb54442d18ull, // +-pi
	0x3ff921fb54442d18ull, 0xbff921fb54442d18ull, // +-pi/2
	0x401921fb54442d18ull, 0xc01921fb54442d18ull, // +-2pi
	0x4000000000000000ull, 0xc000000000000000ull, // +-2
	0x4024000000000000ull, 0xc024000000000000ull, // +-10
	0x4059000000000000ull, 0xc059000000000000ull, // +-100
	0x43abc16d674ec800ull, 0xc3abc16d674ec800ull, // +-1e18
	0x44b52d02c7e14af6ull, 0xc4b52d02c7e14af6ull, // +-1e100
	0x7fe1ccf385ebc8a0ull, 0xffe1ccf385ebc8a0ull, // +-1e308
	0x7fefffffffffffffull, 0xffefffffffffffffull, // +-DBL_MAX
	0x7ff0000000000000ull, 0xfff0000000000000ull, // +-inf
	0x7ff8000000000000ull, 0xfff8000000000000ull, // +-qNaN
	0x7ff0000000000001ull,                        // sNaN
	0x7ff4deadbeef1234ull,                        // NaN payload
	0x3cb0000000000000ull, 0xbcb0000000000000ull, // +-2^-52
	0x3e70000000000000ull,                        // 2^-24
	0x4340000000000000ull,                        // 2^53
};
enum { NUM_EDGES = sizeof(edge_bits) / sizeof(edge_bits[0]) };

static long num_checked, num_failed;

static void check1(const char *name, double (*cf)(double), double (*rf)(double), double x) {
	uint64_t cb = f64_to_bits(cf(x)), rb = f64_to_bits(rf(x));
	num_checked++;
	if (cb != rb && num_failed++ < 10) {
		fprintf(stderr, "MISMATCH %s(%016llx): C %016llx Rust %016llx\n", name,
			(unsigned long long)f64_to_bits(x), (unsigned long long)cb, (unsigned long long)rb);
	}
}

static void check2(const char *name, double (*cf)(double, double), double (*rf)(double, double),
	double a, double b) {
	uint64_t cb = f64_to_bits(cf(a, b)), rb = f64_to_bits(rf(a, b));
	num_checked++;
	if (cb != rb && num_failed++ < 10) {
		fprintf(stderr, "MISMATCH %s(%016llx, %016llx): C %016llx Rust %016llx\n", name,
			(unsigned long long)f64_to_bits(a), (unsigned long long)f64_to_bits(b),
			(unsigned long long)cb, (unsigned long long)rb);
	}
}

typedef struct { const char *name; double (*cf)(double); double (*rf)(double); } fn1;
typedef struct { const char *name; double (*cf)(double, double); double (*rf)(double, double); } fn2;

static const fn1 fns1[] = {
	{ "fabs", ufbx_fabs, rs_fabs }, { "floor", ufbx_floor, rs_floor },
	{ "ceil", ufbx_ceil, rs_ceil }, { "rint", ufbx_rint, rs_rint },
	{ "sqrt", ufbx_sqrt, rs_sqrt }, { "sin", ufbx_sin, rs_sin },
	{ "cos", ufbx_cos, rs_cos }, { "tan", ufbx_tan, rs_tan },
	{ "asin", ufbx_asin, rs_asin }, { "acos", ufbx_acos, rs_acos },
	{ "atan", ufbx_atan, rs_atan },
};
static const fn2 fns2[] = {
	{ "copysign", ufbx_copysign, rs_copysign }, { "atan2", ufbx_atan2, rs_atan2 },
	{ "pow", ufbx_pow, rs_pow }, { "fmin", ufbx_fmin, rs_fmin },
	{ "fmax", ufbx_fmax, rs_fmax }, { "nextafter", ufbx_nextafter, rs_nextafter },
};
enum { NUM_FNS1 = sizeof(fns1) / sizeof(fns1[0]), NUM_FNS2 = sizeof(fns2) / sizeof(fns2[0]) };

int main(int argc, char **argv) {
	long samples = 400000;
	if (argc > 1) samples = atol(argv[1]);
	rng_state = 0x5eed5eed5eed5eedull;

	// Edge cases: every unary fn over the table, every binary fn over the
	// full cross product.
	for (int f = 0; f < NUM_FNS1; f++)
		for (int i = 0; i < NUM_EDGES; i++)
			check1(fns1[f].name, fns1[f].cf, fns1[f].rf, bits_to_f64(edge_bits[i]));
	for (int f = 0; f < NUM_FNS2; f++)
		for (int i = 0; i < NUM_EDGES; i++)
			for (int j = 0; j < NUM_EDGES; j++)
				check2(fns2[f].name, fns2[f].cf, fns2[f].rf,
					bits_to_f64(edge_bits[i]), bits_to_f64(edge_bits[j]));
	for (int i = 0; i < NUM_EDGES; i++) {
		double x = bits_to_f64(edge_bits[i]);
		for (int n = -1100; n <= 1100; n += 13) {
			num_checked++;
			if (f64_to_bits(ufbx_scalbn(x, n)) != f64_to_bits(rs_scalbn(x, n)) && num_failed++ < 10)
				fprintf(stderr, "MISMATCH scalbn(%016llx, %d)\n", (unsigned long long)edge_bits[i], n);
		}
		int ce = 0, re = 0;
		uint64_t cb = f64_to_bits(ufbx_frexp(x, &ce)), rb = f64_to_bits(rs_frexp(x, &re));
		num_checked++;
		if ((cb != rb || ce != re) && num_failed++ < 10)
			fprintf(stderr, "MISMATCH frexp(%016llx)\n", (unsigned long long)edge_bits[i]);
		num_checked++;
		if ((ufbx_isnan(x) != 0) != (rs_isnan(x) != 0) && num_failed++ < 10)
			fprintf(stderr, "MISMATCH isnan(%016llx)\n", (unsigned long long)edge_bits[i]);
	}

	// Random full-range bit patterns + realistic-range values.
	for (long s = 0; s < samples; s++) {
		double xb = bits_to_f64(rng_next()), yb = bits_to_f64(rng_next());
		double xr = rng_realistic(), yr = rng_realistic();
		for (int f = 0; f < NUM_FNS1; f++) {
			check1(fns1[f].name, fns1[f].cf, fns1[f].rf, xb);
			check1(fns1[f].name, fns1[f].cf, fns1[f].rf, xr);
		}
		for (int f = 0; f < NUM_FNS2; f++) {
			check2(fns2[f].name, fns2[f].cf, fns2[f].rf, xb, yb);
			check2(fns2[f].name, fns2[f].cf, fns2[f].rf, xr, yr);
		}
		int n = (int)(rng_next() % 2201u) - 1100;
		num_checked++;
		if (f64_to_bits(ufbx_scalbn(xb, n)) != f64_to_bits(rs_scalbn(xb, n)) && num_failed++ < 10)
			fprintf(stderr, "MISMATCH scalbn(%016llx, %d)\n", (unsigned long long)f64_to_bits(xb), n);
	}

	printf("mathfuzz: %ld checks, %ld mismatches\n", num_checked, num_failed);
	return num_failed == 0 ? 0 : 1;
}
