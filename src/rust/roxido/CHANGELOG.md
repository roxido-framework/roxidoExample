# Changelog

## 0.7.0

R objects and `&Pc` stay on R's main thread. Mutable arguments use copy-on-write.

Behavior changes:

- `&mut` arguments are usually copied. Return the modified object, or use
  `#[unsafe(in_place)]` when exclusive access is guaranteed. Use `&` for read-only inputs.
- `R::null()`, `R::random_bytes()`, `RSymbol` functions and console calls panic off R's main
  thread or before initialization. Borrowed slices still work in scoped threads.
- `new()` initializes numeric and logical data to zero.
- `&mut` arguments, `clone`, and list `get_mut` reject objects R cannot copy, including
  external pointers, symbols, environments and `NULL`.
- List `get` and `get_mut` reject ALTREP lists, whose elements may be temporary. Copy
  them to ordinary lists first, for example with `lapply(x, identity)` in R.
- `RFunction` calls quote arguments that R would evaluate, such as symbols, calls and bytecode.

Updating code:

- Change helper arguments from `&mut Pc` to `&Pc`. Replace `Pc::default()` with
  `pc.nested(|pc| ...)`; only the innermost counter may protect objects.
- Replace `list.set(i, x.to_r(pc))` with `list.set(i, x)` for Rust values; pass vectors and
  arrays as slices. Use `set_with_pc` to build an element, or unsafe `set_object` to store
  an existing R object.
- Attribute setters take Rust values; `*_object` variants accept existing R objects and
  are unsafe. `get_dimnames` returns `&RList`.
- `get_attribute` takes `pc` and protects its result.
- `set_loop_with_pc` takes no index and returns `()`.
- `RFunction::call0` through `call5`, `Pc::protect`, and external-pointer `decode_*`,
  `reencode`, `encode_full` and `set_address` now require `unsafe`. `decode_val` needs
  `&mut self`; stored Rust values must be `'static`.
- `&mut RExternalPtr` and `&mut RSymbol` arguments require `#[unsafe(in_place)]`.
- Raw `SEXP` results need `unsafe { RObject::from_sexp(x, pc) }`. Generated entry points
  are unsafe, and `RObjectVariant` is sealed.
- `__private_set_custom_panic_hook` is renamed `__private_init`.

Fixes:

- `RVector<char>::from_value` and `set_loop_with_pc` no longer overflow R's protection stack.
- Logical scalars are new objects, not R's shared `TRUE` and `FALSE`.
- Empty vectors give valid empty slices.

Remaining limits:

- R errors raised inside R's API still skip Rust destructors and thread joins; see "R errors"
  and "Threads" in the crate documentation.
- Strings borrowed from some third-party ALTREP character vectors can dangle. Copy them to
  ordinary vectors first, for example with `c(x)` in R.
