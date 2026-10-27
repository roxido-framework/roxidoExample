# Run from the repository root: Rscript dev/test-thread-safety.R
# Set ROXIDO_TEST_RELEASE=true to test an optimized build.
root <- normalizePath(".", winslash = "/")
stopifnot(file.exists(file.path(root, "src/rust/roxido/Cargo.toml")))
work <- tempfile("roxido-thread-safety-")

# Make a crate from dev/<name>.rs that depends on roxido, and return its directory.
new_crate <- function(name, type) {
  dir <- file.path(work, name)
  dir.create(file.path(dir, "src"), recursive = TRUE)
  writeLines(c(
    '[package]', sprintf('name = "%s"', name), 'version = "0.0.0"',
    'edition = "2024"', '[lib]', sprintf('crate-type = ["%s"]', type),
    '[dependencies]', sprintf('roxido = { path = %s }',
      encodeString(file.path(root, "src/rust/roxido"), quote = '"'))
  ), file.path(dir, "Cargo.toml"))
  stopifnot(file.copy(file.path(root, "dev", paste0(name, ".rs")), file.path(dir, "src/lib.rs")))
  dir
}
cargo <- function(dir, args, extra = NULL) {
  log <- file.path(dir, "cargo.log")
  status <- system2("cargo", shQuote(c(args, "--manifest-path", file.path(dir, "Cargo.toml"),
    extra)), stdout = log, stderr = log)
  if (status != 0L) {
    writeLines(readLines(log))
    stop("cargo ", args[1], " failed")
  }
}

cargo(new_crate("compile-fail", "rlib"), c("test", "--doc"))
link <- if (Sys.info()[["sysname"]] == "Darwin") {
  c("-C", "link-arg=-Wl,-undefined,dynamic_lookup")
} else if (.Platform$OS.type == "windows") {
  c("-L", paste0("native=", R.home("bin")), "-l", "R")
}
# Roxido calls C shims that a package compiles from src/shims.c, so link them in the same way.
stopifnot(file.copy(file.path(root, "src/shims.c"), work))
local({
  old <- setwd(work)
  on.exit(setwd(old))
  log <- "shims.log"
  status <- system2(file.path(R.home("bin"), "R"), c("CMD", "COMPILE", "shims.c"),
    stdout = log, stderr = log)
  if (status != 0L) {
    writeLines(readLines(log))
    stop("shims build failed")
  }
})
link <- c(link, "-C", paste0("link-arg=", file.path(work, "shims.o")))
dir <- new_crate("thread-safety", "cdylib")
profile <- if (identical(Sys.getenv("ROXIDO_TEST_RELEASE"), "true")) "release" else "debug"
cargo(dir, c("rustc", "--lib", if (profile == "release") "--release"), c("--", link))
prefix <- if (.Platform$OS.type == "windows") "" else "lib"
suffix <- if (Sys.info()[["sysname"]] == "Darwin") ".dylib" else .Platform$dynlib.ext
library <- dyn.load(file.path(dir, "target", profile, paste0(prefix, "thread_safety", suffix)))
call <- function(name, ...) .Call(getNativeSymbolInfo(name, library), ...)

# Build the ALTREP fixture with R's compiler and headers.
stopifnot(file.copy(file.path(root, "dev/altrep_list.c"), work))
local({
  old <- setwd(work)
  on.exit(setwd(old))
  log <- "altrep_list.log"
  status <- system2(file.path(R.home("bin"), "R"), c("CMD", "SHLIB", "altrep_list.c"),
    stdout = log, stderr = log)
  if (status != 0L) {
    writeLines(readLines(log))
    stop("ALTREP fixture build failed")
  }
})
altrep <- dyn.load(file.path(work, paste0("altrep_list", .Platform$dynlib.ext)))
lazy_list <- .Call(getNativeSymbolInfo("make_lazy_list", altrep))

stopifnot(isFALSE(call("before_init")))
invisible(call("__private_init"))
stopifnot(identical(call("guards"), 13L), identical(call("parallel_slices"), 200))
stopifnot(isTRUE(call("nul_unwinds_workers")))
error <- tryCatch(call("embedded_nul"), error = conditionMessage)
stopifnot(identical(error, "R strings cannot contain NUL bytes."))
error <- tryCatch(call("propagated_panic"), error = conditionMessage)
stopifnot(grepl("main thread", error, fixed = TRUE))
stopifnot(identical(call("strings", 100000L), rep("a", 100000L)))
for (name in c("fill_list", "set_values")) for (n in c(0L, 100000L)) {
  list <- call(name, n)
  stopifnot(length(list) == n, all(unlist(list) == seq_len(n)))
}
# Strings are read as UTF-8: ASCII and UTF-8 as they are, others translated, bytes rejected.
utf8 <- "caf\u00e9"
for (x in list("abc", utf8, iconv(utf8, "UTF-8", "latin1"))) {
  y <- call("echo_str", x)
  stopifnot(identical(y, enc2utf8(x)), Encoding(y) %in% c("unknown", "UTF-8"))
}
error <- tryCatch(call("echo_str", `Encoding<-`(utf8, "bytes")), error = conditionMessage)
stopifnot(grepl("bytes", error, fixed = TRUE))
stopifnot(identical(call("echo_str", `Encoding<-`("abc", "bytes")), "abc"))
error <- tryCatch(call("nul_message"), error = conditionMessage)
stopifnot(identical(error, "before"))

gctorture(TRUE)
tryCatch({
  for (x in list(quote(f(a = 1, b = 2)), pairlist(a = 1, b = 2),
    data.frame(a = 1:10))) {
    which <- if (is.data.frame(x)) "row.names" else "names"
    stopifnot(identical(call("attribute_after_allocations", x, as.name(which)), attr(x, which)))
  }
  stopifnot(identical(lazy_list[[1]], 123L))
  for (name in c("list_element", "list_element_mut")) {
    error <- tryCatch(call(name, lazy_list), error = conditionMessage)
    stopifnot(identical(error,
      "Cannot borrow ALTREP list elements; copy to an ordinary list first."))
    stopifnot(identical(call(name, lapply(lazy_list, identity)), 123L))
  }
  stopifnot(isTRUE(call("logical_scalars")), isTRUE(call("initialized_constructors")),
    isTRUE(call("incorrect_iterator_lengths")))
  pointer <- call("make_pointer")
  for (value in list(pointer, quote(a), new.env(), base::sum, NULL)) {
    for (name in c("clone_object", "borrow_mutable_object", "borrow_mutable_element")) {
      arg <- if (name == "borrow_mutable_element") list(value) else value
      error <- tryCatch(call(name, arg), error = conditionMessage)
      expected <- if (name == "borrow_mutable_object") "R cannot copy" else "unsafe borrowing"
      stopifnot(is.character(error), grepl(expected, error, fixed = TRUE))
    }
  }
  invisible(call("pointer_in_place", pointer))
  stopifnot(identical(call("pointer_value", pointer), 43L))
  stopifnot(identical(call("nested_counters"), list(2, 4L, 5L)))
  stopifnot(isTRUE(call("external_pointers")))
  stopifnot(identical(call("set_attributes"),
    structure(c(a = 0, b = 0), class = "foo", note = "hi")))
  stopifnot(identical(call("matrix_dimnames"),
    matrix(0, 2, 3, dimnames = list(NULL, c("x", "y", "z")))))
  stopifnot(identical(call("array_dimnames"),
    array(0, c(1, 2, 1), dimnames = list("a", NULL, "d"))))
  z <- c(1, 2)
  stopifnot(identical(call("bump", z, z), c(2, 1)), identical(z, c(1, 2)))
  stopifnot(identical(call("transform", z), c(2, 4)), identical(z, c(1, 2)))
  v <- c(1, 2)
  l <- list(v)
  stopifnot(identical(call("bump_first", l), list(c(2, 2))), identical(v, c(1, 2)),
    identical(l, list(c(1, 2))))
  # Call directly, since passing through '...' adds a reference that would mask a missing copy.
  transform <- getNativeSymbolInfo("transform", library)
  stopifnot(identical(.Call(transform, z), c(2, 4)), identical(z, c(1, 2)))
  f <- function(a) {
    .Call(transform, a)
    a
  }
  stopifnot(identical(f(c(1, 2)), c(1, 2)))
  bump_first <- getNativeSymbolInfo("bump_first", library)
  l <- list(c(1, 2))
  stopifnot(identical(.Call(bump_first, l), list(c(2, 2))), identical(l, list(c(1, 2))))
  # An R error, or a warning turned into one, inside R's API unwinds Rust frames, and their
  # destructors can still use R's API, before R's jump resumes.
  r_error <- function(case, x = NULL) .Call(getNativeSymbolInfo("r_error", library), case, x)
  invisible(call("guard_drops"))
  warn_cases <- list(to_i32 = "a", to_f64 = "a", to_u8 = 300L, f64 = "a")
  for (case in names(warn_cases)) {
    x <- warn_cases[[case]]
    stopifnot(identical(tryCatch(r_error(case, x), warning = function(w) "caught"), "caught"))
    old <- options(warn = 2)
    error <- tryCatch(r_error(case, x), error = conditionMessage)
    options(old)
    stopifnot(grepl("converted from warning", error, fixed = TRUE))
    stopifnot(identical(call("guard_drops"), 2L))
  }
  stopifnot(identical(tryCatch(r_error("f64", as.raw(1)), error = function(e) "caught"), "caught"))
  for (case in c("symbol", "allocate", "protect", "dim", "class")) {
    stopifnot(identical(tryCatch(r_error(case), error = function(e) "caught"), "caught"))
  }
  stopifnot(identical(call("guard_drops"), 6L))
  old_seed <- if (exists(".Random.seed", globalenv())) get(".Random.seed", globalenv())
  assign(".Random.seed", "invalid", envir = globalenv())
  old <- options(warn = 2)
  error <- tryCatch(r_error("random"), error = conditionMessage)
  options(old)
  if (is.null(old_seed)) rm(".Random.seed", envir = globalenv()) else
    assign(".Random.seed", old_seed, envir = globalenv())
  stopifnot(grepl(".Random.seed", error, fixed = TRUE), identical(call("guard_drops"), 1L))
  invisible(capture.output(stopifnot(identical(call("swallowed_r_error"), 42L)),
    type = "message"))
  stopifnot(identical(call("guard_drops"), 1L))

  # RCow arguments are strict, fresh only when nothing else references them, and stay fresh
  # through a coercion that makes a new object. ALTREP sequences are always shared.
  cow_fresh <- getNativeSymbolInfo("cow_fresh", library)
  y <- c(1, 2)
  stopifnot(isTRUE(.Call(cow_fresh, y * 1)), isFALSE(.Call(cow_fresh, y)))
  for (x in list(1:2, c(TRUE, FALSE), "a")) {
    error <- tryCatch(.Call(cow_fresh, x), error = conditionMessage)
    stopifnot(identical(error, "'x' is expected to have storage mode double."))
  }
  cow_scale <- getNativeSymbolInfo("cow_scale", library)
  stopifnot(identical(.Call(cow_scale, y), c(2, 4)), identical(y, c(1, 2)))
  stopifnot(identical(.Call(cow_scale, y + 0), c(2, 4)))
  cow_coerce <- getNativeSymbolInfo("cow_coerce", library)
  yi <- c(1L, 2L)
  ys <- 1:2
  stopifnot(identical(.Call(cow_coerce, yi), structure(c(2, 2), fresh = TRUE)),
    identical(.Call(cow_coerce, ys), structure(c(2, 2), fresh = FALSE)),
    identical(.Call(cow_coerce, y), structure(c(2, 2), fresh = FALSE)),
    identical(.Call(cow_coerce, y * 1), structure(c(2, 2), fresh = TRUE)),
    identical(yi, c(1L, 2L)), identical(ys, 1:2), identical(y, c(1, 2)))
  stopifnot(identical(call("cow_aliased"), c(FALSE, FALSE, TRUE, TRUE)))
  w <- c(1, 2)
  invisible(call("bump_in_place", w, NULL))
  stopifnot(identical(w, c(2, 2)))
  error <- tryCatch(call("bump_in_place", w, w), error = conditionMessage)
  stopifnot(identical(error,
    "'x' cannot be modified in place because it is passed in another argument too."))
  error <- tryCatch(call("object_in_place", NULL), error = conditionMessage)
  stopifnot(identical(error, "'x' cannot be modified in place because it is NULL."))
  stopifnot(identical(call("optional_cow", NULL), -1), identical(call("optional_cow", w), 3),
    identical(w, c(2, 2)))
  error <- tryCatch(call("optional_cow", 1L), error = conditionMessage)
  stopifnot(identical(error, "'x' is expected to have storage mode double."))
  callbacks <- list(function() call("callback_error"), function() call("embedded_nul"),
    function() call("raw_r_error"), function() stop("R error"))
  for (f in callbacks) {
    invisible(capture.output(
      stopifnot(isTRUE(call("recover_after_callback", f, TRUE))), type = "message"))
  }
  # Symbols, calls and bytecode reach R functions as data, for builtins and closures alike.
  code <- compiler::compile(quote(1 + 1))
  for (f in list(base::list, function(x, y) base::list(x, y))) {
    stopifnot(identical(call("call_with_language", f, quote(a), quote(g(1, b))),
      list(quote(a), quote(g(1, b)))))
    stopifnot(identical(call("call_with_language", f, quote(a), code), list(quote(a), code)))
  }
  f <- function() tryCatch(call("raw_r_error"), error = function(e) NULL)
  stopifnot(isTRUE(call("recover_after_callback", f, FALSE)))
}, finally = gctorture(FALSE))
# Regressions for the RCow audit. Keep these under GC torture, including token pool growth.
gctorture(TRUE)
tryCatch({
  x <- c(1, 2)
  stopifnot(identical(call("custom_read", x), 3),
    identical(call("custom_write", x), c(9, 2)), identical(x, c(1, 2)))
  custom_write <- getNativeSymbolInfo("custom_write", library)
  stopifnot(identical(.Call(custom_write, x * 1), c(9, 2)))
  stopifnot(isTRUE(call("uninitialized_outputs")))
  stopifnot(identical(call("list_copy_policy", list(x)), list(c(9, 2))), identical(x, c(1, 2)))

  invisible(call("guard_drops"))
  for (depth in c(1L, 8L, 32L)) {
    for (i in 1:2) {
      result <- tryCatch(call("nested_warning", "bad", depth), warning = function(w) "caught")
      stopifnot(identical(result, "caught"))
    }
    stopifnot(identical(call("guard_drops"), 2L))
  }
  old <- options(warn = 2)
  tryCatch({
    error <- tryCatch(call("nested_warning", "bad", 8L), error = conditionMessage)
    stopifnot(grepl("converted from warning", error, fixed = TRUE))
    invisible(capture.output(stopifnot(identical(call("nested_recovery", "bad"), 42L)),
      type = "message"))
  }, finally = options(old))
  stopifnot(identical(call("guard_drops"), 2L))
  error <- tryCatch(call("raw_boundary_cleanup"), error = conditionMessage)
  stopifnot(identical(error, "raw boundary error"), identical(call("guard_drops"), 1L))
  stopifnot(identical(call("rust_panic_in_boundary"), 42L), identical(call("guard_drops"), 1L))

  make_bad <- getNativeSymbolInfo("make_bad_metadata", altrep)
  cases <- list(c("len", "empty", "scalar", "slice", "get"),
    c("matrix", "array", "names"), c("nrow", "ncol"), "data_frame")
  for (code in 0:3) for (case in cases[[code + 1L]]) {
    x <- .Call(make_bad, code)
    error <- tryCatch(call("metadata_error", case, x), error = conditionMessage)
    stopifnot(grepl("ALTREP", error, fixed = TRUE),
      identical(call("guard_drops"), 1L), isTRUE(call("worker_joined")))
  }
  x <- .Call(getNativeSymbolInfo("make_read_only", altrep))
  stopifnot(identical(call("read_only_slice", x), 3))

  make_string <- getNativeSymbolInfo("make_ephemeral_string", altrep)
  x <- .Call(make_string)
  error <- tryCatch(call("borrowed_altrep_string", x), error = conditionMessage)
  stopifnot(grepl("get_with_pc", error, fixed = TRUE))
  stopifnot(identical(call("protected_altrep_string", x), "temporary-string-1"))
  x <- .Call(make_string)
  stopifnot(identical(call("echo_str", x), "temporary-string-1"))
}, finally = gctorture(FALSE))
# Coercion must allocate the double result without first copying the shared integer input.
if (isTRUE(capabilities("profmem"))) {
  x <- rep(1L, 1000000L)
  log <- file.path(work, "coercion-allocations.log")
  coerce <- getNativeSymbolInfo("cow_coerce", library)
  Rprofmem(log, threshold = 1000000L)
  y <- tryCatch(.Call(coerce, x), finally = Rprofmem(NULL))
  allocations <- suppressWarnings(as.numeric(sub(" .*", "", readLines(log))))
  allocations <- allocations[!is.na(allocations)]
  stopifnot(length(allocations) == 1L, allocations[[1]] >= 8000000,
    identical(x, rep(1L, 1000000L)), identical(y[[1]], 2), isTRUE(attr(y, "fresh")))
}
cat("Compile-fail, thread, and protection checks passed.\n")
