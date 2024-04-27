// The 'roxido_registration' macro is called at the start of the 'lib.rs' file.
roxido_registration!();
use roxido::*;

// The function below and the associated comments give examples of using the
// roxido framework. Delete these functions (and the associated R functions in
// the 'R' directory of the package) as you see fit.

// Roxido equivalent of the C 'convolve2' function from "Section 5.10.1
// Calling .Call" in "Writing R Extensions".  This function is automatically
// registered when the package is installed and it can be executed from R using
// '.Call(.convolve2, a, b)'.  Note the 'dot' in from of the function name.
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

// The 'convolve2' function above took advantage of automatically-generated code
// by the roxido macro. When R calls a native function, all arguments are passed
// a SEXP type, which is a pointer to a structure with typedef SEXPREC. The
// roxido macro will automatically declare types for arguments, as is the case
// in the 'convolve2' function above. Of course, this automatic type declaration
// is optional and the function below does this type declaration "by hand". The
// use of the automatic type declaration is a no-cost abstraction since by-hand
// type declaration is no more efficient that the automatically generated code.
#[roxido]
fn convolve2_byhand(a: SEXP, b: SEXP) {
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
    vec
}

// Regarding automatic type declaration, declaring 'a2: f64' causes the 'roxido'
// macro to automatically emit code equivalent to the by-hand code below for
// 'a1: SEXP'. Note that, if an argument cannot be viewed as the declared
// type, the 'stop_str' method causes a R error to be thrown with a helpful
// message about the variable name that was problematic. Also, note the use of
// the 'rprintln!' macro, which is like Rust's builtin 'println!' macro except
// output is guaranteed to go to R's console.
#[roxido]
fn automatic_type_declaration(a1: SEXP, a2: f64) {
    let a1 = unsafe { RObject::from_sexp(a1, pc) };
    let a1 = a1
        .as_scalar()
        .stop_str("\'a1\' is expected to be a scalar.");
    let a1 = a1.f64();
    rprintln!("{} and {} are the same value.", a1, a2);
}

// Automatic type declaration for "scalars" (i.e., R vectors of length one) are
// available for f64 (R's double), i32 (R's integer), u8 (R's raw), bool (R's
// logical), and &str (R's character). Again, R errors are always thrown with
// helpful messages when type declarations do not match the actual arguments
// passed to the function from R.
#[roxido]
fn automatic_type_declare_scalars(_a1: f64, _a2: i32, _a3: u8, _a4: bool, _a5: &str) {}

// Automatic type declaration for slices are available for &[f64] (R's vector of
// storage mode double), &[i32] (R's vector of storage mode integer), and &[u8]
// (R's vector of storage mode raw).
#[roxido]
fn automatic_type_declare_slices(_a1: &[f64], _a2: &[i32], _a3: &[u8]) {}

// Automatic type declarations for vectors are also available. Vectors can
// optionally have element types, e.g., the argument 'a1' in the function below
// must be a vector with storage mode double. In contrast, whereas 'a2' and
// 'a3' must be vectors, any storage mode is acceptable for them. The storage
// mode can later be asserted, as in the code below for 'a2'. And, regardless of
// the storage mode, a vector can be declared to a desired storage mode, as in
// the code below for 'a3'. In the 'to_f64' method, if the storage mode of 'a3'
// is already double, no conversion or copying is performed. Otherwise, a new
// vector of storage mode double is created and, since new memory is allocated,
// a reference to a Pc struct is required. The Pc struct is a protection counter
// that prevents R's garbage collector for reclaiming memory that is still in
// use. The Pc struct and its methods are in lieu of 'PROTECT' and 'UNPROTECT'
// macros that are used when programming against R's C API. A variable called
// 'pc' is a reference to a Pc struct and is automatically made available in
// every function with the 'roxido' attribute.
#[roxido]
fn automatic_type_declare_vectors(_a1: &RVector<f64>, a2: &RVector, a3: &RVector) {
    let _a2 = a2
        .as_f64()
        .stop_str("\'_a1\' is expected to have storage mode double.");
    let _a3 = a3.to_f64(pc);
    rprintln!("a1 and a2 have the same value.");
}

// Automatic type declarations to matrices and arrays are also available.
#[roxido]
fn automatic_type_declare_matrix_and_array(_a1: &RMatrix<i32>, _a2: &RArray<u8>) {}

// Lists are also supported.  Note the use of 0-based indexing.
#[roxido]
fn lists(a1: &RList) {
    let _my_list = RList::with_names(&["first.name", "last.name", "age"], pc);
    let first_element = a1.get(0).stop_str("The supplied list is empty.");
    let name = a1.get_names().get(0).stop_str("Couldn't get name.");
    rprintln!("The name of the first element is: {}", name);
    match first_element.enumerate() {
        RObjectEnum::RVector(x) => {
            rprintln!("Got a vector of length {}!", x.len())
        }
        RObjectEnum::RMatrix(x) => {
            rprintln!("Got a matrix with dimension {}-by-{}!", x.nrow(), x.ncol())
        }
        _ => {
            rprintln!("Got something I didn't expect!")
        }
    }
}

// The function below finds a root of a univariate function supplied by the R
// user and is inspired by the 'zero' function in "Section 5.11.1 Zero-finding"
// of "Writing R Extensions". The body of functions with the 'roxido' attribute
// are actually the body of closures. The return type of the closure can be
// 'SEXP', 'RObject', or, indeed, anything for which an 'to_r' method is defined
// and in scope. See the next function called 'create_r_objects_from_rust_types'
// for a comprehensive list.  In the function below, the return type is 'f64'
// and the 'to_r' is implicitly called after excuting the body.
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

// This shows how to create R objects from Rust types using the 'to_r' method.
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

// Note that roxido functions can be in modules, e.g., the 'myrnorm' function in
// the 'dist' module.
mod dist;
