//! Web Fluxu v Ruste: `cargo run -p flux-site -- <priečinok>` zapíše stránku (index.html, llms.txt, robots.txt,
//! sitemap.xml, install.sh) a skopíruje obrázky z assets/. CI (.github/workflows/pages.yml) ju zverejní na gh-pages.
mod page;

use std::path::Path;

// (súbor, obsah, spustiteľný)
const FILES: [(&str, &str, bool); 5] = [("index.html", page::INDEX, false), ("llms.txt", page::LLMS, false), ("robots.txt", page::ROBOTS, false), ("sitemap.xml", page::SITEMAP, false), ("install.sh", page::INSTALL, true)];

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(to)?;
    let mut n = 0;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() {
            n += copy_dir(&p, &to.join(e.file_name()))?;
        } else {
            std::fs::copy(&p, to.join(e.file_name()))?;
            n += 1;
        }
    }
    Ok(n)
}

pub fn build(out: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(out)?;
    for (name, text, exec) in FILES {
        let p = out.join(name);
        std::fs::write(&p, text)?;
        #[cfg(unix)]
        if exec {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755))?;
        }
        let _ = exec;
    }
    // .nojekyll: GitHub Pages nemá stránku prehnať cez Jekyll
    std::fs::write(out.join(".nojekyll"), "")?;
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    Ok(FILES.len() + copy_dir(&assets, out)?)
}

fn main() {
    let out = std::env::args().nth(1).unwrap_or_else(|| "site-out".into());
    match build(Path::new(&out)) {
        Ok(n) => println!("{n} súborov → {out}"),
        Err(e) => {
            eprintln!("chyba: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_builds() {
        let dir = std::env::temp_dir().join(format!("flux-site-test-{}", std::process::id()));
        let n = build(&dir).unwrap();
        assert!(n > 10);
        let html = std::fs::read_to_string(dir.join("index.html")).unwrap();
        assert!(html.starts_with("<!DOCTYPE html>") || html.starts_with("<!doctype html>"));
        assert!(html.contains("Flux vs VS Code"));
        assert!(dir.join("images/editor.png").exists() && dir.join("icon.png").exists());
        assert!(page::INSTALL.starts_with("#!"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
