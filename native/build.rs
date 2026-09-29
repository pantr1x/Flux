// Na Windows vloží do programu ikonu Fluxu a popis „Flux“ (assets/flux.rc).
fn main() {
    // commit zostavy (CI: GITHUB_SHA) – podľa neho Flux Native pozná, či je v ci-native novšia verzia
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rustc-env=FLUX_SHA={}", std::env::var("GITHUB_SHA").unwrap_or_else(|_| "dev".into()));
    println!("cargo:rerun-if-changed=assets/flux.rc");
    println!("cargo:rerun-if-changed=assets/flux.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // hlavné vlákno s 8 MB zásobníkom ako na Linuxe (Windows má predvolene len 1 MB)
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
            println!("cargo:rustc-link-arg-bins=/STACK:8388608");
        } else {
            println!("cargo:rustc-link-arg-bins=-Wl,--stack,8388608");
        }
        let _ = embed_resource::compile("assets/flux.rc", embed_resource::NONE).manifest_optional();
    }
}
