//! The target `cargo-cardiac` is built for, which names the driver a release has for it (`driver::download`).
fn main() {
    println!("cargo:rustc-env=CARDIAC_HOST={}", std::env::var("TARGET").unwrap());
    println!("cargo:rerun-if-changed=build.rs");
}
