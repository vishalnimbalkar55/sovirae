fn main() {
    // The WebGPU build links @rpath/libwebgpu_dawn.dylib: look next to the
    // worker (development) and in the app bundle's Frameworks folder.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,@executable_path");
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,@executable_path/../Frameworks");
    }
}
