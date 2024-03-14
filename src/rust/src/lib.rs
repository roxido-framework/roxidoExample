mod registration {
    include!(concat!(env!("OUT_DIR"), "/registration.rs"));
}

use roxido::*;

#[roxido]
fn convolve2(a: &RObject<RVector, f64>, b: &RObject<RVector, f64>) {
    let r = pc.new_vector_double(a.len() + b.len() - 1);
    let ab = r.slice_mut();
    for abi in ab.iter_mut() {
        *abi = 0.0;
    }
    for (i, ai) in a.slice().iter().enumerate() {
        for (j, bj) in b.slice().iter().enumerate() {
            ab[i + j] += ai * bj;
        }
    }
    r
}

#[roxido]
fn zero(f: &RObject<RFunction>, guesses: &RObject<RVector>, tol: f64) {
    if guesses.len() != 2 {
        stop!("'guesses' must be a vector of length two.");
    }
    let (mut x0, mut x1) = {
        let g = guesses.to_double(pc).slice();
        (g[0], g[1])
    };
    if !tol.is_finite() || tol <= 0.0 {
        stop!("'tol' must be a strictly positive value.");
    }
    let x_rval = pc.new_vector_double(1);
    let mut g = |x: f64| {
        let _ = x_rval.set(0, x);
        let Ok(fx) = f.call1(x_rval, pc) else {
            stop!("Error in function evaluation.");
        };
        let fx = fx
            .scalar()
            .stop_str("Unexpected return value from function.")
            .f64();
        if !fx.is_finite() {
            stop!("Non-finite return value from function.");
        }
        fx
    };
    let mut f0 = g(x0);
    if f0 == 0.0 {
        return x0.to_r(pc);
    }
    let f1 = g(x1);
    if f1 == 0.0 {
        return x1.to_r(pc);
    }
    if f0 * f1 > 0.0 {
        stop!("Oops, guesses[0] and guesses[1] have the same sign.");
    }
    loop {
        let xc = 0.5 * (x0 + x1);
        if (x0 - x1).abs() < tol {
            return xc.to_r(pc);
        }
        let fc = g(xc);
        if fc == 0.0 {
            return xc.to_r(pc);
        }
        if f0 * fc > 0.0 {
            x0 = xc;
            f0 = fc;
        } else {
            x1 = xc;
        }
    }
}

#[roxido]
fn myrnorm(n: SEXP, mean: SEXP, sd: SEXP) {
    unsafe {
        use rbindings::*;
        use std::convert::TryFrom;
        let (mean, sd) = (Rf_asReal(mean), Rf_asReal(sd));
        let len_i32 = Rf_asInteger(n);
        let len_isize = isize::try_from(len_i32).unwrap();
        let len_usize = usize::try_from(len_i32).unwrap();
        let vec = Rf_protect(Rf_allocVector(REALSXP, len_isize));
        let slice = std::slice::from_raw_parts_mut(REAL(vec), len_usize);
        GetRNGstate();
        for x in slice {
            *x = Rf_rnorm(mean, sd);
        }
        PutRNGstate();
        Rf_unprotect(1);
        vec
    }
}
