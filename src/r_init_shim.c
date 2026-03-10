// Forwards R's package init to the Rust-defined registration function.

void R_init_roxidoExample_rust(void *dll);
void R_init_roxidoExample(void *dll) { R_init_roxidoExample_rust(dll); }
