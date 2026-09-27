//! Stamps the commit this binary was built from.

fn main() {
    benilla_buildstamp::emit();
    // With `gfx`, the gfx library is copied beside the binary (`benilla-gfx/build.rs`); the rpath
    // finds it there when the binary runs outside cargo.
    if std::env::var_os("CARGO_FEATURE_GFX").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
    {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN");
    }
}
