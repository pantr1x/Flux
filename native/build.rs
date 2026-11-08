// Na Windows vloží do programu ikonu Fluxu a popis „Flux“ (assets/flux.rc).
fn main() {
    // commit zostavy (CI: GITHUB_SHA) – podľa neho Flux Native pozná, či je v ci-native novšia verzia
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rustc-env=FLUX_SHA={}", std::env::var("GITHUB_SHA").unwrap_or_else(|_| "dev".into()));
    // témy z priečinka themes/ (súbory .conf) – nová téma = nový súbor, nič iné sa nemení (src/themes.rs)
    let dir = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../themes");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "conf")).collect()).unwrap_or_default();
    files.sort();
    let mut out = String::from("pub const BUNDLED: &[(&str, &str)] = &[\n");
    for f in &files {
        println!("cargo:rerun-if-changed={}", f.display());
        let id = f.file_stem().unwrap().to_string_lossy().to_string();
        out.push_str(&format!("    ({:?}, include_str!({:?})),\n", id, f.to_string_lossy()));
    }
    out.push_str("];\n");
    std::fs::write(std::path::Path::new(&std::env::var("OUT_DIR").unwrap()).join("themes_bundled.rs"), out).unwrap();
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
