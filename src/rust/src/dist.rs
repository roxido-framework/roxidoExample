use roxido::*;

// This is an example of a roxido function in a module.  Note that, for
// a 'roxido' function in a module, the 'module token must be set to the
// module name.  The function is then executed from R with, for example,
// '.Call(.dist__myrnorm, n, mean, sd)'.  This example also illustrates
// that, whereas one can directly calls R's C API by directly accessing the
// 'rbindings' module, that does not take advantages of the ergonomics and
// safety of the roxido framework.
#[roxido(module = dist)]
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
