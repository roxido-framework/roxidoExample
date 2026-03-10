#if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn))
#endif
extern void Rf_error(const char *, ...);

#if defined(_WIN32)
#  if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn))
#  endif
#else
#  if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn, visibility("hidden")))
#  endif
#endif
void abort(void) {
    Rf_error("Rust called abort(), but converted to an R error.");
}

#if defined(_WIN32)
#  if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn))
#  endif
#else
#  if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn, visibility("hidden")))
#  endif
#endif
void _exit(int status) {
    (void)status;
    Rf_error("Rust called _exit(), but converted to an R error.");
}
