//! The paper
//! [*Writing R Extensions in Rust*](https://raw.githubusercontent.com/dbdahl/cargo-framework/main/cargo/inst/doc/Writing_R_Extensions_in_Rust.pdf)
//! complements
//! [*Writing R Extensions*](https://cran.r-project.org/doc/manuals/R-exts.html)
//! (the official guide for writing R extensions) for those interested in developing
//! [R](https://www.r-project.org/) packages using
//! [Rust](https://www.rust-lang.org/). It highlights idiosyncrasies of
//! [R](https://www.r-project.org/) and [Rust](https://www.rust-lang.org/) that must
//! be addressed by any integration and describes how to develop
//! [Rust](https://www.rust-lang.org/)-based packages which comply with the [CRAN
//! Repository Policy](https://cran.r-project.org/web/packages/policies.html).  The
//! [paper]( https://raw.githubusercontent.com/dbdahl/cargo-framework/main/cargo/inst/doc/Writing_R_Extensions_in_Rust.pdf)
//! introduces the cargo framework, a
//! transparent [Rust](https://www.rust-lang.org/)-based API which wraps
//! commonly-used parts of [R](https://www.r-project.org/)'s API with minimal
//! overhead and allows a programmer to easily add additional wrappers.
//!
//! This crate provides the Rust API for the cargo framework.  Of particular note
//! is [R] and [RObject].
//!
//! # Example
//!
//! This example function takes two R vectors and computes the convolution
//! of them. Any function with the roxido attribute must return an [RObject],
//! and may only take parameters of [RObject] type.
//!
//! ```
//! use roxido::*;
//!
//! #[roxido]
//! fn convolve2(a: &RObject<RVector>, b: &RObject<RVector>) {
//!     let vec = RObject::<RVector, f64>::from_value(0.0, a.len() + b.len() - 1, pc);
//!     let ab = vec.slice_mut();
//!     for (i, ai) in a.to_f64(pc).slice().iter().enumerate() {
//!         for (j, bj) in b.to_f64(pc).slice().iter().enumerate() {
//!             ab[i + j] += ai * bj;
//!         }
//!     }
//!     vec
//! }
//! ```

// Helpful resources:
//   https://cran.r-project.org/doc/manuals/r-release/R-ints.html
//   https://svn.r-project.org/R/trunk/src/include/Rinternals.h
//   https://github.com/hadley/r-internals
//   https://www.tidyverse.org/blog/2019/05/resource-cleanup-in-c-and-the-r-api
//   https://github.com/wch/r-source

pub mod rbindings;

/// A procedural macro to facilitate calling a Rust function from R.
pub use roxido_macro::roxido;

pub use rbindings::SEXP;

pub trait SexpMethods {
    fn as_robject(&self) -> &RObject;

    #[allow(clippy::mut_from_ref)]
    fn as_robject_mut(&self) -> &mut RObject;

    fn as_str<'a>(&self) -> Result<&'a str, &'static str>;

    fn from_str(x: &str, pc: &Pc) -> Self;

    /// # Safety
    /// Expert use only.
    unsafe fn transmute<T: IsRObject, A>(self, _anchor: &A) -> &T;

    /// # Safety
    /// Expert use only.
    #[allow(clippy::mut_from_ref)]
    unsafe fn transmute_mut<T: IsRObject, A>(self, _anchor: &A) -> &mut T;

    /// # Safety
    /// Expert use only.
    unsafe fn transmute_static<T: IsRObject>(self) -> &'static T;
}

impl SexpMethods for SEXP {
    fn as_robject(&self) -> &RObject {
        unsafe { self.transmute(self) }
    }

    fn as_robject_mut(&self) -> &mut RObject {
        unsafe { self.transmute_mut(self) }
    }

    fn as_str<'a>(&self) -> Result<&'a str, &'static str> {
        let c_str = unsafe { CStr::from_ptr(R_CHAR(*self) as *const c_char) };
        c_str.to_str().map_err(|_| "Not valid UTF8.")
    }

    fn from_str(x: &str, pc: &Pc) -> Self {
        pc.protect(unsafe {
            Rf_mkCharLenCE(
                x.as_ptr() as *const c_char,
                x.len().try_into().stop_str(TOO_LONG),
                cetype_t_CE_UTF8,
            )
        })
    }

    unsafe fn transmute<T, A>(self, _anchor: &A) -> &T {
        unsafe { &*self.cast::<T>() }
    }

    unsafe fn transmute_mut<T, A>(self, _anchor: &A) -> &mut T {
        unsafe { &mut *self.cast::<T>() }
    }

    unsafe fn transmute_static<T>(self) -> &'static T {
        unsafe { &*self.cast::<T>() }
    }
}

use rbindings::*;
use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr};
use std::marker::PhantomData;

static TOO_LONG: &str = "Could not fit usize into i32.";

pub struct R;

pub trait IsRObject: Sized {
    fn sexp(&self) -> SEXP {
        self as *const Self as SEXP
    }

    /// # Safety
    /// Expert use only.
    unsafe fn transmute<T: IsRObject>(&self) -> &T
    where
        Self: Sized,
    {
        self.sexp().transmute(self)
    }

    /// # Safety
    /// Expert use only.
    unsafe fn transmute_mut<T: IsRObject>(&mut self) -> &mut T
    where
        Self: Sized,
    {
        self.sexp().transmute_mut(self)
    }

    /// Duplicate an object.
    ///
    /// Multiple symbols may be bound to the same object, so if the usual R semantics are to
    /// apply, any code which alters one of them needs to make a copy first.
    /// E.g, call this method on arguments pass via `.Call` before modifying them.
    ///
    #[allow(clippy::mut_from_ref)]
    fn clone<'a>(&self, pc: &'a Pc) -> &'a mut Self {
        unsafe { pc.protect_and_transmute(Rf_duplicate(self.sexp())) }
    }

    /// Get the class or classes of the data in an RObject.
    fn get_class(&self) -> &RVector<char> {
        unsafe { Rf_getAttrib(self.sexp(), RSymbol::class().sexp()).transmute(self) }
    }

    /// Set the class or classes of the data for an RObject.
    fn set_class(&mut self, names: &RVector<char>) {
        unsafe {
            Rf_classgets(self.sexp(), names.sexp());
        }
    }

    /// Get an attribute.
    fn get_attribute(&self, which: &RSymbol) -> &RObject {
        unsafe { Rf_getAttrib(self.sexp(), which.sexp()).transmute(self) }
    }

    /// Set an attribute.
    fn set_attribute(&mut self, which: &RSymbol, value: &impl IsRObject) {
        unsafe {
            Rf_setAttrib(self.sexp(), which.sexp(), value.sexp());
        }
    }
}

macro_rules! baseline {
    ($name:ident) => {
        #[repr(C)]
        pub struct $name {
            pub sexprec: SEXPREC,
        }
        impl IsRObject for $name {}
    };
}

baseline!(RObject);
baseline!(RSymbol);
baseline!(RList);
baseline!(RDataFrame);
baseline!(RFunction);
baseline!(RExternalPtr);
baseline!(RError);

pub trait RHasLength: IsRObject {
    /// Returns the length of the RObject.
    fn len(&self) -> usize {
        let len = unsafe { Rf_xlength(self.sexp()) };
        len.try_into().unwrap() // Won't ever fail if R is sane.
    }

    /// Checks to see if the RObject is empty.
    fn is_empty(&self) -> bool {
        unsafe { Rf_xlength(self.sexp()) == 0 }
    }

    /// Checks to see if the RObject is a scalar (has a length of 1).
    fn is_scalar(&self) -> bool {
        unsafe { Rf_xlength(self.sexp()) == 1 }
    }
}

pub trait RHasNames: RHasLength {
    /// Get names of values in a RVector.
    fn get_names(&self) -> &RVector<char> {
        unsafe { Rf_getAttrib(self.sexp(), R_NamesSymbol).transmute(self) }
    }

    /// Set names of values in a RVector.
    fn set_names(&mut self, names: &RVector<char>) -> Result<(), &'static str> {
        if unsafe { Rf_length(names.sexp()) != Rf_length(self.sexp()) } {
            return Err("Length of names is not correct.");
        }
        unsafe {
            Rf_namesgets(self.sexp(), names.sexp());
        }
        Ok(())
    }
}

macro_rules! baseline_with_type {
    ($name:ident) => {
        #[repr(C)]
        pub struct $name<RMode = ()> {
            pub sexprec: SEXPREC,
            rtype: PhantomData<RMode>,
        }
        impl<T> IsRObject for $name<T> {}
        impl<T> RHasLength for $name<T> {}
    };
}

baseline_with_type!(RScalar);
baseline_with_type!(RVector);
baseline_with_type!(RMatrix);
baseline_with_type!(RArray);

impl RHasLength for RList {}
impl RHasLength for RDataFrame {}
impl<T> RHasNames for RScalar<T> {}
impl<T> RHasNames for RVector<T> {}
impl RHasNames for RList {}
impl RHasNames for RDataFrame {}

pub struct Pc {
    counter: std::cell::RefCell<i32>,
}

impl Default for Pc {
    fn default() -> Self {
        Self::__private_new()
    }
}

impl Drop for Pc {
    fn drop(&mut self) {
        let count = self.counter.borrow();
        if *self.counter.borrow() > 0 {
            unsafe { Rf_unprotect(*count) };
        }
    }
}

impl Pc {
    /// Allocate a new protection counter.
    ///
    /// This is an implementation detail and *should not* be called directly!
    /// Functions defined with the `roxido` macro already have an instance of this structure named
    /// `pc`, so this function is generally not needed.
    ///
    #[doc(hidden)]
    pub fn __private_new() -> Self {
        Self { counter: 0.into() }
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    pub fn protect(&self, sexp: SEXP) -> SEXP {
        unsafe { Rf_protect(sexp) };
        let mut counter = self.counter.borrow_mut();
        *counter += 1;
        sexp
    }

    /// # Safety
    /// Expert use only.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn protect_and_transmute<T: IsRObject>(&self, sexp: SEXP) -> &mut T {
        let sexp = self.protect(sexp);
        unsafe { sexp.transmute_mut(self) }
    }
}

impl R {
    /// Returns an R NULL value.
    pub fn null() -> &'static RObject {
        unsafe { R_NilValue.transmute_static() }
    }

    /// Returns an R NA value for storage mode "double".
    pub fn na_f64() -> f64 {
        unsafe { R_NaReal }
    }

    /// Returns an R NA value for storage mode "integer".
    pub fn na_i32() -> i32 {
        unsafe { R_NaInt }
    }

    /// Returns an R NA value for storage mode "logical".
    pub fn na_bool() -> i32 {
        unsafe { R_NaInt }
    }

    /// Returns an R NaN value.
    pub fn nan() -> f64 {
        unsafe { R_NaN }
    }

    /// Returns an R Inf value.
    pub fn positive_infinity() -> f64 {
        unsafe { R_PosInf }
    }

    /// Returns an R -Inf value.
    pub fn negative_infinity() -> f64 {
        unsafe { R_NegInf }
    }

    /// Checks if an f64 can be interpreted as an R NA value.
    pub fn is_na_f64(x: f64) -> bool {
        unsafe { R_IsNA(x) != 0 }
    }

    /// Checks if an i32 can be interpreted as an R NA value.
    pub fn is_na_i32(x: i32) -> bool {
        x == unsafe { R_NaInt }
    }

    /// Checks if a bool can be interpreted as an R NA value.
    pub fn is_na_bool(x: i32) -> bool {
        x == unsafe { R_NaInt }
    }

    pub fn is_true(x: i32) -> bool {
        x != Rboolean_FALSE.try_into().unwrap() && !Self::is_na_bool(x)
    }

    pub fn as_logical(x: bool) -> i32 {
        if x {
            Rboolean_TRUE.try_into().unwrap()
        } else {
            Rboolean_FALSE.try_into().unwrap()
        }
    }

    /// Checks if an f64 can be interpreted as an R NaN value.
    pub fn is_nan(x: f64) -> bool {
        unsafe { R_IsNaN(x) != 0 }
    }

    /// Checks if an f64 would be considered finite in R.
    pub fn is_finite(x: f64) -> bool {
        unsafe { R_finite(x) != 0 }
    }

    /// Checks if an f64 would be considered finite in R.
    pub fn is_positive_infinity(x: f64) -> bool {
        unsafe { x == R_PosInf }
    }

    /// Checks if an f64 would be considered finite in R.
    pub fn is_negative_infinity(x: f64) -> bool {
        unsafe { x == R_NegInf }
    }

    /// Generate random bytes using R's RNG.
    pub fn random_bytes<const LENGTH: usize>() -> [u8; LENGTH] {
        unsafe {
            let m = (u8::MAX as f64) + 1.0;
            let mut bytes: [u8; LENGTH] = [0; LENGTH];
            GetRNGstate();
            for x in bytes.iter_mut() {
                *x = R_unif_index(m) as u8;
            }
            PutRNGstate();
            bytes
        }
    }

    /// Flush the R console.
    pub fn flush_console() {
        unsafe { R_FlushConsole() };
    }

    /// Check to see if the user has attempted to interrupt the execution.
    pub fn check_user_interrupt() -> bool {
        extern "C" fn check_interrupt_fn(_: *mut c_void) {
            unsafe { R_CheckUserInterrupt() };
        }
        unsafe { R_ToplevelExec(Some(check_interrupt_fn), std::ptr::null_mut()) == 0 }
    }
}

impl RObject {
    /// Returns the result of the is_null method, but as an Option value.
    pub fn as_option(&self) -> Option<&Self> {
        if self.is_null() {
            None
        } else {
            Some(self)
        }
    }

    pub fn as_scalar(&self) -> Result<&RScalar, &'static str> {
        let s = self.as_vector()?;
        if s.is_scalar() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not a vector.")
        }
    }

    pub fn as_scalar_mut(&mut self) -> Result<&mut RScalar, &'static str> {
        let s = self.as_vector()?;
        if s.is_scalar() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not a scalar.")
        }
    }

    pub fn as_vector(&self) -> Result<&RVector, &'static str> {
        if self.is_vector() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not a vector.")
        }
    }

    pub fn as_vector_mut(&mut self) -> Result<&mut RVector, &'static str> {
        if self.is_vector() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not a vector.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RMatrix>`.
    /// Checks using R's `Rf_isMatrix` function.
    pub fn as_matrix(&self) -> Result<&RMatrix, &'static str> {
        if self.is_matrix() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not a matrix.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RMatrix>`.
    /// Checks using R's `Rf_isMatrix` function.
    pub fn as_matrix_mut(&mut self) -> Result<&mut RMatrix, &'static str> {
        if self.is_matrix() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not a matrix.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RArray>`.
    /// Checks using R's `Rf_isArray` function.
    pub fn as_array(&self) -> Result<&RArray, &'static str> {
        if self.is_array() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not a vector.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RArray>`.
    /// Checks using R's `Rf_isArray` function.
    pub fn as_array_mut(&mut self) -> Result<&mut RArray, &'static str> {
        if self.is_array() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not a vector.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RVector, RList>`.
    /// Checks using R's `Rf_isVectorList` function.
    pub fn as_list(&self) -> Result<&RList, &'static str> {
        if self.is_list() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not a list.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RVector, RList>`.
    /// Checks using R's `Rf_isVectorList` function.
    pub fn as_list_mut(&mut self) -> Result<&mut RList, &'static str> {
        if self.is_list() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not a list.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RVector, RDataFrame>`.
    /// Checks using R's `Rf_isFrame` function.
    pub fn as_data_frame(&self) -> Result<&RDataFrame, &'static str> {
        if self.is_data_frame() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not a data frame.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RVector, RDataFrame>`.
    /// Checks using R's `Rf_isFrame` function.
    pub fn as_data_frame_mut(&mut self) -> Result<&mut RDataFrame, &'static str> {
        if self.is_data_frame() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not a data frame.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RFunction>`.
    /// Checks using R's `Rf_isFunction` function.
    pub fn as_function(&self) -> Result<&RFunction, &'static str> {
        if self.is_function() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not a function.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RFunction>`.
    /// Checks using R's `Rf_isFunction` function.
    pub fn as_function_mut(&mut self) -> Result<&mut RFunction, &'static str> {
        if self.is_function() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not a function.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RExternalPtr>`.
    /// Uses the SEXP type to determine if this is possible.
    pub fn as_external_ptr(&self) -> Result<&RExternalPtr, &'static str> {
        if self.is_external_ptr() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not an external pointer.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RExternalPtr>`.
    /// Uses the SEXP type to determine if this is possible.
    pub fn as_external_ptr_mut(&mut self) -> Result<&mut RExternalPtr, &'static str> {
        if self.is_external_ptr() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not an external pointer.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RSymbol>`.
    /// Uses the SEXP type to determine if this is possible.
    pub fn as_symbol(&self) -> Result<&RSymbol, &'static str> {
        if self.is_symbol() {
            Ok(unsafe { self.transmute() })
        } else {
            Err("Not an external pointer.")
        }
    }

    /// Check if appropriate to characterize as an `RObject<RSymbol>`.
    /// Uses the SEXP type to determine if this is possible.
    pub fn as_symbol_mut(&mut self) -> Result<&mut RSymbol, &'static str> {
        if self.is_symbol() {
            Ok(unsafe { self.transmute_mut() })
        } else {
            Err("Not an external pointer.")
        }
    }

    /// Check if RObject can be interpreted as the NULL value in R.
    pub fn is_null(&self) -> bool {
        unsafe { Rf_isNull(self.sexp()) != 0 }
    }

    pub fn is_vector(&self) -> bool {
        unsafe { Rf_isVectorAtomic(self.sexp()) != 0 }
    }

    pub fn is_matrix(&self) -> bool {
        unsafe { Rf_isMatrix(self.sexp()) != 0 }
    }

    pub fn is_array(&self) -> bool {
        unsafe { Rf_isArray(self.sexp()) != 0 }
    }

    pub fn is_list(&self) -> bool {
        unsafe { Rf_isVectorList(self.sexp()) != 0 }
    }

    pub fn is_data_frame(&self) -> bool {
        unsafe { Rf_isFrame(self.sexp()) != 0 }
    }

    pub fn is_function(&self) -> bool {
        unsafe { Rf_isFunction(self.sexp()) != 0 }
    }

    pub fn is_external_ptr(&self) -> bool {
        unsafe { TYPEOF(self.sexp()) == EXTPTRSXP as i32 }
    }

    pub fn is_symbol(&self) -> bool {
        unsafe { TYPEOF(self.sexp()) == SYMSXP as i32 }
    }
}

impl RError {
    /// Define a new error.
    ///
    /// This does *not* throw an error.  To throw an R error, simply use `stop!`.
    ///
    #[allow(clippy::mut_from_ref)]
    pub fn new<'a>(message: &str, pc: &'a Pc) -> &'a mut Self {
        let list = RList::with_names(&["message", "calls"], pc);
        let _ = list.set(0, message.to_r(pc));
        let _ = list.set(1, R::null());
        list.set_class(["error", "condition"].to_r(pc));
        unsafe { list.transmute_mut() }
    }
}

impl RSymbol {
    /// Define a new symbol.
    #[allow(clippy::mut_from_ref)]
    pub fn new<'a>(x: &str, pc: &'a Pc) -> &'a mut Self {
        unsafe { pc.protect_and_transmute(SEXP::from_str(x, pc)) }
    }

    /// Get R's "dim" symbol.
    pub fn dim() -> &'static Self {
        unsafe { R_DimSymbol.transmute_static() }
    }

    /// Get R's "names" symbol.
    pub fn names() -> &'static Self {
        unsafe { R_NamesSymbol.transmute_static() }
    }

    /// Get R's "rownames" symbol.
    pub fn rownames() -> &'static Self {
        unsafe { R_RowNamesSymbol.transmute_static() }
    }

    /// Get R's "dimnames" symbol.
    pub fn dimnames() -> &'static Self {
        unsafe { R_DimNamesSymbol.transmute_static() }
    }

    /// Get R's "class" symbol.
    pub fn class() -> &'static Self {
        unsafe { R_ClassSymbol.transmute_static() }
    }
}

impl RFunction {
    fn eval(expression: SEXP, pc: &Pc) -> Result<&RObject, i32> {
        let expression = pc.protect(expression);
        let mut p_out_error: i32 = 0;
        let sexp = pc.protect(unsafe {
            R_tryEval(expression, R_GetCurrentEnv(), &mut p_out_error as *mut i32)
        });
        match p_out_error {
            0 => Ok(unsafe { sexp.transmute(pc) }),
            e => Err(e),
        }
    }

    /// Evaluate a function with 0 parameters.
    pub fn call0<'a>(&self, pc: &'a Pc) -> Result<&'a RObject, i32> {
        let expression = unsafe { Rf_lang1(self.sexp()) };
        Self::eval(expression, pc)
    }

    /// Evaluate a function with 1 parameter.
    pub fn call1<'a>(&self, arg1: &impl IsRObject, pc: &'a Pc) -> Result<&'a RObject, i32> {
        let expression = unsafe { Rf_lang2(self.sexp(), arg1.sexp()) };
        Self::eval(expression, pc)
    }

    /// Evaluate a function with 2 parameters.
    pub fn call2<'a>(
        &self,
        arg1: &impl IsRObject,
        arg2: &impl IsRObject,
        pc: &'a Pc,
    ) -> Result<&'a RObject, i32> {
        let expression = unsafe { Rf_lang3(self.sexp(), arg1.sexp(), arg2.sexp()) };
        Self::eval(expression, pc)
    }

    /// Evaluate a function with 3 parameters.
    pub fn call3<'a>(
        &self,
        arg1: &impl IsRObject,
        arg2: &impl IsRObject,
        arg3: &impl IsRObject,
        pc: &'a Pc,
    ) -> Result<&'a RObject, i32> {
        let expression = unsafe { Rf_lang4(self.sexp(), arg1.sexp(), arg2.sexp(), arg3.sexp()) };
        Self::eval(expression, pc)
    }

    /// Evaluate a function with 4 parameters.
    pub fn call4<'a>(
        &self,
        arg1: &impl IsRObject,
        arg2: &impl IsRObject,
        arg3: &impl IsRObject,
        arg4: &impl IsRObject,
        pc: &'a Pc,
    ) -> Result<&'a RObject, i32> {
        let expression = unsafe {
            Rf_lang5(
                self.sexp(),
                arg1.sexp(),
                arg2.sexp(),
                arg3.sexp(),
                arg4.sexp(),
            )
        };
        Self::eval(expression, pc)
    }

    /// Evaluate a function with 5 parameters.
    pub fn call5<'a>(
        &self,
        arg1: &impl IsRObject,
        arg2: &impl IsRObject,
        arg3: &impl IsRObject,
        arg4: &impl IsRObject,
        arg5: &impl IsRObject,
        pc: &'a Pc,
    ) -> Result<&'a RObject, i32> {
        let expression = unsafe {
            Rf_lang6(
                self.sexp(),
                arg1.sexp(),
                arg2.sexp(),
                arg3.sexp(),
                arg4.sexp(),
                arg5.sexp(),
            )
        };
        Self::eval(expression, pc)
    }
}

impl RScalar {
    /// Check if appropriate to characterize as an f64.
    pub fn f64(&self) -> f64 {
        unsafe { Rf_asReal(self.sexp()) }
    }

    /// Check if appropriate to characterize as an i32.
    pub fn i32(&self) -> Result<i32, &'static str> {
        if self.is_i32() {
            let x = unsafe { Rf_asInteger(self.sexp()) };
            if x == i32::MIN {
                Err("i32 equals R's NA for integers.")
            } else {
                Ok(x)
            }
        } else if self.is_f64() {
            let y = unsafe { Rf_asReal(self.sexp()) };
            if y > f64::from(i32::MAX) {
                Err("Greater than maximum integer value.")
            } else if y < f64::from(i32::MIN) {
                Err("Less than minimum integer value.")
            } else if y == f64::from(i32::MIN) {
                Err("Equals R's NA for integers.")
            } else if y.is_nan() {
                Err("Equals R's NaN.")
            } else {
                Ok(y.round() as i32)
            }
        } else if self.is_u8() {
            Ok(unsafe { Rf_asInteger(self.sexp()) })
        } else if self.is_bool() {
            let y = unsafe { Rf_asLogical(self.sexp()) };
            if y == i32::MIN {
                Err("Equals R's NA for logical.")
            } else {
                Ok(y)
            }
        } else {
            Err("Unsupported R type.")
        }
    }

    /// Check if appropriate to characterize as a usize.
    pub fn usize(&self) -> Result<usize, &'static str> {
        if self.is_i32() {
            let x = unsafe { Rf_asInteger(self.sexp()) };
            if x == i32::MIN {
                Err("Equals R's NA for integers.")
            } else if x < 0 {
                Err("Negative value not expected.")
            } else {
                usize::try_from(x).map_err(|_| "Cannot convert to usize.")
            }
        } else if self.is_f64() {
            let y = unsafe { Rf_asReal(self.sexp()) };
            if y < 0.0 {
                Err("Negative value not expected.")
            } else {
                let z = y as usize;
                if z as f64 == y {
                    Ok(z)
                } else {
                    Err("Cannot convert to usize.")
                }
            }
        } else if self.is_u8() {
            let x = unsafe { Rf_asInteger(self.sexp()) };
            usize::try_from(x).map_err(|_| "Cannot convert to usize.")
        } else if self.is_bool() {
            let x = unsafe { Rf_asLogical(self.sexp()) };
            if x == i32::MIN {
                Err("Equals R's NA for logical.")
            } else {
                usize::try_from(x).map_err(|_| "Cannot convert to usize.")
            }
        } else {
            Err("Unsupported R type.")
        }
    }

    /// Check if appropriate to characterize as a u8.
    pub fn u8(&self) -> Result<u8, &'static str> {
        if self.is_i32() {
            let x = unsafe { Rf_asInteger(self.sexp()) };
            u8::try_from(x).map_err(|_| "Cannot convert to u8.")
        } else if self.is_f64() {
            let y = unsafe { Rf_asReal(self.sexp()) };
            if y < 0.0 {
                Err("Negative value not expected.")
            } else {
                let z = y as u8;
                if z as f64 == y {
                    Ok(z)
                } else {
                    Err("Cannot convert to u8.")
                }
            }
        } else if self.is_u8() {
            let x = unsafe { Rf_asInteger(self.sexp()) };
            u8::try_from(x).map_err(|_| "Cannot convert to u8.")
        } else if self.is_bool() {
            let x = unsafe { Rf_asLogical(self.sexp()) };
            if x == i32::MIN {
                Err("Equals R's NA for logical.")
            } else {
                u8::try_from(x).map_err(|_| "Cannot convert to u8.")
            }
        } else {
            Err("Unsupported R type.")
        }
    }

    /// Check if appropriate to characterize as a bool.
    pub fn bool(&self) -> Result<bool, &'static str> {
        if self.is_i32() {
            let x = unsafe { Rf_asInteger(self.sexp()) };
            if x == i32::MIN {
                Err("Equals R's NA for integers.")
            } else {
                Ok(R::is_true(x))
            }
        } else if self.is_f64() {
            let y = unsafe { Rf_asReal(self.sexp()) };
            if R::is_na_f64(y) {
                Err("Equals R's NA for doubles.")
            } else if R::is_nan(y) {
                Err("Equals R's NaN.")
            } else {
                Ok(y != 0.0)
            }
        } else if self.is_u8() {
            Ok(R::is_true(unsafe { Rf_asInteger(self.sexp()) }))
        } else if self.is_bool() {
            let y = unsafe { Rf_asLogical(self.sexp()) };
            if y == i32::MIN {
                Err("Equals R's NA for logical.")
            } else {
                Ok(R::is_true(y))
            }
        } else {
            Err("Unsupported R type.")
        }
    }

    /// Check if appropriate to characterize as a str reference.
    pub fn str<'a>(&'a self, pc: &'a Pc) -> &'a str {
        let s: &RVector<char> = unsafe { self.to_char(pc).transmute() }; // DBD
        s.get(0).unwrap()
    }

    /// Check if RObject can be interpreted as an NA value in R.
    pub fn is_na(&self) -> bool {
        if self.is_f64() {
            unsafe { R_IsNA(Rf_asReal(self.sexp())) != 0 }
        } else if self.is_i32() {
            unsafe { Rf_asInteger(self.sexp()) == R::na_i32() }
        } else if self.is_bool() {
            unsafe { Rf_asLogical(self.sexp()) == R::na_bool() }
        } else if self.is_char() {
            unsafe { Rf_asChar(self.sexp()) == R_NaString }
        } else {
            false
        }
    }

    /// Check if RObject can be interpreted as an NaN value in R.
    pub fn is_nan(&self) -> bool {
        if self.is_f64() {
            unsafe { R_IsNaN(Rf_asReal(self.sexp())) != 0 }
        } else {
            false
        }
    }

    pub fn is_finite(&self) -> bool {
        if self.is_f64() {
            unsafe { R_finite(Rf_asReal(self.sexp())) != 0 }
        } else {
            false
        }
    }

    pub fn is_positive_infinity(&self) -> bool {
        if self.is_f64() {
            unsafe { Rf_asReal(self.sexp()) == R_PosInf }
        } else {
            false
        }
    }

    pub fn is_negative_infinity(&self) -> bool {
        if self.is_f64() {
            unsafe { Rf_asReal(self.sexp()) == R_NegInf }
        } else {
            false
        }
    }
}

pub trait RScalarConstructor<T> {
    #[allow(clippy::mut_from_ref)]
    fn from_value(value: T, pc: &Pc) -> &mut Self;
}

macro_rules! r2scalar2 {
    ($tipe:ty, $code:expr) => {
        impl RScalarConstructor<$tipe> for RScalar<$tipe> {
            #[allow(clippy::mut_from_ref)]
            fn from_value(value: $tipe, pc: &Pc) -> &mut Self {
                unsafe { pc.protect_and_transmute($code(value)) }
            }
        }
    };
}

r2scalar2!(f64, Rf_ScalarReal);
r2scalar2!(i32, Rf_ScalarInteger);
r2scalar2!(u8, Rf_ScalarRaw);

impl RScalarConstructor<bool> for RScalar<bool> {
    #[allow(clippy::mut_from_ref)]
    fn from_value(value: bool, pc: &Pc) -> &mut Self {
        unsafe { pc.protect_and_transmute(Rf_ScalarLogical(R::as_logical(value))) }
    }
}

impl RScalarConstructor<&str> for RScalar<char> {
    #[allow(clippy::mut_from_ref)]
    fn from_value<'a>(value: &str, pc: &'a Pc) -> &'a mut Self {
        unsafe {
            pc.protect_and_transmute(Rf_ScalarString(pc.protect(Rf_mkCharLenCE(
                value.as_ptr() as *const c_char,
                value.len().try_into().stop_str(TOO_LONG),
                cetype_t_CE_UTF8,
            ))))
        }
    }
}

macro_rules! rconvertable {
    ($name:ident) => {
        impl<T> $name<T> {
            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_f64(&self) -> Result<&$name<f64>, &'static str> {
                if self.is_f64() {
                    Ok(unsafe { self.transmute() })
                } else {
                    Err("Not of storage mode 'double'.")
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_f64_mut(&mut self) -> Result<&mut $name<f64>, &'static str> {
                if self.is_f64() {
                    Ok(unsafe { self.transmute_mut() })
                } else {
                    Err("Not of storage mode 'double'.")
                }
            }

            /// Checks to see if the data can be interpreted as R double.
            pub fn is_f64(&self) -> bool {
                unsafe { Rf_isReal(self.sexp()) != 0 }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_f64<'a>(&'a self, pc: &'a Pc) -> &'a $name<f64> {
                if self.is_f64() {
                    unsafe { self.transmute() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), REALSXP)) }
                }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_f64_mut<'a>(&'a mut self, pc: &'a Pc) -> &'a mut $name<f64> {
                if self.is_f64() {
                    unsafe { self.transmute_mut() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), REALSXP)) }
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_i32(&self) -> Result<&$name<i32>, &'static str> {
                if self.is_i32() {
                    Ok(unsafe { self.transmute() })
                } else {
                    Err("Not of storage mode 'integer'.")
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_i32_mut(&mut self) -> Result<&mut $name<i32>, &'static str> {
                if self.is_i32() {
                    Ok(unsafe { self.transmute_mut() })
                } else {
                    Err("Not of storage mode 'integer'.")
                }
            }

            /// Checks to see if the data can be interpreted as R double.
            pub fn is_i32(&self) -> bool {
                unsafe { Rf_isInteger(self.sexp()) != 0 }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_i32<'a>(&'a self, pc: &'a Pc) -> &'a $name<i32> {
                if self.is_i32() {
                    unsafe { self.transmute() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), INTSXP)) }
                }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_i32_mut<'a>(&'a mut self, pc: &'a Pc) -> &'a mut $name<i32> {
                if self.is_i32() {
                    unsafe { self.transmute_mut() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), INTSXP)) }
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_u8(&self) -> Result<&$name<u8>, &'static str> {
                if self.is_u8() {
                    Ok(unsafe { self.transmute() })
                } else {
                    Err("Not of storage mode 'raw'.")
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_u8_mut(&mut self) -> Result<&mut $name<u8>, &'static str> {
                if self.is_u8() {
                    Ok(unsafe { self.transmute_mut() })
                } else {
                    Err("Not of storage mode 'raw'.")
                }
            }

            /// Checks to see if the data can be interpreted as R double.
            pub fn is_u8(&self) -> bool {
                unsafe { TYPEOF(self.sexp()) == RAWSXP as i32 }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_u8<'a>(&'a self, pc: &'a Pc) -> &'a $name<u8> {
                if self.is_u8() {
                    unsafe { self.transmute() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), RAWSXP)) }
                }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_u8_mut<'a>(&'a mut self, pc: &'a Pc) -> &'a mut $name<u8> {
                if self.is_u8() {
                    unsafe { self.transmute_mut() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), RAWSXP)) }
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_bool(&self) -> Result<&$name<bool>, &'static str> {
                if self.is_bool() {
                    Ok(unsafe { self.transmute() })
                } else {
                    Err("Not of storage mode 'logical'.")
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_bool_mut(&mut self) -> Result<&mut $name<bool>, &'static str> {
                if self.is_bool() {
                    Ok(unsafe { self.transmute_mut() })
                } else {
                    Err("Not of storage mode 'logical'.")
                }
            }

            /// Checks to see if the data can be interpreted as R double.
            pub fn is_bool(&self) -> bool {
                unsafe { Rf_isLogical(self.sexp()) != 0 }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_bool<'a>(&'a self, pc: &'a Pc) -> &'a $name<bool> {
                if self.is_bool() {
                    unsafe { self.transmute() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), LGLSXP)) }
                }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_bool_mut<'a>(&'a mut self, pc: &'a Pc) -> &'a mut $name<bool> {
                if self.is_bool() {
                    unsafe { self.transmute_mut() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), LGLSXP)) }
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_char(&self) -> Result<&$name<char>, &'static str> {
                if self.is_char() {
                    Ok(unsafe { self.transmute() })
                } else {
                    Err("Not of storage mode 'character'.")
                }
            }

            /// Check if appropriate to characterize storage mode as "double".
            pub fn as_char_mut(&mut self) -> Result<&mut $name<char>, &'static str> {
                if self.is_char() {
                    Ok(unsafe { self.transmute_mut() })
                } else {
                    Err("Not of storage mode 'character'.")
                }
            }

            /// Checks to see if the data can be interpreted as R double.
            pub fn is_char(&self) -> bool {
                unsafe { Rf_isString(self.sexp()) != 0 }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_char<'a>(&'a self, pc: &'a Pc) -> &'a $name<char> {
                if self.is_char() {
                    unsafe { self.transmute() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), STRSXP)) }
                }
            }

            /// Attempts to coerce storage mode to "double".
            pub fn to_char_mut<'a>(&'a mut self, pc: &'a Pc) -> &'a mut $name<char> {
                if self.is_char() {
                    unsafe { self.transmute_mut() }
                } else {
                    unsafe { pc.protect_and_transmute(Rf_coerceVector(self.sexp(), STRSXP)) }
                }
            }
        }
    };
}

rconvertable!(RScalar);
rconvertable!(RVector);
rconvertable!(RMatrix);
rconvertable!(RArray);

pub trait RSliceable<T> {
    fn slice(&self) -> &[T];

    fn slice_mut(&mut self) -> &mut [T];
}

macro_rules! rsliceable {
    ($name:ident, $tipe:ty, $tipe2:ty, $ptr:expr) => {
        impl RSliceable<$tipe2> for $name<$tipe> {
            fn slice(&self) -> &[$tipe2] {
                unsafe { std::slice::from_raw_parts($ptr(self.sexp()), self.len()) }
            }

            fn slice_mut(&mut self) -> &mut [$tipe2] {
                unsafe { std::slice::from_raw_parts_mut($ptr(self.sexp()), self.len()) }
            }
        }
    };
}

rsliceable!(RScalar, f64, f64, REAL);
rsliceable!(RScalar, i32, i32, INTEGER);
rsliceable!(RScalar, u8, u8, RAW);
rsliceable!(RScalar, bool, i32, LOGICAL);
rsliceable!(RVector, f64, f64, REAL);
rsliceable!(RVector, i32, i32, INTEGER);
rsliceable!(RVector, u8, u8, RAW);
rsliceable!(RVector, bool, i32, LOGICAL);
rsliceable!(RMatrix, f64, f64, REAL);
rsliceable!(RMatrix, i32, i32, INTEGER);
rsliceable!(RMatrix, u8, u8, RAW);
rsliceable!(RMatrix, bool, i32, LOGICAL);
rsliceable!(RArray, f64, f64, REAL);
rsliceable!(RArray, i32, i32, INTEGER);
rsliceable!(RArray, u8, u8, RAW);
rsliceable!(RArray, bool, i32, LOGICAL);

pub trait R2FromIterator2<T> {
    #[allow(clippy::mut_from_ref)]
    fn from_iter1<S>(iter: S, pc: &Pc) -> &mut Self
    where
        S: IntoIterator<Item = T> + ExactSizeIterator;

    #[allow(clippy::mut_from_ref)]
    fn from_iter2<'a, 'b, S>(iter: S, pc: &'a Pc) -> &'a mut Self
    where
        S: IntoIterator<Item = &'b T> + ExactSizeIterator,
        T: 'b;
}

pub trait RGetSet0<T> {
    /// Get the value at a certain index in an $tipe RVector.
    fn get(&self) -> T;

    /// Set the value at a certain index in an $tipe RVector.
    fn set(&mut self, value: T);
}

pub trait RGetSet1<T> {
    /// Get the value at a certain index in an $tipe RVector.
    fn get(&self, index: usize) -> Result<T, &'static str>;

    /// Set the value at a certain index in an $tipe RVector.
    fn set(&mut self, index: usize, value: T) -> Result<(), &'static str>;
}

pub trait RGetSet2<T> {
    /// Get the value at a certain index in an $tipe RVector.
    fn get(&self, row: usize, col: usize) -> Result<T, &'static str>;

    /// Set the value at a certain index in an $tipe RVector.
    fn set(&mut self, row: usize, col: usize, value: T) -> Result<(), &'static str>;
}

pub trait RGetSetN<T> {
    /// Get the value at a certain index in an $tipe RVector.
    fn get(&self, index: &[usize]) -> Result<T, &'static str>;

    /// Set the value at a certain index in an $tipe RVector.
    fn set(&mut self, index: &[usize], value: T) -> Result<(), &'static str>;
}

macro_rules! r2scalar_getset2 {
    ($tipe:ty, $tipe2:ty, $get:expr, $set:expr) => {
        impl RGetSet0<$tipe2> for RScalar<$tipe> {
            fn get(&self) -> $tipe2 {
                unsafe { $get(self.sexp(), 0) }
            }

            fn set(&mut self, value: $tipe2) {
                unsafe { $set(self.sexp(), 0, value) }
            }
        }
    };
}

r2scalar_getset2!(f64, f64, REAL_ELT, SET_REAL_ELT);
r2scalar_getset2!(i32, i32, INTEGER_ELT, SET_INTEGER_ELT);
r2scalar_getset2!(u8, u8, RAW_ELT, SET_RAW_ELT);

impl RGetSet0<bool> for RScalar<bool> {
    fn get(&self) -> bool {
        R::is_true(unsafe { LOGICAL_ELT(self.sexp(), 0) })
    }

    fn set(&mut self, value: bool) {
        unsafe { SET_LOGICAL_ELT(self.sexp(), 0, R::as_logical(value)) }
    }
}

impl RScalar<char> {
    pub fn get(&self) -> Result<&str, &'static str> {
        let sexp = unsafe { STRING_ELT(self.sexp(), 0) };
        sexp.as_str()
    }

    pub fn set(&mut self, value: &str, pc: &Pc) {
        unsafe { SET_STRING_ELT(self.sexp(), 0, SEXP::from_str(value, pc)) }
    }
}

pub trait RVectorConstructors<T> {
    #[allow(clippy::mut_from_ref)]
    fn new(length: usize, pc: &Pc) -> &mut Self;

    #[allow(clippy::mut_from_ref)]
    fn from_value(value: T, length: usize, pc: &Pc) -> &mut Self;

    #[allow(clippy::mut_from_ref)]
    fn from_array<const N: usize>(slice: [T; N], pc: &Pc) -> &mut Self;

    #[allow(clippy::mut_from_ref)]
    fn from_slice<'a>(slice: &[T], pc: &'a Pc) -> &'a mut Self;
}

macro_rules! r2vector2 {
    ($tipe:ty, $code:expr, $ptr:expr, $get:expr, $set:expr) => {
        impl R2FromIterator2<$tipe> for RVector<$tipe> {
            fn from_iter1<T>(iter: T, pc: &Pc) -> &mut Self
            where
                T: IntoIterator<Item = $tipe> + ExactSizeIterator,
            {
                let result = Self::new(iter.len(), pc);
                let slice = result.slice_mut();
                for (s, d) in slice.iter_mut().zip(iter) {
                    *s = d;
                }
                result
            }

            fn from_iter2<'a, 'b, T>(iter: T, pc: &'a Pc) -> &'a mut Self
            where
                T: IntoIterator<Item = &'b $tipe> + ExactSizeIterator,
            {
                let result = Self::new(iter.len(), pc);
                let slice = result.slice_mut();
                for (s, d) in slice.iter_mut().zip(iter) {
                    *s = *d;
                }
                result
            }
        }

        impl RGetSet1<$tipe> for RVector<$tipe> {
            /// Get the value at a certain index in an $tipe RVector.
            fn get(&self, index: usize) -> Result<$tipe, &'static str> {
                if index < self.len() {
                    Ok(unsafe { $get(self.sexp(), index.try_into().unwrap()) })
                } else {
                    Err("Index out of bounds.")
                }
            }

            /// Set the value at a certain index in an $tipe RVector.
            fn set(&mut self, index: usize, value: $tipe) -> Result<(), &'static str> {
                if index < self.len() {
                    unsafe { $set(self.sexp(), index.try_into().unwrap(), value) };
                    Ok(())
                } else {
                    Err("Index out of bounds.")
                }
            }
        }

        impl RVectorConstructors<$tipe> for RVector<$tipe> {
            fn new(length: usize, pc: &Pc) -> &mut Self {
                unsafe {
                    pc.protect_and_transmute(Rf_allocVector(
                        $code,
                        length.try_into().stop_str(TOO_LONG),
                    ))
                }
            }

            fn from_value(value: $tipe, length: usize, pc: &Pc) -> &mut Self {
                let result = Self::new(length, pc);
                let slice = result.slice_mut();
                slice.fill(value);
                result
            }

            fn from_array<const N: usize>(slice: [$tipe; N], pc: &Pc) -> &mut Self {
                let result = Self::new(slice.len(), pc);
                let slice2 = result.slice_mut();
                slice2.copy_from_slice(slice.as_ref());
                result
            }

            fn from_slice<'a>(slice: &[$tipe], pc: &'a Pc) -> &'a mut Self {
                let result = Self::new(slice.len(), pc);
                let slice2 = result.slice_mut();
                slice2.copy_from_slice(slice);
                result
            }
        }
    };
}

r2vector2!(f64, REALSXP, REAL, REAL_ELT, SET_REAL_ELT);
r2vector2!(i32, INTSXP, INTEGER, INTEGER_ELT, SET_INTEGER_ELT);
r2vector2!(u8, RAWSXP, RAW, RAW_ELT, SET_RAW_ELT);

impl R2FromIterator2<bool> for RVector<bool> {
    fn from_iter1<T>(iter: T, pc: &Pc) -> &mut Self
    where
        T: IntoIterator<Item = bool> + ExactSizeIterator,
    {
        let result = Self::new(iter.len(), pc);
        let slice = result.slice_mut();
        for (s, d) in slice.iter_mut().zip(iter) {
            *s = d as i32;
        }
        result
    }

    fn from_iter2<'a, 'b, T>(iter: T, pc: &'a Pc) -> &'a mut Self
    where
        T: IntoIterator<Item = &'b bool> + ExactSizeIterator,
    {
        let result = Self::new(iter.len(), pc);
        let slice = result.slice_mut();
        for (s, d) in slice.iter_mut().zip(iter) {
            *s = *d as i32;
        }
        result
    }
}

impl RGetSet1<bool> for RVector<bool> {
    /// Get the value at a certain index in an $tipe RVector.
    fn get(&self, index: usize) -> Result<bool, &'static str> {
        if index < self.len() {
            let value = unsafe { LOGICAL_ELT(self.sexp(), index.try_into().unwrap()) };
            Ok(R::is_true(value))
        } else {
            Err("Index out of bounds.")
        }
    }

    /// Set the value at a certain index in an $tipe RVector.
    fn set(&mut self, index: usize, value: bool) -> Result<(), &'static str> {
        if index < self.len() {
            unsafe {
                SET_LOGICAL_ELT(self.sexp(), index.try_into().unwrap(), R::as_logical(value))
            };
            Ok(())
        } else {
            Err("Index out of bounds.")
        }
    }
}

impl RVector<bool> {
    /// Get the value at a certain index in an $tipe RVector.
    pub fn get_i32(&self, index: usize) -> Result<i32, &'static str> {
        if index < self.len() {
            Ok(unsafe { LOGICAL_ELT(self.sexp(), index.try_into().unwrap()) })
        } else {
            Err("Index out of bounds.")
        }
    }

    /// Set the value at a certain index in an $tipe RVector.
    pub fn set_i32(&mut self, index: usize, value: i32) -> Result<(), &'static str> {
        if index < self.len() {
            unsafe { SET_LOGICAL_ELT(self.sexp(), index.try_into().unwrap(), value) };
            Ok(())
        } else {
            Err("Index out of bounds.")
        }
    }
}

impl RVectorConstructors<bool> for RVector<bool> {
    fn new(length: usize, pc: &Pc) -> &mut Self {
        unsafe {
            pc.protect_and_transmute(Rf_allocVector(LGLSXP, length.try_into().stop_str(TOO_LONG)))
        }
    }

    fn from_value(value: bool, length: usize, pc: &Pc) -> &mut Self {
        let result = Self::new(length, pc);
        let slice = result.slice_mut();
        slice.fill(value as i32);
        result
    }

    fn from_array<const N: usize>(slice: [bool; N], pc: &Pc) -> &mut Self {
        Self::from_iter2(slice.iter(), pc)
    }

    fn from_slice<'a>(slice: &[bool], pc: &'a Pc) -> &'a mut Self {
        Self::from_iter2(slice.iter(), pc)
    }
}

impl RVector<char> {
    pub fn get(&self, index: usize) -> Result<&str, &'static str> {
        if index < self.len() {
            self.get_unchecked(index)
        } else {
            Err("Index out of bounds.")
        }
    }

    pub fn set(&mut self, index: usize, value: &str) -> Result<(), &'static str> {
        if index < self.len() {
            self.set_unchecked(index, value);
            Ok(())
        } else {
            Err("Index out of bounds.")
        }
    }

    fn get_unchecked(&self, index: usize) -> Result<&str, &'static str> {
        let sexp = unsafe { STRING_ELT(self.sexp(), index.try_into().unwrap()) };
        let c_str = unsafe { CStr::from_ptr(R_CHAR(sexp) as *const c_char) };
        c_str.to_str().map_err(|_| "Not valid UTF8.")
    }

    fn set_unchecked(&mut self, index: usize, value: &str) {
        let sexp = unsafe {
            // Doesn't need protection because it's immediately set to a protected object. // DBD?
            Rf_mkCharLenCE(
                value.as_ptr() as *const c_char,
                value.len().try_into().unwrap(),
                cetype_t_CE_UTF8,
            )
        };
        unsafe { SET_STRING_ELT(self.sexp(), index.try_into().unwrap(), sexp) };
    }
}

impl RVectorConstructors<&str> for RVector<char> {
    #[allow(clippy::mut_from_ref)]
    fn new(length: usize, pc: &Pc) -> &mut Self {
        unsafe {
            pc.protect_and_transmute(Rf_allocVector(STRSXP, length.try_into().stop_str(TOO_LONG)))
        }
    }

    #[allow(clippy::mut_from_ref)]
    fn from_value<'a>(value: &str, length: usize, pc: &'a Pc) -> &'a mut Self {
        let length_i32 = length.try_into().stop_str(TOO_LONG);
        let vec = pc.protect(unsafe { Rf_allocVector(STRSXP, length_i32) });
        if length_i32 > 0 {
            let element = pc.protect(unsafe {
                Rf_mkCharLenCE(
                    value.as_ptr() as *const c_char,
                    value.len().try_into().unwrap(),
                    cetype_t_CE_UTF8,
                )
            });
            unsafe { SET_STRING_ELT(vec, 0, element) };
            for index in 1..length_i32 {
                unsafe { SET_STRING_ELT(vec, index, pc.protect(Rf_duplicate(element))) };
            }
        }
        unsafe { vec.transmute_mut(pc) }
    }

    #[allow(clippy::mut_from_ref)]
    fn from_array<'a, const N: usize>(array: [&str; N], pc: &'a Pc) -> &'a mut Self {
        let result = Self::new(array.len(), pc);
        for (index, value) in array.iter().enumerate() {
            result.set_unchecked(index, value)
        }
        result
    }

    #[allow(clippy::mut_from_ref)]
    fn from_slice<'a>(slice: &[&str], pc: &'a Pc) -> &'a mut Self {
        let result = Self::new(slice.len(), pc);
        for (index, value) in slice.iter().enumerate() {
            result.set_unchecked(index, value)
        }
        result
    }
}

pub trait RMatrixConstructors<T> {
    #[allow(clippy::mut_from_ref)]
    fn new(nrow: usize, ncol: usize, pc: &Pc) -> &mut Self;

    #[allow(clippy::mut_from_ref)]
    fn from_value(value: T, nrow: usize, ncol: usize, pc: &Pc) -> &mut Self;
}

macro_rules! r2matrix2 {
    ($tipe:ty, $code:expr, $get:expr, $set:expr) => {
        impl RGetSet2<$tipe> for RMatrix<$tipe> {
            /// Get the value at a certain index in an $tipe RMatrix.
            fn get(&self, row: usize, col: usize) -> Result<$tipe, &'static str> {
                let index = self.index(row, col, None);
                if index < self.len() {
                    Ok(unsafe { $get(self.sexp(), index.try_into().unwrap()) })
                } else {
                    Err("Index out of bounds.")
                }
            }

            /// Set the value at a certain index in an $tipe RMatrix.
            fn set(&mut self, row: usize, col: usize, value: $tipe) -> Result<(), &'static str> {
                let index = self.index(row, col, None);
                if index < self.len() {
                    unsafe { $set(self.sexp(), index.try_into().unwrap(), value) };
                    Ok(())
                } else {
                    Err("Index out of bounds.")
                }
            }
        }

        impl RMatrixConstructors<$tipe> for RMatrix<$tipe> {
            fn new(nrow: usize, ncol: usize, pc: &Pc) -> &mut Self {
                unsafe {
                    pc.protect_and_transmute(Rf_allocMatrix(
                        $code,
                        nrow.try_into().stop_str(TOO_LONG),
                        ncol.try_into().stop_str(TOO_LONG),
                    ))
                }
            }

            fn from_value(value: $tipe, nrow: usize, ncol: usize, pc: &Pc) -> &mut Self {
                let result = Self::new(nrow, ncol, pc);
                let slice = result.slice_mut();
                slice.fill(value);
                result
            }
        }
    };
}

r2matrix2!(f64, REALSXP, REAL_ELT, SET_REAL_ELT);
r2matrix2!(i32, INTSXP, INTEGER_ELT, SET_INTEGER_ELT);
r2matrix2!(u8, RAWSXP, RAW_ELT, SET_RAW_ELT);

impl<T> RMatrix<T> {
    /// Returns the number of rows in the RMatrix.
    pub fn nrow(&self) -> usize {
        unsafe { Rf_nrows(self.sexp()).try_into().unwrap() }
    }

    /// Returns the number of columns in the RMatrix.
    pub fn ncol(&self) -> usize {
        unsafe { Rf_ncols(self.sexp()).try_into().unwrap() }
    }

    /// Returns the dimensions of the RMatrix.
    pub fn dim(&self) -> [usize; 2] {
        [self.nrow(), self.ncol()]
    }

    /// Transpose the matrix.
    #[allow(clippy::mut_from_ref)]
    pub fn transpose<'a>(&self, pc: &'a Pc) -> &'a mut Self {
        let transposed = self.clone(pc);
        let mut dim = transposed.dim();
        dim.swap(0, 1);
        let dim2: [i32; 2] = [dim[0].try_into().unwrap(), dim[1].try_into().unwrap()];
        transposed.set_attribute(RSymbol::dim(), dim2.to_r(pc));
        unsafe { Rf_copyMatrix(transposed.sexp(), self.sexp(), Rboolean_TRUE) };
        transposed
    }

    /// Manipulates the matrix in place to be a vector by dropping the `dim` attribute.
    pub fn to_vector(&mut self) -> &mut RVector<T> {
        unsafe { Rf_setAttrib(self.sexp(), R_DimSymbol, R_NilValue).transmute_mut(self) }
    }

    /// Get the index of a value based on the row and column number.
    pub fn index(&self, row: usize, col: usize, nrow: Option<usize>) -> usize {
        let nrow = nrow.unwrap_or_else(|| self.nrow());
        nrow * col + row
    }

    /// Get the dimnames of a matrix.
    pub fn get_dimnames(&self) -> &RVector<char> {
        unsafe { Rf_getAttrib(self.sexp(), R_DimNamesSymbol).transmute(self) }
    }

    /// Set the dimnames of a matrix.
    pub fn set_dimnames(&mut self, dimnames: &RList) -> Result<(), &'static str> {
        match dimnames.get(0) {
            Ok(rownames) => match rownames.as_vector() {
                Ok(rownames) => {
                    if rownames.len() != self.nrow() {
                        return Err("Row names do not match the number of rows.");
                    }
                }
                Err(_) => {
                    return Err("Row names must be a character vector.");
                }
            },
            Err(_) => return Err("No row names element found."),
        };
        match dimnames.get(1) {
            Ok(colnames) => match colnames.as_vector() {
                Ok(colnames) => {
                    if colnames.len() != self.ncol() {
                        return Err("Column names do not match the number of columns.");
                    }
                }
                Err(_) => {
                    return Err("Column names must be a character vector.");
                }
            },
            Err(_) => return Err("No column names element found."),
        };
        unsafe {
            Rf_dimnamesgets(self.sexp(), dimnames.sexp());
        }
        Ok(())
    }
}

pub trait RArrayConstructors<T> {
    #[allow(clippy::mut_from_ref)]
    fn new<'a>(dim: &[usize], pc: &'a Pc) -> &'a mut Self;

    #[allow(clippy::mut_from_ref)]
    fn from_value<'a>(value: T, dim: &[usize], pc: &'a Pc) -> &'a mut Self;
}

macro_rules! r2array2 {
    ($tipe:ty, $code:expr, $get:expr, $set:expr) => {
        impl RGetSetN<$tipe> for RArray<$tipe> {
            /// Get the value at a certain index in an $tipe RArray.
            fn get(&self, index: &[usize]) -> Result<$tipe, &'static str> {
                let index = self.index(index, None);
                if index < self.len() {
                    Ok(unsafe { $get(self.sexp(), index.try_into().unwrap()) })
                } else {
                    Err("Index out of bounds.")
                }
            }
            /// Set the value at a certain index in an $tipe RArray.
            fn set(&mut self, index: &[usize], value: $tipe) -> Result<(), &'static str> {
                let index = self.index(index, None);
                if index < self.len() {
                    unsafe { $set(self.sexp(), index.try_into().unwrap(), value) };
                    Ok(())
                } else {
                    Err("Index out of bounds.")
                }
            }
        }
        impl RArrayConstructors<$tipe> for RArray<$tipe> {
            fn new<'a>(dim: &[usize], pc: &'a Pc) -> &'a mut Self {
                let dim = dim
                    .iter()
                    .map(|x| i32::try_from(*x).stop_str(TOO_LONG))
                    .to_r(pc)
                    .sexp();
                unsafe { pc.protect_and_transmute(Rf_allocArray($code, dim)) }
            }
            fn from_value<'a>(value: $tipe, dim: &[usize], pc: &'a Pc) -> &'a mut Self {
                let result = Self::new(dim, pc);
                let slice = result.slice_mut();
                slice.fill(value);
                result
            }
        }
    };
}

r2array2!(f64, REALSXP, REAL_ELT, SET_REAL_ELT);
r2array2!(i32, INTSXP, INTEGER_ELT, SET_INTEGER_ELT);
r2array2!(u8, RAWSXP, RAW_ELT, SET_RAW_ELT);

impl<T> RArray<T> {
    /// Returns the dimensions of the RMatrix.
    pub fn dim(&self) -> Vec<usize> {
        let d: &RVector<i32> = unsafe { Rf_getAttrib(self.sexp(), R_DimSymbol).transmute(self) };
        d.slice().iter().map(|&x| x.try_into().unwrap()).collect()
    }

    /// Manipulates the matrix in place to be a vector by dropping the `dim` attribute.
    pub fn to_vector(&mut self) -> &mut RVector<T> {
        unsafe { Rf_setAttrib(self.sexp(), R_DimSymbol, R_NilValue).transmute_mut(self) }
    }

    /// Get the index of a value based on the row and column number.
    pub fn index(&self, index: &[usize], dim: Option<&[usize]>) -> usize {
        let mut dim_holder: Vec<_> = vec![];
        let dim = dim.unwrap_or_else(|| {
            dim_holder = self.dim();
            &dim_holder[..]
        });
        let coef = std::iter::once(&1)
            .chain(dim[..dim.len() - 1].iter())
            .scan(1, |prod, d| {
                *prod *= *d;
                Some(*prod)
            });
        let mut i = 0;
        for (x, y) in coef.zip(index.iter()) {
            i += x * (*y);
        }
        i
    }

    /// Get the dimnames of a matrix.
    pub fn get_dimnames(&self) -> &RVector<char> {
        unsafe { Rf_getAttrib(self.sexp(), R_DimNamesSymbol).transmute(self) }
    }

    /// Set the dimnames of a matrix.
    pub fn set_dimnames(&mut self, dimnames: &RList) -> Result<(), String> {
        let dim = self.dim();
        if dimnames.len() != dim.len() {
            return Err(format!(
                "Length of dimnames is {} whereas the dim is of length {}.",
                dimnames.len(),
                dim.len()
            ));
        }
        for (i, &len) in dim.iter().enumerate() {
            match dimnames.get(i).unwrap().as_vector() {
                Ok(names) => {
                    if names.len() != len {
                        return Err(format!("Element {} of the dimnames list has length {}, but the corresponding dimension is {}.", i, names.len(), len));
                    }
                }
                Err(_) => {
                    return Err(format!(
                        "Element {} of the dimnames list must be a character vector.",
                        i
                    ));
                }
            }
        }
        unsafe {
            Rf_dimnamesgets(self.sexp(), dimnames.sexp());
        }
        Ok(())
    }
}

pub struct R2ListMap2<'a> {
    unused_counter: usize,
    used: Vec<bool>,
    robj: &'a RList,
    map: HashMap<&'a str, usize>,
}

impl R2ListMap2<'_> {
    /// Find an RObject in the list based on its name.
    pub fn get(&mut self, name: &str) -> Result<&RObject, String> {
        let Some(index) = self.map.get(name) else {
            return Err(format!("'{}' not found.", name));
        };
        if !self.used[*index] {
            self.unused_counter -= 1;
            self.used[*index] = true;
        }
        Ok(self.robj.get(*index)?)
    }

    /// Check to see if every RObject in the map has been used.
    pub fn exhaustive(&self) -> Result<(), String> {
        if self.unused_counter != 0 {
            return Err(format!(
                "Unrecognized elements in list:\n    {}.",
                self.unused_elements().join("\n    ")
            ));
        }
        Ok(())
    }

    /// Return the number of unused RObjects in the map.
    pub fn unused_counter(&self) -> usize {
        self.unused_counter
    }

    /// Return the names of all unused RObjects in the map.
    pub fn unused_elements(&self) -> Vec<&str> {
        let result = self
            .map
            .iter()
            .filter(|(_, index)| !self.used[**index])
            .map(|(name, _)| *name)
            .take(self.unused_counter);
        result.collect()
    }
}

macro_rules! rlistlike {
    ($name:ident) => {
        impl $name {
            /// Get the value at a certain index in an $tipe RVector.
            pub fn get(&self, index: usize) -> Result<&RObject, &'static str> {
                if index < self.len() {
                    Ok(unsafe {
                        VECTOR_ELT(self.sexp(), index.try_into().unwrap()).transmute(self)
                    })
                } else {
                    Err("Index out of bounds.")
                }
            }

            pub fn get_mut(&mut self, index: usize) -> Result<&mut RObject, &'static str> {
                if index < self.len() {
                    Ok(unsafe {
                        VECTOR_ELT(self.sexp(), index.try_into().unwrap()).transmute_mut(self)
                    })
                } else {
                    Err("Index out of bounds.")
                }
            }

            /// Set the value at a certain index in an $tipe RVector.
            pub fn set(
                &mut self,
                index: usize,
                value: &impl IsRObject,
            ) -> Result<(), &'static str> {
                if index < self.len() {
                    unsafe { SET_VECTOR_ELT(self.sexp(), index.try_into().unwrap(), value.sexp()) };
                    Ok(())
                } else {
                    Err("Index out of bounds.")
                }
            }

            /// Get a value from the RList based on its key.
            pub fn get_by_key(&self, key: impl AsRef<str>) -> Result<&RObject, String> {
                let names = self.get_names();
                for i in 0..names.len() {
                    if names.get(i).unwrap() == key.as_ref() {
                        return Ok(self.get(i)?);
                    }
                }
                Err(format!("Could not find '{}' in the list.", key.as_ref()))
            }

            /// Get a value from the RList based on its key.
            pub fn get_mut_by_key(&mut self, key: impl AsRef<str>) -> Result<&mut RObject, String> {
                let names = self.get_names();
                for i in 0..names.len() {
                    if names.get(i).unwrap() == key.as_ref() {
                        return Ok(self.get_mut(i)?);
                    }
                }
                Err(format!("Could not find '{}' in the list.", key.as_ref()))
            }

            /// Convert the list into an [RListMap]
            ///
            /// This allows Rust HashMap methods to be used on the contents
            /// of the list, while still retaining the original list within
            /// the RListMap struct in the robj field.
            pub fn make_map(&self) -> R2ListMap2 {
                let mut map = HashMap::new();
                let names = self.get_names();
                let len = names.len();
                for i in 0..len {
                    map.insert(names.get(i).unwrap(), i);
                }
                R2ListMap2 {
                    unused_counter: len,
                    used: vec![false; len],
                    robj: unsafe { self.transmute() },
                    map,
                }
            }
        }
    };
}

rlistlike!(RList);
rlistlike!(RDataFrame);

impl RList {
    #[allow(clippy::mut_from_ref)]
    pub fn new(length: usize, pc: &Pc) -> &mut Self {
        unsafe {
            pc.protect_and_transmute(Rf_allocVector(VECSXP, length.try_into().stop_str(TOO_LONG)))
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn with_names<'a>(names: &[&str], pc: &'a Pc) -> &'a mut Self {
        let result = Self::new(names.len(), pc);
        unsafe {
            Rf_namesgets(result.sexp(), names.to_r(pc).sexp());
        }
        result
    }
}

impl RExternalPtr {
    /// Move Rust object to an R external pointer.
    ///
    /// This *method* moves a Rust object to an R external pointer and then, as far as Rust is concerned, leaks the memory.
    /// Thus the programmer is then responsible to release the memory by calling [`RObject::decode_val`].
    ///
    #[allow(clippy::mut_from_ref)]
    pub fn encode<'a, T>(x: T, tag: &str, pc: &'a Pc) -> &'a mut Self {
        Self::encode_full(x, tag.to_r(pc), true, pc)
    }

    /// Move Rust object to an R external pointer.
    ///
    /// This *method* moves a Rust object to an R external pointer and then, as far as Rust is concerned, leaks the memory.
    /// Thus the programmer is then responsible to release the memory by calling [`RObject::decode_val`].
    ///
    #[allow(clippy::mut_from_ref)]
    pub fn encode_full<'a, T>(
        x: T,
        tag: &impl IsRObject,
        managed_by_r: bool,
        pc: &'a Pc,
    ) -> &'a mut Self {
        unsafe {
            let ptr = Box::into_raw(Box::new(x));
            let sexp = pc.protect(R_MakeExternalPtr(
                ptr as *mut c_void,
                tag.sexp(),
                R_NilValue,
            ));
            if managed_by_r {
                unsafe extern "C" fn free<S>(sexp: SEXP) {
                    let addr = R_ExternalPtrAddr(sexp);
                    if addr.as_ref().is_none() {
                        return;
                    }
                    let _ = Box::from_raw(addr as *mut S);
                    R_ClearExternalPtr(sexp);
                }
                Rf_setAttrib(sexp, R_AtsignSymbol, R_AtsignSymbol);
                R_RegisterCFinalizerEx(sexp, Some(free::<T>), 0);
            }
            sexp.transmute_mut(pc)
        }
    }

    /// Check if an external pointer is managed by R.
    pub fn is_managed_by_r(&self) -> bool {
        unsafe { Rf_getAttrib(self.sexp(), R_AtsignSymbol) == R_AtsignSymbol }
    }

    /// Move an R external pointer to a Rust object.
    ///
    /// This method moves an R external pointer created by [`RObject::external_ptr`] to a Rust object and Rust will then manage its memory.
    ///
    pub fn decode_val<T>(&self) -> Result<T, &'static str> {
        if self.is_managed_by_r() {
            return Err("External pointer is managed by R.");
        }
        unsafe {
            let addr = R_ExternalPtrAddr(self.sexp());
            if addr.as_ref().is_none() {
                return Err("External pointer was already decoded by value.");
            }
            R_ClearExternalPtr(self.sexp());
            Ok(*Box::from_raw(addr as *mut T))
        }
    }

    /// Obtain a reference to a Rust object from an R external pointer.
    ///
    /// This method obtains a reference to a Rust object from an R external pointer created by [`RObject::external_ptr`].
    ///
    pub fn decode_ref<T>(&self) -> &T {
        unsafe {
            let ptr = R_ExternalPtrAddr(self.sexp()) as *mut T;
            ptr.as_ref().unwrap()
        }
    }

    /// Obtain a reference to a Rust object from an R external pointer, pretending a static lifetime.
    ///
    /// This method obtains a reference to a Rust object from an R external pointer created by [`RObject::external_ptr`].
    ///
    /// # Safety
    ///
    /// Despite the use of a static lifetime here, the reference is only valid as long as R's
    /// garbage collector has not reclaimed the underlying object's memory.
    pub unsafe fn decode_ref_static<T>(&self) -> &'static T {
        let ptr = R_ExternalPtrAddr(self.sexp()) as *mut T;
        ptr.as_ref().unwrap()
    }

    /// Obtain a mutable reference to a Rust object from an R external pointer.
    ///
    /// This method obtains a mutable reference to a Rust object from an R external pointer created by [`RObject::external_ptr`].
    ///
    pub fn decode_mut<'a, T>(&mut self) -> &'a mut T {
        unsafe {
            let ptr = R_ExternalPtrAddr(self.sexp()) as *mut T;
            ptr.as_mut().unwrap()
        }
    }

    /// Obtain a mutable reference to a Rust object from an R external pointer, pretending a static lifetime.
    ///
    /// This method obtains a mutable reference to a Rust object from an R external pointer created by [`RObject::external_ptr`].
    ///
    /// # Safety
    ///
    /// Despite the use of a static lifetime here, the reference is only valid as long as R's
    /// garbage collector has not reclaimed the underlying object's memory.
    pub unsafe fn decode_mut_static<T>(&mut self) -> &'static mut T {
        let ptr = R_ExternalPtrAddr(self.sexp()) as *mut T;
        ptr.as_mut().unwrap()
    }

    /// Get the memory address of the external pointer.
    pub fn address(&self) -> *mut c_void {
        unsafe { R_ExternalPtrAddr(self.sexp()) }
    }

    /// Register the external pointer to be finalized.
    ///
    /// This allows the object to perform cleanup actions when no longer referenced in R.
    ///
    pub fn register_finalizer(&self, func: extern "C" fn(sexp: SEXP)) -> Result<(), &'static str> {
        if self.is_managed_by_r() {
            return Err("External pointer is managed by R.");
        }
        unsafe {
            R_RegisterCFinalizerEx(self.sexp(), Some(func), 0);
            Ok(())
        }
    }

    /// Get tag for an R external pointer.
    ///
    /// This method gets the tag associated with an R external pointer, which was set by [`Self::external_ptr`].
    ///
    pub fn tag(&self) -> &RObject {
        unsafe { R_ExternalPtrTag(self.sexp()).transmute(self) }
    }
}

// Conversions
pub trait FromR<T: IsRObject, U> {
    fn from_r(x: &T, pc: &Pc) -> Result<Self, U>
    where
        Self: Sized;
}

/// Trait for converting objects to RObjects.
pub trait ToRScalar<RMode> {
    #[allow(clippy::mut_from_ref)]
    fn to_r(self, pc: &Pc) -> &mut RScalar<RMode>;
}

macro_rules! to_rscalar {
    ($tipe:ty, $tipe2:ty) => {
        impl<'a> ToRScalar<$tipe> for $tipe2 {
            fn to_r(self, pc: &Pc) -> &mut RScalar<$tipe> {
                RScalar::from_value(self, pc)
            }
        }
    };
}

to_rscalar!(f64, f64);
to_rscalar!(i32, i32);
to_rscalar!(u8, u8);
to_rscalar!(bool, bool);
to_rscalar!(char, &str);

impl ToRScalar<i32> for usize {
    fn to_r(self, pc: &Pc) -> &mut RScalar<i32> {
        RScalar::from_value(self.try_into().stop_str(TOO_LONG), pc)
    }
}

/// Trait for converting objects to RObjects.
pub trait ToRObject2 {
    #[allow(clippy::mut_from_ref)]
    fn to_r(self, pc: &Pc) -> &impl IsRObject;
}

/// Trait for converting objects to RObjects.
pub trait ToRObject2Mut {
    #[allow(clippy::mut_from_ref)]
    fn to_r(self, pc: &Pc) -> &mut impl IsRObject;
}

/// Trait for converting objects to RObjects.
pub trait ToRObject2Mut2 {
    #[allow(clippy::mut_from_ref)]
    fn to_r<'a>(&self, pc: &'a Pc) -> &'a mut impl IsRObject;
}

/// Trait for converting objects to RObjects.
pub trait ToRVector2<RMode> {
    #[allow(clippy::mut_from_ref)]
    fn to_r(self, pc: &Pc) -> &mut RVector<RMode>;
}

/// Trait for converting iterators to RObjects.
pub trait ToRVector22<RMode> {
    #[allow(clippy::mut_from_ref)]
    fn to_r(self, pc: &Pc) -> &mut RVector<RMode>;
}

pub trait ToRVector33<RMode> {
    #[allow(clippy::mut_from_ref)]
    fn to_r(self, pc: &Pc) -> &mut RVector<RMode>;
}

macro_rules! to_rvector2 {
    ($tipe:ty, $tipe2:ty) => {
        impl<'a, const N: usize> ToRVector22<$tipe> for [$tipe2; N] {
            fn to_r(self, pc: &Pc) -> &mut RVector<$tipe> {
                RVector::from_array(self, pc)
            }
        }
        impl<'a> ToRVector2<$tipe> for &[$tipe2] {
            fn to_r(self, pc: &Pc) -> &mut RVector<$tipe> {
                RVector::from_slice(self, pc)
            }
        }
    };
}

to_rvector2!(f64, f64);
to_rvector2!(i32, i32);
to_rvector2!(u8, u8);
to_rvector2!(bool, bool);
to_rvector2!(char, &str);

macro_rules! to_rvector24 {
    ($tipe:ty, $tipe2:ty) => {
        impl<'a, T: IntoIterator<Item = $tipe2> + ExactSizeIterator> ToRVector33<$tipe> for T {
            fn to_r(self, pc: &Pc) -> &mut RVector<$tipe> {
                RVector::from_iter1(self, pc)
            }
        }
    };
    ($tipe:ty, $tipe2:ty) => {
        impl<'a, T: IntoIterator<Item = &'a $tipe2> + ExactSizeIterator> ToRVector33<$tipe> for T {
            fn to_r(self, pc: &Pc) -> &mut RVector<$tipe> {
                RVector::from_iter2(self, pc)
            }
        }
    };
}

to_rvector24!(f64, f64);
to_rvector24!(i32, i32);
to_rvector24!(u8, u8);
to_rvector24!(bool, bool);

impl ToRObject2 for () {
    fn to_r(self, pc: &Pc) -> &impl IsRObject {
        unsafe { R_NilValue.transmute_mut::<RObject, Pc>(pc) }
    }
}

impl ToRObject2 for SEXP {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn to_r(self, pc: &Pc) -> &impl IsRObject {
        unsafe { self.transmute::<RObject, Pc>(pc) }
    }
}

impl<T: IsRObject> ToRObject2 for &T {
    fn to_r(self, pc: &Pc) -> &impl IsRObject {
        unsafe { self.sexp().transmute::<RObject, Pc>(pc) }
    }
}

/// Print to the R console.
///
/// This is an implementation detail and *should not* be called directly!
/// This returns `true` if the print statement swallowed a user interrupt.
/// R checks for user interrupt every 100 print statements.
/// See the `Rvprintf` function in `printutils.c` of R's source.
///
#[doc(hidden)]
pub fn __private_print(x: &str, use_stdout: bool) -> bool {
    #[repr(C)]
    struct DummyFat {
        len: usize,
        ptr: *const c_char,
        use_stdout: bool,
    }
    let mut y = DummyFat {
        len: x.len(),
        ptr: x.as_ptr() as *const c_char,
        use_stdout,
    };
    let y_ptr = &mut y as *mut DummyFat as *mut c_void;
    extern "C" fn print_fn(y_ptr: *mut c_void) {
        unsafe {
            let y_ptr = y_ptr as *mut DummyFat;
            if (*y_ptr).use_stdout {
                Rprintf(
                    b"%.*s\0".as_ptr() as *const c_char,
                    (*y_ptr).len,
                    (*y_ptr).ptr,
                );
            } else {
                REprintf(
                    b"%.*s\0".as_ptr() as *const c_char,
                    (*y_ptr).len,
                    (*y_ptr).ptr,
                );
            }
        }
    }
    unsafe { R_ToplevelExec(Some(print_fn), y_ptr) == 0 }
}

#[macro_export]
macro_rules! roxido_registration {
    () => {
        mod registration {
            include!(concat!(env!("OUT_DIR"), "/registration.rs"));
        }
    };
}

/// Just like Rust's usual `print!` macro, except output goes to the R console.
#[macro_export]
macro_rules! rprint {
    ($fmt_string:expr) => {
        __private_print(format!($fmt_string).as_str(), true)
    };
    ($fmt_string:expr, $( $arg:expr ),* ) => {
        __private_print(format!($fmt_string, $($arg),*).as_str(), true)
    }
}

/// Just like Rust's usual `println!` macro, except output goes to the R console.
#[macro_export]
macro_rules! rprintln {
    () => {
        __private_print("\n", true)
    };
    ($fmt_string:expr) => {
        __private_print(format!(concat!($fmt_string,"\n")).as_str(), true)
    };
    ($fmt_string:expr, $( $arg:expr ),* ) => {
        __private_print(format!(concat!($fmt_string,"\n"), $($arg),*).as_str(), true)
    }
}

/// Just like Rust's usual `eprint!` macro, except output goes to the R console.
#[macro_export]
macro_rules! reprint {
    ($fmt_string:expr) => {
        __private_print(format!($fmt_string).as_str(), false)
    };
    ($fmt_string:expr, $( $arg:expr ),* ) => {
        __private_print(format!($fmt_string, $($arg),*).as_str(), false)
    }
}

/// Just like Rust's usual `eprintln!` macro, except output goes to the R console.
#[macro_export]
macro_rules! reprintln {
    () => {
        __private_print("\n", false)
    };
    ($fmt_string:expr) => {
        __private_print(format!(concat!($fmt_string,"\n")).as_str(), false)
    };
    ($fmt_string:expr, $( $arg:expr ),* ) => {
        __private_print(format!(concat!($fmt_string,"\n"), $($arg),*).as_str(), false)
    }
}

use std::fmt::Display;

#[doc(hidden)]
pub struct RStopHelper(pub String);

/// Throw an R error.
#[macro_export]
#[allow(clippy::crate_in_macro_def)]
macro_rules! stop {
    () => {
        match std::env::var("RUST_BACKTRACE") {
            Ok(_) => {
                panic!("Panic in stop!() due to RUST_BACKTRACE environment variable.")
            },
            Err(_) => {
                std::panic::panic_any(RStopHelper(String::new()))
            }
        }
    };
    ($fmt_string:expr) => {
        match std::env::var("RUST_BACKTRACE") {
            Ok(_) => {
                let mut msg = String::new();
                msg.push_str("Panic in stop!(...) due to RUST_BACKTRACE environment variable: ");
                msg.push_str(&format!($fmt_string));
                panic!("{}", msg);
            },
            Err(_) => {
                std::panic::panic_any(RStopHelper(format!($fmt_string)))
            }
        }
    };
    ($fmt_string:expr, $( $arg:expr ),* ) => {
        match std::env::var("RUST_BACKTRACE") {
            Ok(_) => {
                let mut msg = String::new();
                msg.push_str("Panic in stop!(...) due to RUST_BACKTRACE environment variable: ");
                msg.push_str(&format!($fmt_string, $($arg),*));
                panic!("{}", msg);
            },
            Err(_) => {
                std::panic::panic_any(RStopHelper(format!($fmt_string, $($arg),*)))
            }
        }
    }
}

pub trait UnwrapOrStop<T> {
    fn stop(self) -> T;
    fn stop_str(self, msg: &str) -> T;
    fn stop_closure(self, msg: impl FnMut() -> String) -> T;
}

impl<T, S: Display> UnwrapOrStop<T> for Result<T, S> {
    fn stop(self) -> T {
        match self {
            Ok(t) => t,
            Err(e) => stop!("{}", e),
        }
    }
    fn stop_str(self, msg: &str) -> T {
        match self {
            Ok(t) => t,
            Err(_) => stop!("{}", msg),
        }
    }
    fn stop_closure<'a>(self, mut msg: impl FnMut() -> String) -> T {
        match self {
            Ok(t) => t,
            Err(_) => stop!("{}", msg()),
        }
    }
}

impl<T> UnwrapOrStop<T> for Option<T> {
    fn stop(self) -> T {
        match self {
            Some(t) => t,
            None => stop!(),
        }
    }
    fn stop_str(self, msg: &str) -> T {
        match self {
            Some(t) => t,
            None => stop!("{}", msg),
        }
    }
    fn stop_closure<'a>(self, mut msg: impl FnMut() -> String) -> T {
        match self {
            Some(t) => t,
            None => stop!("{}", msg()),
        }
    }
}

#[doc(hidden)]
#[no_mangle]
pub extern "C" fn __private_set_custom_panic_hook() -> SEXP {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        if panic_info.payload().downcast_ref::<RStopHelper>().is_none() {
            default_panic(panic_info);
        }
    }));
    unsafe { R_NilValue }
}
