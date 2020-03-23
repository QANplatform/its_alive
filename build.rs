use std::env;

fn main() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rustc-link-search={}", project_dir); // the "-L" flag
    #[cfg(feature = "quantum")]
    println!("cargo:rustc-link-lib=hash"); // the "-l" flag
}