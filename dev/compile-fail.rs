//! Code that must not compile. Run by test-thread-safety.R.
//!
//! R objects and protection counters stay on R's main thread.
//!
//! ```compile_fail
//! fn f<T: Send>() {}
//! f::<roxido::RObject>();
//! ```
//! ```compile_fail
//! fn f<T: Sync>() {}
//! f::<roxido::RObject>();
//! ```
//! ```compile_fail
//! fn f<T: Send>() {}
//! f::<roxido::RVector<f64>>();
//! ```
//! ```compile_fail
//! fn f<T: Sync>() {}
//! f::<roxido::RVector<f64>>();
//! ```
//! ```compile_fail
//! fn f<T: Sync>() {}
//! f::<roxido::Pc>();
//! ```
//!
//! Only Roxido's types are R objects.
//!
//! ```compile_fail
//! struct Fake;
//! impl roxido::RObjectVariant for Fake {}
//! ```
//!
//! Raw pointers, protection counters, and in-place arguments need `unsafe`.
//!
//! ```compile_fail
//! use roxido::*;
//! fn f(x: SEXP, pc: &Pc) { x.to_r(pc); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(x: SEXP, pc: &Pc) { pc.protect(x); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! #[roxido]
//! fn f(x: SEXP) { RObject::from_sexp(x, pc) }
//! ```
//! ```compile_fail
//! roxido::Pc::__private_new();
//! ```
//! ```compile_fail
//! use roxido::*;
//! #[roxido]
//! fn f() {}
//! f();
//! ```
//! ```compile_fail
//! use roxido::*;
//! #[roxido]
//! fn f(#[in_place] x: &mut [f64]) {}
//! ```
//! ```compile_fail
//! use roxido::*;
//! #[roxido]
//! fn f(#[unsafe(in_place)] x: &[f64]) {}
//! ```
//! ```compile_fail
//! use roxido::*;
//! #[roxido]
//! fn f(x: &mut RExternalPtr) {}
//! ```
//! ```compile_fail
//! use roxido::*;
//! #[roxido]
//! fn f(x: &mut RSymbol) {}
//! ```
//!
//! References cannot outlive the counter protecting their object.
//!
//! ```compile_fail
//! use roxido::*;
//! fn f(pc: &Pc) -> &RVector<f64> {
//!     pc.nested(|inner| RVector::from_value(1.0, 1, inner))
//! }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(pc: &Pc) -> &RObject {
//!     pc.nested(|inner| RVector::from_value(1.0, 1, inner).to_r(pc))
//! }
//! ```
//!
//! Attribute borrows cannot outlive their counter or permit mutation of their parent.
//!
//! ```compile_fail
//! use roxido::*;
//! fn f<'a>(x: &'a RObject, pc: &Pc) -> &'a RObject {
//!     pc.nested(|inner| x.get_attribute(RSymbol::names(), inner))
//! }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(x: &mut RObject, pc: &Pc) {
//!     let names = x.get_attribute(RSymbol::names(), pc);
//!     x.set_attribute(RSymbol::names(), &["new"][..]);
//!     let _ = names.is_null();
//! }
//! ```
//!
//! External pointers cannot retain stack borrows.
//!
//! ```compile_fail
//! use roxido::*;
//! fn f(pc: &Pc) {
//!     let x = 42;
//!     RExternalPtr::encode(&x, "i32", pc);
//! }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(pc: &Pc) {
//!     let x = 42;
//!     unsafe { RExternalPtr::encode_full(&x, R::null(), false, pc); }
//! }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &mut RExternalPtr) {
//!     let x = 42;
//!     unsafe { p.reencode(|_| &x); }
//! }
//! ```
//!
//! Unchecked pointer access requires unsafe and respects Rust borrows.
//!
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &RExternalPtr) { p.decode_ref::<i32>(); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &mut RExternalPtr) { p.decode_mut::<i32>(); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &mut RExternalPtr) { p.decode_val::<i32>(); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &mut RExternalPtr, addr: &mut std::ffi::c_void) {
//!     p.set_address(addr);
//! }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &RExternalPtr) { unsafe { p.decode_val::<i32>(); } }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &mut RExternalPtr) -> &'static mut i32 {
//!     unsafe { p.decode_mut() }
//! }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &mut RExternalPtr) {
//!     let value = unsafe { p.decode_ref::<String>() };
//!     let _ = unsafe { p.decode_val::<String>() };
//!     println!("{value}");
//! }
//! ```
//!
//! Publishing aliases requires unsafe, including attributes and external-pointer tags, and
//! `set_with_pc` closures cannot return existing objects.
//!
//! ```compile_fail
//! use roxido::*;
//! fn f(list: &mut RList, value: &RObject) { list.set(0, value); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(list: &mut RList, value: &RObject) { list.set_object(0, value); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(list: &mut RList, value: &RObject) { list.set_with_pc(0, |_| value); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(list: &mut RList, value: &RObject) { list.set_with_pc(0, |pc| value.to_r(pc)); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(list: &mut RList, value: &mut RVector<f64>) { list.set_loop_with_pc(|_| &*value); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(list: &mut RDataFrame, value: &RObject) { list.set_object(0, value); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(x: &mut RObject, names: &RVector<char>) { x.set_class_object(names); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(x: &mut RObject, key: &RSymbol, value: &RObject) {
//!     x.set_attribute(key, value);
//! }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(x: &mut RObject, key: &RSymbol, value: &RObject) {
//!     x.set_attribute_object(key, value);
//! }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(x: &mut RVector<f64>, names: &RVector<char>) { x.set_names_object(names); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(x: &mut RMatrix<f64>, names: &RList) { x.set_dimnames_object(names); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(x: &mut RArray<f64>, names: &RList) { x.set_dimnames_object(names); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(pc: &Pc) { RExternalPtr::encode_full(42_i32, R::null(), false, pc); }
//! ```
//!
//! Callbacks and replacing external-pointer allocations require unsafe.
//!
//! ```compile_fail
//! use roxido::*;
//! fn f(fun: &RFunction, pc: &Pc) { fun.call0(pc); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(fun: &RFunction, x: &RObject, pc: &Pc) { fun.call1(x, pc); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(fun: &RFunction, x: &RObject, pc: &Pc) { fun.call2(x, x, pc); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(fun: &RFunction, x: &RObject, pc: &Pc) { fun.call3(x, x, x, pc); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(fun: &RFunction, x: &RObject, pc: &Pc) { fun.call4(x, x, x, x, pc); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(fun: &RFunction, x: &RObject, pc: &Pc) { fun.call5(x, x, x, x, x, pc); }
//! ```
//! ```compile_fail
//! use roxido::*;
//! fn f(p: &mut RExternalPtr) { p.reencode(|_| 42_i32); }
//! ```
