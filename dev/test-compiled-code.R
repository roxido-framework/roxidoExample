# Run after installation: Rscript dev/test-compiled-code.R /path/to/library/roxidoExample
args <- commandArgs(TRUE)
stopifnot(length(args) == 1L, dir.exists(file.path(args[[1]], "libs")))
issues <- tools:::check_compiled_code(normalizePath(args[[1]]))
if (length(issues)) {
  print(issues)
  stop("Compiled-code check failed.")
}
cat("Compiled-code check passed.\n")
