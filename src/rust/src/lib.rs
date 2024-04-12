roxido_registration!();
use roxido::*;

#[roxido]
fn convolve2(a: &[f64], b: &[f64]) {
    let vec = RVector::from_value(0.0, a.len() + b.len() - 1, pc);
    let ab = vec.slice_mut();
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            ab[i + j] += ai * bj;
        }
    }
    vec
}

#[roxido]
fn convolve2a(a: SEXP, b: SEXP) {
    let a = unsafe { RObject::from_sexp(a, pc) };
    let a = a.as_vector().stop_str("'a' is not a vector.");
    let a = a.as_f64().stop_str("'a' is not of storage mode 'double'.");
    let a = a.slice();
    let b = unsafe { RObject::from_sexp(b, pc) };
    let b = b.as_vector().stop_str("'a' is not a vector.");
    let b = b.as_f64().stop_str("'a' is not of storage mode 'double'.");
    let b = b.slice();
    let vec = RVector::from_value(0.0, a.len() + b.len() - 1, pc);
    let ab = vec.slice_mut();
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            ab[i + j] += ai * bj;
        }
    }
    vec.sexp()
}

#[roxido]
fn zero(f: &RFunction, guess1: f64, guess2: f64, tol: f64) {
    if !tol.is_finite() || tol <= 0.0 {
        stop!("'tol' must be a strictly positive value.");
    }
    let x_rval = 0.0.to_r(pc);
    let mut g = |x: f64| {
        x_rval.set(x);
        let Ok(fx) = f.call1(x_rval, pc) else {
            stop!("Error in function evaluation.");
        };
        let fx = fx
            .as_scalar()
            .stop_str("Unexpected return value from function.")
            .f64();
        if !fx.is_finite() {
            stop!("Non-finite return value from function.");
        }
        fx
    };
    let (mut x0, mut x1) = (guess1, guess2);
    let mut f0 = g(x0);
    if f0 == 0.0 {
        return x0;
    }
    let f1 = g(x1);
    if f1 == 0.0 {
        return x1;
    }
    if f0 * f1 > 0.0 {
        stop!("Oops, values of the function at 'guesses1' and 'guesses2' have the same sign.");
    }
    loop {
        let xc = 0.5 * (x0 + x1);
        if (x0 - x1).abs() < tol {
            return xc;
        }
        let fc = g(xc);
        if fc == 0.0 {
            return xc;
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

#[roxido]
fn create_r_objects_from_rust_types() {
    // Scalars
    let _x = 0.0.to_r(pc);
    let _x = 0.to_r(pc);
    let _x = 0_u8.to_r(pc);
    let _x = false.to_r(pc);
    let _x = "A".to_r(pc);
    let _x = (String::from("A")).to_r(pc);
    // Arrays to be used later
    let array_f64 = [0.0, 1.0];
    let array_i32 = [0, 1];
    let array_u8 = [0_u8, 1];
    let array_bool = [false, true];
    let array_str = ["A", "B"];
    // Array
    let _x = array_f64.to_r(pc);
    let _x = array_i32.to_r(pc);
    let _x = array_u8.to_r(pc);
    let _x = array_bool.to_r(pc);
    let _x = array_str.to_r(pc);
    // Reference to array
    let _x = (&array_f64).to_r(pc);
    let _x = (&array_i32).to_r(pc);
    let _x = (&array_u8).to_r(pc);
    let _x = (&array_bool).to_r(pc);
    let _x = (&array_str).to_r(pc);
    // (Reference to a) Slice
    let _x = (&array_f64[..]).to_r(pc);
    let _x = (&array_i32[..]).to_r(pc);
    let _x = (&array_u8[..]).to_r(pc);
    let _x = (&array_bool[..]).to_r(pc);
    let _x = (&array_str[..]).to_r(pc);
    // Iterator
    let _x = array_f64.iter().to_r(pc);
    let _x = array_i32.iter().to_r(pc);
    let _x = array_u8.iter().to_r(pc);
    let _x = array_bool.iter().to_r(pc);
    // IntoIterator
    let _x = array_f64.into_iter().to_r(pc);
    let _x = array_i32.into_iter().to_r(pc);
    let _x = array_u8.into_iter().to_r(pc);
    let _x = array_bool.into_iter().to_r(pc);
    // Map from iterator
    let _x = array_f64.iter().map(|x| 2.0 * x).to_r(pc);
    let _x = array_i32.iter().map(|x| 2 * x).to_r(pc);
    let _x = array_u8.iter().map(|x| 2 * x).to_r(pc);
    let _x = array_bool.iter().map(|x| !x).to_r(pc);
    // Map from into_iterator
    let _x = array_f64.into_iter().map(|x| 2.0 * x).to_r(pc);
    let _x = array_i32.into_iter().map(|x| 2 * x).to_r(pc);
    let _x = array_u8.into_iter().map(|x| 2 * x).to_r(pc);
    let _x = array_bool.into_iter().map(|x| !x).to_r(pc);
    // Unit type
    let _x = ().to_r(pc);
    // SEXP
    let _x = R::null().sexp().to_r(pc);
    // T: RObjectVariant
    let _x = 0_i32.to_r(pc).to_r(pc);
}
