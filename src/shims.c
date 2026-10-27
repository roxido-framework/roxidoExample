// Last-resort conversion of runtime termination calls to R errors.
// This skips Rust cleanup and cannot safely recover from worker-thread failures.
// Ordinary panics should unwind to the #[roxido] boundary instead.

#if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn))
#endif
extern void Rf_error(const char *, ...);

#if defined(_WIN32)
#  define SHIM_ATTRS
#  if defined(__GNUC__) || defined(__clang__)
#    undef SHIM_ATTRS
#    define SHIM_ATTRS __attribute__((noreturn))
#  endif
#else
#  define SHIM_ATTRS
#  if defined(__GNUC__) || defined(__clang__)
#    undef SHIM_ATTRS
#    define SHIM_ATTRS __attribute__((noreturn, visibility("hidden")))
#  endif
#endif

#if defined(__GNUC__) || defined(__clang__)
__attribute__((noreturn))
#endif
static void exit_shim(const char *fn, int status) {
    Rf_error("Intercepted %s(%d) from Rust and converted to an R error.", fn, status);
}

SHIM_ATTRS void abort(void)       { exit_shim("abort", 0); }
SHIM_ATTRS void _exit(int status) { exit_shim("_exit", status); }
SHIM_ATTRS void exit(int status)  { exit_shim("exit", status); }
SHIM_ATTRS void _Exit(int status) { exit_shim("_Exit", status); }

#include <Rinternals.h>
// Portable across R versions: a macro before R 4.5, a function from R 4.5.
int roxido_no_references(SEXP x) { return NO_REFERENCES(x); }
int roxido_maybe_shared(SEXP x) { return MAYBE_SHARED(x); }

#include <setjmp.h>
// R_UnwindProtect restores R's state, then calls cleanup; the cleanup jumps back here so Rust
// can unwind its own frames before R's jump is resumed with R_ContinueUnwind.
static void roxido_cleanup(void *jb, Rboolean jump) {
    if (jump) longjmp(*(jmp_buf *) jb, 1);
}
SEXP roxido_unwind_protect(SEXP (*fun)(void *), void *data, SEXP token, int *jumped) {
    jmp_buf jb;
    if (setjmp(jb)) {
        *jumped = 1;
        return R_NilValue;
    }
    return R_UnwindProtect(fun, data, roxido_cleanup, &jb, token);
}
