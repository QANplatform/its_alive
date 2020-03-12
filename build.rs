use std::env;

#[cfg(not(feature = "quantum"))]
fn main() {
    let project_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    println!("cargo:rustc-link-search={}", project_dir); // the "-L" flag
    println!("cargo:rustc-link-lib=hash"); // the "-l" flag
}