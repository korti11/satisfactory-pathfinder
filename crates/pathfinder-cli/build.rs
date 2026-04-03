fn main() {
    // Use RELEASE_VERSION (set by CI from the git tag) if available,
    // otherwise fall back to the version in Cargo.toml.
    let version = std::env::var("RELEASE_VERSION")
        .unwrap_or_else(|_| std::env::var("CARGO_PKG_VERSION").unwrap());
    println!("cargo:rustc-env=APP_VERSION={version}");
}
