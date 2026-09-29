// Na Windows vloží do programu ikonu Fluxu a popis „Flux“ (assets/flux.rc).
fn main() {
    println!("cargo:rerun-if-changed=assets/flux.rc");
    println!("cargo:rerun-if-changed=assets/flux.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let _ = embed_resource::compile("assets/flux.rc", embed_resource::NONE).manifest_optional();
    }
}
