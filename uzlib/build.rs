use std::env;
use std::path::PathBuf;

fn main() {
    cc::Build::new()
        .files(vec!["c/adler32.c", "c/tinflate.c"])
        .includes(vec!["c"])
        .compile("uzlib");

    bindgen::Builder::default()
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .header("bindings.h")
        .clang_args(vec!["-Ic"])
        .use_core()
        .size_t_is_usize(true)
        .ctypes_prefix("crate::ctypes")
        .layout_tests(false)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
