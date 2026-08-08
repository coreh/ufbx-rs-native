//! Differential fuzz harness: exports the crate-private `native/math.rs`
//! routines under `rs_*` C symbols so `main.c` can compare them bit-for-bit
//! against `extra/ufbx_math.c`. The module is the SAME source file as the
//! library's (via #[path]) — there is exactly one anchored body under test.

#[path = "../../../ufbx/src/native/math.rs"]
#[allow(dead_code)]
mod math;

macro_rules! export1 {
    ($($cname:literal $name:ident;)*) => {$(
        #[export_name = $cname]
        pub extern "C" fn $name(x: f64) -> f64 { math::$name(x) }
    )*};
}

export1! {
    "rs_fabs" fabs;
    "rs_floor" floor;
    "rs_ceil" ceil;
    "rs_rint" rint;
    "rs_sqrt" sqrt;
    "rs_sin" sin;
    "rs_cos" cos;
    "rs_tan" tan;
    "rs_asin" asin;
    "rs_acos" acos;
    "rs_atan" atan;
}

macro_rules! export2 {
    ($($cname:literal $name:ident;)*) => {$(
        #[export_name = $cname]
        pub extern "C" fn $name(a: f64, b: f64) -> f64 { math::$name(a, b) }
    )*};
}

export2! {
    "rs_copysign" copysign;
    "rs_atan2" atan2;
    "rs_pow" pow;
    "rs_fmin" fmin;
    "rs_fmax" fmax;
    "rs_nextafter" nextafter;
}

#[export_name = "rs_scalbn"]
pub extern "C" fn scalbn(x: f64, n: i32) -> f64 {
    math::scalbn(x, n)
}

#[export_name = "rs_frexp"]
pub extern "C" fn frexp(x: f64, eptr: *mut i32) -> f64 {
    math::frexp(x, unsafe { &mut *eptr })
}

#[export_name = "rs_isnan"]
pub extern "C" fn isnan(x: f64) -> i32 {
    math::isnan(x)
}
