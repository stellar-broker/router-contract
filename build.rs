fn main() {
    // Shrink the WASM shadow stack (default 1 MiB)
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.starts_with("wasm32") {
        println!("cargo:rustc-link-arg=-zstack-size=16384");
    }
}
