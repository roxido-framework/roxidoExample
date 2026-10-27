//! The paper
//! [*Writing R Extensions in Rust*](https://raw.githubusercontent.com/dbdahl/cargo-framework/main/cargo/inst/doc/Writing_R_Extensions_in_Rust.pdf)
//! complements
//! [*Writing R Extensions*](https://cran.r-project.org/doc/manuals/R-exts.html)
//! (the official guide for writing R extensions) for those interested in developing
//! [R](https://www.r-project.org/) packages using
//! [Rust](https://www.rust-lang.org/). It highlights idiosyncrasies of
//! [R](https://www.r-project.org/) and [Rust](https://www.rust-lang.org/) that must
//! be addressed by any integration and describes how to develop
//! [Rust](https://www.rust-lang.org/)-based packages which comply with the [CRAN
//! Repository Policy](https://cran.r-project.org/web/packages/policies.html).  The
//! [paper]( https://raw.githubusercontent.com/dbdahl/cargo-framework/main/cargo/inst/doc/Writing_R_Extensions_in_Rust.pdf)
//! introduces the cargo framework, a
//! transparent [Rust](https://www.rust-lang.org/)-based API which wraps
//! commonly-used parts of [R](https://www.r-project.org/)'s API with minimal
//! overhead and allows a programmer to easily add additional wrappers.
//!
//! This crate provides the `roxido` procedural macro to support the
//! [roxido](https://crates.io/crates/roxido) crate

use proc_macro::TokenStream;
use quote::quote;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::{Token, parse_quote};

/// Export a function to R, converting each argument with `FromRArg` and the result with `to_r`.
/// Declare an argument as `RCow<T>` to modify it.
#[proc_macro_attribute]
pub fn roxido(attr: TokenStream, item: TokenStream) -> TokenStream {
    let options: Vec<_> = syn::punctuated::Punctuated::<NestedMeta, Token![,]>::parse_terminated
        .parse(attr)
        .map(|punctuated| punctuated.into_iter().collect())
        .unwrap();
    match syn::parse_macro_input!(item as syn::Item) {
        syn::Item::Fn(item_fn) => roxido_fn(options, item_fn),
        _ => panic!("The 'roxido' attribute can only be applied to a function."),
    }
}

struct NestedMeta(syn::Meta);

impl syn::parse::Parse for NestedMeta {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(syn::Ident::peek_any) {
            input.parse().map(NestedMeta)
        } else {
            Err(input.error("Parse error."))
        }
    }
}

macro_rules! TYPE_MESSAGE {
    () => { "ach argument to a 'roxido' function must be a named argument whose type implements 'FromRArg', such as &RVector<f64>, RCow<RVector<f64>>, f64, &str, &[f64], SEXP, or Option<T>." };
}

fn roxido_fn(options: Vec<NestedMeta>, item_fn: syn::ItemFn) -> TokenStream {
    let r_function_directory = match std::env::var("ROXIDO_R_FUNC_DIR") {
        Ok(x) if !x.is_empty() => {
            let path = Path::new(&x).to_owned();
            if path.exists() && path.is_dir() {
                Some(path)
            } else {
                None
            }
        }
        _ => None,
    };
    let mut invisible = false;
    for meta in options {
        let meta = meta.0;
        let meta_string = quote!(#meta).to_string();
        match meta {
            syn::Meta::NameValue(x) => {
                let name = x.path;
                let value = x.value;
                let name_string = quote!(#name).to_string();
                let value_string = quote!(#value).to_string();
                match name_string.as_str() {
                    "invisible" => match value_string.as_str() {
                        "true" => invisible = true,
                        "false" => invisible = false,
                        _ => {
                            panic!("Unsupported value '{value_string}' for {name_string}.")
                        }
                    },
                    // Read by build.rs, which names and registers the function.
                    "module" => {}
                    _ => panic!("Unsupported token '{name_string}'."),
                }
            }
            _ => panic!("Unsupported token '{meta_string}'."),
        }
    }
    let name = item_fn.sig.ident;
    let vis = item_fn.vis;
    let args = item_fn.sig.inputs;
    let body = item_fn.block;
    let output = item_fn.sig.output;
    // Check that visibility is okay.
    let vis_as_string = quote!(#vis).to_string();
    if !vis_as_string.is_empty() {
        panic!(
            "A function with the 'roxido' attribute may not have a visibility modifier, but found '{vis_as_string}'."
        );
    }
    // Generate code to convert each argument from its SEXP.
    let mut arg_names = Vec::with_capacity(args.len());
    let mut all_arg_idents: Vec<Box<syn::Pat>> = Vec::new();
    let mut generated_statements: Vec<syn::Stmt> = Vec::new();
    let mut new_args = args.clone();
    new_args.clear();
    for arg in &args {
        match arg {
            syn::FnArg::Typed(pat_type) => {
                let name = &pat_type.pat;
                let name_as_string = quote!(#name).to_string();
                if let Some(name_as_string) = name_as_string.strip_prefix("mut ") {
                    panic!(
                        "'{name_as_string}' is marked as mutable, but arguments to 'roxido' functions cannot themselves be mutable."
                    );
                }
                for attr in &pat_type.attrs {
                    let attr: String = quote!(#attr).to_string().split_whitespace().collect();
                    if matches!(attr.as_str(), "#[unsafe(in_place)]" | "#[in_place]") {
                        panic!(
                            "'{name_as_string}' is marked '{attr}', but an argument to modify in place is now declared as 'RCow<T>' and modified through 'unsafe {{ {name_as_string}.into_mut_in_place() }}'."
                        );
                    }
                }
                let mut y = pat_type.clone();
                y.ty = Box::new(syn::parse_str::<syn::Type>("SEXP").unwrap());
                new_args.push(syn::FnArg::Typed(y));
                let ty = &pat_type.ty;
                let option_inner = option_inner_type(ty);
                let inner = option_inner.as_ref().unwrap_or(ty);
                if let syn::Type::Reference(reference) = inner.as_ref() {
                    if reference.mutability.is_some() {
                        let n = &name_as_string;
                        let (declared, then) = match reference.elem.as_ref() {
                            syn::Type::Slice(slice) => (
                                format!("RCow<RVector<{}>>", type_string(&slice.elem)),
                                ", and then call 'slice_mut()'",
                            ),
                            elem => (format!("RCow<{}>", type_string(elem)), ""),
                        };
                        panic!(
                            "'{n}' is '&mut {}', but a 'roxido' function cannot take a '&mut' argument. Declare '{n}: {declared}' and get write access with '{n}.into_mut_or_copy(pc)', which copies only if R may still reference the object, '{n}.try_into_mut()', which never copies, or 'unsafe {{ {n}.into_mut_in_place() }}', which modifies R's object as '#[unsafe(in_place)]' did{then}.",
                            type_string(&reference.elem)
                        );
                    }
                    if reference.lifetime.is_some() {
                        panic!(
                            "'{name_as_string}' has a lifetime, which is not supported for a 'roxido' function."
                        );
                    }
                }
                if option_inner.is_some() && type_string(inner) == "SEXP" {
                    panic!(
                        "'{name_as_string}' is 'Option<SEXP>', but a 'SEXP' argument can already be NULL."
                    );
                }
                generated_statements.push(parse_quote! {
                    let #name: #ty = unsafe {
                        __private_from_arg(#name, stringify!(#name), &__roxido_all_args, pc)
                    };
                });
                arg_names.push(name_as_string);
                all_arg_idents.push(name.clone());
            }
            _ => panic!(concat!("E", TYPE_MESSAGE!())),
        }
    }
    // Check that there is no explicit return type.
    match &output {
        syn::ReturnType::Default => {}
        _ => {
            panic!(
                "A 'roxido' function should not have an explicit return type, as its body is actually a closure which returns any value whose type provides a 'to_r' method."
            );
        }
    }
    let func_name = quote!(#name).to_string();
    if let Some(mut path) = r_function_directory {
        path.push(&func_name);
        let mut file = File::create(&path).unwrap_or_else(|err| {
            panic!(
                "Could not open file '{}' for writing: {err}",
                path.display()
            );
        });
        let args = arg_names.join(", ");
        let comma = if args.is_empty() { "" } else { ", " };
        let (invisible_opening, invisible_closing) = match invisible {
            true => ("invisible(", ")"),
            false => ("", ""),
        };
        let write_result = write!(
            file,
            "{func_name} <- function({args}) {invisible_opening}.Call(.{func_name}{comma}{args}){invisible_closing}"
        );
        if let Err(err) = write_result {
            panic!("Could not write to the file '{}': {err}", path.display());
        }
    }
    let n_args = all_arg_idents.len();
    // Write the function itself, wrapping the body in 'catch_unwind' so that a panic runs Rust
    // destructors before Rf_error long-jumps into R.
    TokenStream::from(quote! {
        #[allow(clippy::useless_transmute)]
        /// # Safety
        /// Call from R's main thread after package initialization, with valid R arguments.
        #[deny(unsafe_op_in_unsafe_fn)]
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn #name(#new_args) -> SEXP {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let __roxido_pc = unsafe { Pc::__private_new() };
                let pc = &__roxido_pc;
                #[allow(unused_variables)]
                let __roxido_all_args: [SEXP; #n_args] = [#( #all_arg_idents ),*];
                #( #generated_statements )*
                let mut f = || { #body };
                f().to_r(pc).sexp()
            }));
            match result {
                Ok(obj) => obj,
                Err(payload) => unsafe { __private_raise(payload, stringify!(#name)) },
            }
        }
    })
}

// A type as written, such as 'RVector<f64>', for an error message.
fn type_string(ty: &syn::Type) -> String {
    quote!(#ty).to_string().replace(' ', "")
}

// The type 'T' of an argument declared as 'Option<T>'.
fn option_inner_type(ty: &syn::Type) -> Option<Box<syn::Type>> {
    let syn::Type::Path(path) = ty else {
        return None;
    };
    if path.qself.is_some() {
        return None;
    }
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    match arguments.args.iter().collect::<Vec<_>>().as_slice() {
        [syn::GenericArgument::Type(inner)] => Some(Box::new(inner.clone())),
        _ => None,
    }
}
