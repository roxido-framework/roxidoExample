#include <stdio.h>
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

static void init_audit_classes(DllInfo *dll);

void R_init_altrep_list(DllInfo *dll) {
    init_audit_classes(dll);
    lazy_list_class = R_make_altlist_class("lazy_list", "roxido_tests", dll);
    R_set_altrep_Length_method(lazy_list_class, lazy_length);
    R_set_altlist_Elt_method(lazy_list_class, lazy_element);
}

static R_altrep_class_t bad_real_class, bad_dim_class, bad_dim_data_class;
static R_altrep_class_t bad_class_class, read_only_class, ephemeral_string_class;
static int string_counter;

static R_xlen_t bad_length(SEXP x) {
    if (R_altrep_data2(x) != R_NilValue) Rf_error("ALTREP length failed");
    return 2;
}
static R_xlen_t two_elements(SEXP x) { return 2; }
static void *bad_dim_data(SEXP x, Rboolean writable) {
    static int dimensions[2] = {2, 2};
    if (R_altrep_data2(x) != R_NilValue) Rf_error("ALTREP dimension data failed");
    return dimensions;
}
static SEXP bad_class_element(SEXP x, R_xlen_t i) {
    if (R_altrep_data2(x) != R_NilValue) Rf_error("ALTREP class failed");
    return Rf_mkChar("data.frame");
}
static void *read_only_data(SEXP x, Rboolean writable) {
    if (writable) Rf_error("read-only ALTREP received a writable request");
    return (void *) REAL_RO(R_altrep_data1(x));
}
static SEXP ephemeral_string(SEXP x, R_xlen_t i) {
    char buffer[96];
    snprintf(buffer, sizeof(buffer), "temporary-string-%d", ++string_counter);
    return Rf_mkChar(buffer);
}

SEXP make_bad_metadata(SEXP kind) {
    int code = Rf_asInteger(kind);
    if (code == 0) return R_new_altrep(bad_real_class, R_NilValue, R_BaseEnv);
    SEXP x = PROTECT(Rf_allocVector(code == 3 ? VECSXP : REALSXP, 4));
    R_altrep_class_t cls = code == 1 ? bad_dim_class :
        code == 2 ? bad_dim_data_class : bad_class_class;
    SEXP attribute = PROTECT(R_new_altrep(cls, R_NilValue, R_NilValue));
    // Install valid metadata through R's public API, then enable failures when it is read.
    Rf_setAttrib(x, code == 3 ? R_ClassSymbol : R_DimSymbol, attribute);
    R_set_altrep_data2(attribute, R_BaseEnv);
    UNPROTECT(2);
    return x;
}

SEXP make_read_only(void) {
    SEXP values = PROTECT(Rf_allocVector(REALSXP, 2));
    REAL(values)[0] = 1;
    REAL(values)[1] = 2;
    SEXP x = R_new_altrep(read_only_class, values, R_NilValue);
    UNPROTECT(1);
    return x;
}

SEXP make_ephemeral_string(void) {
    string_counter = 0;
    return R_new_altrep(ephemeral_string_class, R_NilValue, R_NilValue);
}

static void init_audit_classes(DllInfo *dll) {
    bad_real_class = R_make_altreal_class("bad_length", "roxido_tests", dll);
    R_set_altrep_Length_method(bad_real_class, bad_length);
    bad_dim_class = R_make_altinteger_class("bad_dim_length", "roxido_tests", dll);
    R_set_altrep_Length_method(bad_dim_class, bad_length);
    R_set_altvec_Dataptr_method(bad_dim_class, bad_dim_data);
    bad_dim_data_class = R_make_altinteger_class("bad_dim_data", "roxido_tests", dll);
    R_set_altrep_Length_method(bad_dim_data_class, two_elements);
    R_set_altvec_Dataptr_method(bad_dim_data_class, bad_dim_data);
    bad_class_class = R_make_altstring_class("bad_class", "roxido_tests", dll);
    R_set_altrep_Length_method(bad_class_class, lazy_length);
    R_set_altstring_Elt_method(bad_class_class, bad_class_element);
    read_only_class = R_make_altreal_class("read_only", "roxido_tests", dll);
    R_set_altrep_Length_method(read_only_class, two_elements);
    R_set_altvec_Dataptr_method(read_only_class, read_only_data);
    ephemeral_string_class = R_make_altstring_class("ephemeral_string", "roxido_tests", dll);
    R_set_altrep_Length_method(ephemeral_string_class, lazy_length);
    R_set_altstring_Elt_method(ephemeral_string_class, ephemeral_string);
}
