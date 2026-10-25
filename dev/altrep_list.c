#include <R.h>
#include <Rinternals.h>
#include <R_ext/Altrep.h>

static R_altrep_class_t lazy_list_class;

static R_xlen_t lazy_length(SEXP x) { return 1; }

// Each read creates an element that the list does not retain.
static SEXP lazy_element(SEXP x, R_xlen_t i) { return Rf_ScalarInteger(123); }

SEXP make_lazy_list(void) {
    return R_new_altrep(lazy_list_class, R_NilValue, R_NilValue);
}

void R_init_altrep_list(DllInfo *dll) {
    lazy_list_class = R_make_altlist_class("lazy_list", "roxido_tests", dll);
    R_set_altrep_Length_method(lazy_list_class, lazy_length);
    R_set_altlist_Elt_method(lazy_list_class, lazy_element);
}
