use camino::{Utf8Path, Utf8PathBuf};
use std::path::Path;
use uniffi_bindgen::bindings::SwiftBindingGenerator;
use uniffi_bindgen::library_mode;
use uniffi_bindgen::EmptyCrateConfigSupplier;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("no parent")
        .parent()
        .expect("no grandparent"); // locus/

    // Library mode: extract everything from the cdylib's embedded metadata.
    // No UDL file needed — proc macros embed the symbols at compile time.
    let dylib_path = workspace_root.join("target/release/liblocus_core.dylib");
    let lib_utf8 = Utf8Path::from_path(&dylib_path).expect("non-UTF8 dylib path");

    let out_dir_abs = workspace_root.join("generated/swift");
    std::fs::create_dir_all(&out_dir_abs).expect("mkdir out");
    let out_dir = Utf8PathBuf::from_path_buf(out_dir_abs).expect("non-UTF8 out path");

    let gen = SwiftBindingGenerator;
    let supplier = EmptyCrateConfigSupplier;

    library_mode::generate_bindings(lib_utf8, None, &gen, &supplier, None, &out_dir, false)
        .expect("failed to generate Swift bindings");

    println!("Swift bindings written to {out_dir}");
}
