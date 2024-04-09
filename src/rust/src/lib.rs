mod registration {
    include!(concat!(env!("OUT_DIR"), "/registration.rs"));
}

use roxido::*;

#[roxido]
fn convolve4(a: &RVector<f64>, b: &RVector<f64>) {
    let vec_3 = RObject::<RVector, f64>::from_value(5.0, a.len() + b.len() - 1, pc);

    let vec_3 = R2Vector2::from_value(5.0, a.len() + b.len() - 1, pc);
    // let vec_3 = RVector::from_value(5.0, a.len() + b.len() - 1, pc);

    let bob = ["David", "Dahl"].to_2r(pc);
    let bob = [false, true].to_2r(pc);

    let bill = "David".to_2r(pc);
    let bill = 1.0.to_2r(pc);
    let bill = 1_usize.to_2r(pc);
    let sue = 1.0.to_2r(pc);

    sue.set(4.0);
    let s = sue.get();

    // let first_names
    // let ages = c(1,2,3);
    // let vec_3: () = RList::from_tuples([("first",first_names),("ages",ages)], pc);

    let vec_3 = R2Vector2::<f64>::new(a.len() + b.len() - 1, pc);
    let vec_3 = R2Vector2::<char>::new(a.len() + b.len() - 1, pc);
    vec_3.set(4, "advid");
    let vec_4: &mut R2Vector2<f64> = R2Vector2::<f64>::new(a.len() + b.len() - 1, pc);
    let vec = R2Vector2::from_value(0.0, a.len() + b.len() - 1, pc);
    let vec = R2Vector2::from_value("String", a.len() + b.len() - 1, pc);
    let vec2 = R2Vector2::from_value(0, a.len() + b.len() - 1, pc);
    let vec2 = R2Vector2::from_value(false, a.len() + b.len() - 1, pc);
    let vec2 = R2Vector2::from_array([false, true], pc);
    let vec2 = R2Vector2::from_array(["David", "Lisa"], pc);
    //let bob: () = vec2.get(0).stop();
    let vec2 = R2Vector2::from_array([0.0, 1.0], pc);
    let vec2 = R2Vector2::from_array([0_u8, 1], pc);
    let vec2 = R2Vector2::from_array([0_i32, 2], pc);
    let vec = R2Vector2::from_value(0.0, a.len() + b.len() - 1, pc);
    let bob = vec2.get(0).stop();
    macro_rules! rvec {
        ($elem:expr; $n:expr) => {
            R2Vector2::from_value($elem, $n, pc)
        };
    }
    let bob = rvec![0.0; a.len() + b.len() - 1];
    let bob2 = rvec![0; a.len() + b.len() - 1];
    let ab = vec.slice_mut();
    for (i, ai) in a.slice().iter().enumerate() {
        for (j, bj) in b.slice().iter().enumerate() {
            ab[i + j] += ai * bj;
        }
    }
    vec.sexp()
}

#[roxido]
fn convolve2(a: &RObject<RVector>, b: &RObject<RVector>) {
    let vec = RObject::<RVector, f64>::from_value(0.0, a.len() + b.len() - 1, pc);
    let ab = vec.slice_mut();
    for (i, ai) in a.to_f64(pc).slice().iter().enumerate() {
        for (j, bj) in b.to_f64(pc).slice().iter().enumerate() {
            ab[i + j] += ai * bj;
        }
    }
    vec.sexp()
}

#[roxido]
fn convolve23(a: &[f64], b: &[f64]) {
    let vec = R2Vector2::from_value(0.0, a.len() + b.len() - 1, pc);
    let ab = vec.slice_mut();
    for (i, ai) in a.iter().enumerate() {
        for (j, bj) in b.iter().enumerate() {
            ab[i + j] += ai * bj;
        }
    }
    vec.sexp()
}

#[roxido]
fn add(a: f64, b: f64) {
    a + b
}

#[roxido]
fn add2(c: &str) {
    // let c = c.to_2r(pc);
    // c.sexp()
    c
}

#[roxido]
fn add3(c: &RScalar) {
    c.f64().to_2r(pc).sexp()
}

#[roxido]
fn zero(f: &RObject<RFunction>, guess1: f64, guess2: f64, tol: f64) {
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
        stop!("Oops, guesses1 and guesses2 have the same sign.");
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
fn zero2(f: &RFunction, guess1: f64, guess2: f64, tol: f64) {
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
        stop!("Oops, guesses1 and guesses2 have the same sign.");
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
