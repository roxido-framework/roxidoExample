use std::fs::write;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Tell Cargo to re-run this build script if the wrapper.h changes.
    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=src/rbindings.rs");

    // Get the R_HOME environment variable, or dynamically fetch it by running `Rscript`.
    let r_home = std::env::var("R_HOME").unwrap_or_else(|_| {
        // Run `Rscript` to get the R_HOME directory.
        let output = Command::new("Rscript")
            .args(["-e", "cat(R.home())"]) // Run Rscript and evaluate R.home()
            .output()
            .expect("Failed to execute Rscript. Ensure Rscript is installed and in your PATH.");
        if !output.status.success() {
            panic!(
                "Failed to determine R_HOME dynamically: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        // Convert the output to a String and trim any whitespace.
        String::from_utf8(output.stdout)
            .expect("Invalid UTF-8 output from Rscript.")
            .trim()
            .to_string()
    });

    // Compute the include path by appending "include" to R_HOME.
    let include_path = PathBuf::from(&r_home).join("include");

    // Check if the computed include path exists.
    if !include_path.exists() {
        panic!(
            "The computed include path '{}' does not exist.",
            include_path.display()
        );
    }

    // Custom code to prepend to the bindings
    let custom_code = r#"//! Automatically generated bindings for R's C API.
//! Do not edit manually.
#![allow(improper_ctypes)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(clippy::all)]

"#;

    // Generate the bindings using the bindgen crate.
    let bindings = bindgen::Builder::default()
        .header("wrapper.h") // Specify the input header file.
        .blocklist_item("FP_NAN") // Blocklist specific items.
        .blocklist_item("FP_INFINITE")
        .blocklist_item("FP_ZERO")
        .blocklist_item("FP_SUBNORMAL")
        .blocklist_item("FP_NORMAL")
        .clang_arg(format!("-I{}", include_path.display())) // Add include path.
        .generate()
        .expect("Unable to generate the bindings.");

    // Convert the generated bindings to a string.
    let bindings_string = bindings.to_string();

    // Combine custom code with the generated bindings.
    let combined_code = format!("{custom_code}{bindings_string}");

    // Write the combined code to src/bindings.rs.
    let out_path = PathBuf::from("src").join("rbindings.rs");
    write(&out_path, combined_code).expect("Could not write the bindings!");
}
