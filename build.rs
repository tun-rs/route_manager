fn main() {
    // detect docs rs builder so we don't try to link to macos/freebsd libs while cross compiling
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    let docs_builder = std::env::var("DOCS_RS").is_ok();
    if docs_builder {
        println!("cargo:rustc-cfg=docsrs");
        return;
    }

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "macos"
        || target_os == "freebsd"
        || target_os == "openbsd"
        || target_os == "netbsd"
    {
        #[cfg(feature = "bindgen")]
        build_wrapper();
    }
}
#[cfg(feature = "bindgen")]
fn build_wrapper() {
    use std::env;
    use std::path::PathBuf;
    // Tell cargo to look for shared libraries in the specified directory
    //println!("cargo:rustc-link-search=/path/to/lib");

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    //println!("cargo:rustc-link-lib=bz2");

    // Tell cargo to invalidate the built crate whenever the wrapper changes
    println!("cargo:rerun-if-changed=wrapper.h");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
