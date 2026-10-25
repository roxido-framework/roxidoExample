# Run from the repository root: Rscript dev/test-thread-safety.R
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
dir <- new_crate("thread-safety", "cdylib")
cargo(dir, c("rustc", "--lib"), c("--", link))
prefix <- if (.Platform$OS.type == "windows") "" else "lib"
suffix <- if (Sys.info()[["sysname"]] == "Darwin") ".dylib" else .Platform$dynlib.ext
library <- dyn.load(file.path(dir, "target/debug", paste0(prefix, "thread_safety", suffix)))
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
      stopifnot(is.character(error), grepl("unsafe borrowing", error, fixed = TRUE))
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
  w <- c(1, 2)
  invisible(call("bump_in_place", w))
  stopifnot(identical(w, c(2, 2)))
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
cat("Compile-fail, thread, and protection checks passed.\n")
