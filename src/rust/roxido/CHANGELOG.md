# Changelog

## Unreleased

An R error inside R's API unwinds Rust. Arguments are modified through `RCow`, not `&mut`.

Breaking changes:

- `#[roxido]` functions cannot take `&mut` arguments or `#[unsafe(in_place)]`; the compiler
  says what to write instead. Declare `x: RCow<T>` and get write access with
  `x.into_mut_or_copy(pc)`, which copies only if R may still reference the object, as `&mut`
  did; `x.try_into_mut()`, which never copies; or `unsafe { x.into_mut_in_place() }`, as
  `#[unsafe(in_place)]` did. `into_mut_in_place` raises an R error for `NULL`, and for an object
  passed in any other argument, not only in a borrowed one.
- `to_f64_mut`, `to_i32_mut`, `to_u8_mut`, `to_bool_mut` and `to_char_mut` are removed. For
  another storage mode they wrote to a coerced copy, so writes to an integer vector through
  `to_f64_mut` were silently lost. Use `x.to_f64(pc).into_mut_or_copy(pc)` on an `RCow`.
- Arguments are converted by the new `FromRArg` trait instead of matching type names, so an
  unsupported type is a trait error and type aliases work. `Option<SEXP>` is rejected, since a
  `SEXP` can already be `NULL`.

New:

- An R error raised inside R's API, such as a failed allocation, an overflow of R's protection
  stack, an invalid `dim` or `class`, or a coercion warning under `options(warn = 2)`, unwinds
  Rust frames as a panic does. Destructors run and scoped threads are joined before R's jump
  continues, including for `tryCatch(f(x), warning = ...)` around a coercion that warns. A call
  that can raise one costs about 25 ns more.
- `r_try` intercepts R errors in raw calls through `rbindings`; `Pc::protect_new` also protects
  the object that the call makes.
- `RCow<T>` gives write access to an argument, copying it only if something else may reach it.
  A temporary passed directly to `.Call`, such as `x * 2`, is written without a copy. Its
  `to_f64`, `to_i32`, `to_u8`, `to_bool` and `to_char` keep it fresh when they coerce. `RCow` of
  an external pointer or a symbol offers only `into_mut_in_place`, and `RCow<RFunction>` is a
  compile error.
- `Option<T>` works for every argument type, including `RCow<T>`. It is `None` for `NULL` only;
  since `c()` is `NULL`, an empty vector made with `c()` is `None`.

Updating code:

1. Replace each `&mut` argument as the compiler suggests: `x: &mut RVector<f64>` becomes
   `x: RCow<RVector<f64>>` followed by `let x = x.into_mut_or_copy(pc);`, and
   `#[unsafe(in_place)] x: &mut T` becomes `x: RCow<T>` followed by
   `let x = unsafe { x.into_mut_in_place() };`.
2. Replace `to_f64_mut(pc)` on an argument with an `RCow` argument and
   `x.to_f64(pc).into_mut_or_copy(pc)`, and likewise for the other storage modes.
3. Wrap raw R API calls that can raise an R error with `r_try`, or with `pc.protect_new` for
   one that makes an object.

Fixes:

- `rprintln!`, `R::flush_console` and `R::check_user_interrupt`, which could raise an R error
  when R's protection stack was full, unwind instead.

Build:

- Roxido calls `roxido_unwind_protect`, `roxido_no_references` and `roxido_maybe_shared` from
  the template's `src/shims.c`, which `roxido update` installs. They also let a package load on
  R before 4.5, where `NO_REFERENCES` and `MAYBE_SHARED` are not functions.

Remaining limits:

- A destructor that uses R's API while Rust unwinds from an R error can fail, for example
  if R's protection stack is full, and a second panic aborts.
- Strings borrowed from some third-party ALTREP character vectors can dangle. Copy them to
  ordinary vectors first, for example with `c(x)` in R.

## 0.7.1

Breaking changes:

- `i32`, `usize` and `u8` arguments, and `RScalar`'s `i32()`, `usize()` and `u8()`, reject doubles
  that are not whole numbers, as vctrs and rlang do. `i32` used to round, so `2.7` became `3`.
  Whole doubles, such as `1` and `1e3`, still work.
- Reading `NA_character_` is an error rather than the string `"NA"`; `RVector<char>::is_na()`
  tests for it, and a `&str` argument rejects it. `RScalar::str()` returns a `Result`.
- Strings are read as UTF-8 by R's `translateCharUTF8`, so latin1 strings now work and strings
  with "bytes" encoding are an error. ASCII and UTF-8 strings are read as before, at no extra cost.
- An in-place argument raises an R error if it is `NULL` or the same object as another argument
  that is borrowed, as in `f(x, x)` in R.

New:

- An argument declared as `Option<T>`, such as `Option<&RMatrix>`, is `None` when R passes `NULL`
  and is otherwise converted as `T`.
- `set_attribute_with_pc` builds an attribute from R objects without `unsafe`.
- List `set` and `set_attribute` accept iterators as well as scalars and slices.
- Vectors, matrices and arrays have unsafe `new_uninit()`, which skips the zeroing of `new()` for
  a result that is about to be completely written, for example by worker threads.
- Errors for `&mut` and in-place arguments name the argument.
- `rprint!`, `rprintln!`, `reprint!` and `reprintln!` accept everything `format!` does.
- Compile errors for values that cannot be converted to R objects suggest a fix.

Build:

- The package's Rust code is compiled once per installation rather than twice, and Cargo decides
  what to rebuild. Changes it tracks outside `src/rust`, such as a path dependency, `RUSTFLAGS`
  or a new toolchain, are no longer missed.
- `build.rs` finds the `#[roxido]` functions by reading the crate's source files, following its
  modules and `#[cfg]` attributes. A `#[roxido]` function produced by another macro is not
  registered.
- After `roxido update`, add the build dependencies that `build.rs` needs to `src/rust/Cargo.toml`:

  ```toml
  [build-dependencies]
  syn = { version = "3", features = ["full"] }
  quote = "1.0"
  ```
- The template's release profile no longer includes debug information, which makes the compiled
  library much smaller (9.0 MB versus 3.6 MB for one package). Panics still report their file and
  line. Remove `debug = 1` from `[profile.release]` in `src/rust/Cargo.toml` by hand, since
  `roxido update` does not change that file.

Fixes:

- `&RVector<()>` and other `<()>` arguments accept any storage mode.
- A latin1 string passed as a `&str` argument no longer panics.
- `stop()` on `None` raises an R error with a message.
- The errors of `as_scalar`, `as_array` and `as_symbol` describe the expected type.
- On Windows with a GNU toolchain, the build adds to `RUSTFLAGS` rather than replacing it.

## 0.7.0

R objects and `&Pc` stay on R's main thread. Mutable arguments use copy-on-write.

Changes that compile but behave differently (the compiler will not point these out):

- A `&mut` argument is copied whenever R may still reference it, which is almost always.
  Changes reach R only if the function returns the object.
- A `&mut` argument raises an R error if it receives an object R cannot copy: `NULL`, an
  environment, a symbol, an external pointer or a primitive function. Declare an argument
  that may be `NULL` as `&RObject` and test `is_null()`. `clone` and list `get_mut` also
  reject these objects.
- `#[unsafe(in_place)]` skips the copy, so every R reference to the object sees the change. It is
  unsound if R can pass the same object in another argument, as in `f(x, x)` in R.
- List `get` and `get_mut` reject ALTREP lists, whose elements may be temporary. Copy them to
  ordinary lists first, for example with `lapply(x, identity)` in R.
- `R::null()`, `R::random_bytes()`, `RSymbol` functions and console calls panic off R's main
  thread or before initialization. Borrowed slices still work in scoped threads.
- `RFunction` calls quote arguments that R would evaluate, such as symbols, calls and bytecode.
- `new()` initializes numeric and logical data to zero.

Updating code:

1. List the `&mut` arguments of `#[roxido]` functions, for example with
   `grep -n '&mut' src/rust/src/*.rs`, and decide for each one:
   - only read: change it to `&`;
   - may be `NULL`: change it to `&RObject` and test `is_null()`;
   - modified and returned: keep `&mut`;
   - modified in place for R to see: mark it `#[unsafe(in_place)]`, only if R can never pass
     the same object in another argument.
2. If a library needs `&mut [T]` for data it only reads, prefer a read-only API. Copying with
   `.to_vec()` costs the same as a `&mut` argument.
3. Fix the compile errors:
   - `ToR<_>` is not implemented for `&mut RScalar<f64>` (or another R object) in
     `list.set(i, x)`: pass the Rust value instead of `x.to_r(pc)`, and pass vectors and
     arrays as slices. Build an element from several R objects with `list.set_with_pc(i, |pc| ...)`;
     unsafe `set_object` stores an existing R object.
   - Attribute setters take Rust values, such as `x.set_class(&["name"])` and
     `m.set_dimnames(&[Some(&rows), Some(&["a", "b"])])`. The `*_object` variants accept
     existing R objects and are unsafe. `get_dimnames` returns `&RList`.
   - Helper arguments of type `&mut Pc` become `&Pc`. Replace `Pc::default()` with
     `pc.nested(|pc| ...)`; only the innermost counter may protect objects.
   - `get_attribute` takes `pc` and protects its result.
   - `set_loop_with_pc` takes no index and returns `()`.
   - `RFunction::call0` through `call5`, `Pc::protect`, and external-pointer `decode_*`,
     `reencode`, `encode_full` and `set_address` require `unsafe`. `decode_val` needs
     `&mut self`; stored Rust values must be `'static`.
   - `&mut RExternalPtr` and `&mut RSymbol` arguments require `#[unsafe(in_place)]`.
   - Raw `SEXP` results need `unsafe { RObject::from_sexp(x, pc) }`. Generated entry points
     are unsafe, and `RObjectVariant` is sealed.
   - `__private_set_custom_panic_hook` is renamed `__private_init`; `roxido update` handles this.
4. Code that used `from_value(0, ...)` because `new()` was uninitialized can use `new()`.
5. Compare the old and new builds: run the examples and tests of both and diff the output.

Fixes:

- `RVector<char>::from_value` and `set_loop_with_pc` no longer overflow R's protection stack.
- Logical scalars are new objects, not R's shared `TRUE` and `FALSE`.
- Empty vectors give valid empty slices.

Remaining limits:

- R errors raised inside R's API still skip Rust destructors and thread joins; see "R errors"
  and "Threads" in the crate documentation.
- Strings borrowed from some third-party ALTREP character vectors can dangle. Copy them to
  ordinary vectors first, for example with `c(x)` in R.
