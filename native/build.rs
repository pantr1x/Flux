// Na Windows vloží do programu ikonu Fluxu a popis „Flux“ (assets/flux.rc).
fn main() {
    // commit zostavy (CI: GITHUB_SHA) – podľa neho Flux Native pozná, či je v ci-native novšia verzia
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rustc-env=FLUX_SHA={}", std::env::var("GITHUB_SHA").unwrap_or_else(|_| "dev".into()));
    println!("cargo:rerun-if-changed=assets/flux.rc");
    println!("cargo:rerun-if-changed=assets/flux.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("assets/flux.rc", embed_resource::NONE).manifest_optional();
    }
}
