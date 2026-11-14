//! flux-tools – pomocník pre preklady Flux Native (náhrada scripts/*.py a extract-strings.mjs).
//!
//!   cargo run -p flux-tools -- strings          texty t("…") z native/src → locales/en.keys.json
//!   cargo run -p flux-tools -- check [--all]    čo v jazykoch chýba a či sedia zástupné znaky {…}
//!   cargo run -p flux-tools -- add <súbor.json> doplní preklady {"sk": {"Text": "Preklad"}, …} do locales/*.json
//!   cargo run -p flux-tools -- fmt              zoradí a naformátuje locales/*.json
//!   cargo run -p flux-tools -- review           kontrola plugin pull requestu (CI, plugin-review.yml)
mod review;

use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    // koreň repozitára (crates/flux-tools/../..)
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repo")
}

fn rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "gen.rs") {
            out.push(p);
        }
    }
}

fn unescape(s: &str) -> String {
    s.replace("\\\"", "\"").replace("\\u{2026}", "…").replace("\\n", "\n").replace("\\\\", "\\")
}

// všetky texty na preklad z kódu Fluxu
pub fn strings(src_root: &Path) -> BTreeSet<String> {
    let q = r#""((?:\\.|[^"\\])*)""#;
    let t = Regex::new(&format!(r"\bt[f]?\({q}")).unwrap();
    let rows = Regex::new(&format!(r#"Row::(?:Toggle|Select|Range|Text|Color|Number|Button)\(\s*"[\w-]+",\s*{q}(?:,\s*{q})?(?:,\s*{q})?"#)).unwrap();
    let info = Regex::new(&format!(r"Row::(?:Info|Lead)\(\s*{q}")).unwrap();
    let sect = Regex::new(r#"\(\s*"([A-Z][^"]*)",\s*(?:vec!\[|pick\()"#).unwrap();
    let km = Regex::new(r#"\("[\w-]+", "(\w+)", "([^"]+)", "[^"]*"\)"#).unwrap();
    let km_fixed = Regex::new(r#"(?m)^\s*\("([^"]+)", "[^"]+"\),$"#).unwrap();
    let tabs = Regex::new(r#"\("[\w-]*", "[\w-]*", "([^"]+)", "([^"]*)"\)"#).unwrap();
    let letter = Regex::new("[A-Za-z]").unwrap();
    let mut keys = BTreeSet::new();
    let mut files = vec![];
    rs_files(src_root, &mut files);
    files.sort();
    for f in files {
        let src = std::fs::read_to_string(&f).unwrap_or_default();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        for c in t.captures_iter(&src) {
            keys.insert(unescape(&c[1]));
        }
        for c in rows.captures_iter(&src) {
            for v in c.iter().skip(1).flatten() {
                if letter.is_match(v.as_str()) {
                    keys.insert(unescape(v.as_str()));
                }
            }
        }
        for c in info.captures_iter(&src) {
            keys.insert(unescape(&c[1]));
        }
        for c in sect.captures_iter(&src) {
            keys.insert(c[1].to_string());
        }
        if name == "keymap.rs" {
            for c in km.captures_iter(&src) {
                keys.insert(c[1].to_string());
                keys.insert(c[2].to_string());
            }
            for c in km_fixed.captures_iter(&src) {
                keys.insert(c[1].to_string());
            }
            keys.insert("Plugins".into());
            keys.insert("Editor".into());
        }
        if name == "prefs.rs" {
            for c in tabs.captures_iter(&src) {
                for v in c.iter().skip(1).flatten().filter(|v| !v.as_str().is_empty()) {
                    keys.insert(v.as_str().to_string());
                }
            }
        }
    }
    // mená, ktoré sa neprekladajú
    for k in ["Python", "JavaScript", "VS Code Dark", "Consolas", "Cascadia Code", "Cascadia Mono", "Courier New"] {
        keys.remove(k);
    }
    keys.retain(|k| !k.trim().is_empty());
    keys
}

fn placeholders(s: &str) -> Vec<String> {
    let re = Regex::new(r"\{\w+\}").unwrap();
    let mut v: Vec<String> = re.find_iter(s).map(|m| m.as_str().to_string()).collect();
    v.sort();
    v
}

// texty, ktoré sa prekladať nemusia (kód, názvy)
fn skip(k: &str) -> bool {
    let three = Regex::new("[A-Za-z]{3}").unwrap();
    let names = Regex::new(r"^(if|for|def|class|main|lambda|try|while|with|Flux|English|Git|Java|Lua|PHP|Ruby|Rust|Claude .*|flux_\w+|main\.\w+|\[.*|\.css)$").unwrap();
    !three.is_match(k) || names.is_match(k)
}

// zápis ako predtým build-locales.py: zoradené kľúče, odsadenie 1, bez \u escape
fn write_json(path: &Path, map: &BTreeMap<String, String>) {
    let mut buf = Vec::new();
    let fmt = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, fmt);
    serde::Serialize::serialize(map, &mut ser).unwrap();
    std::fs::write(path, buf).unwrap();
}

fn read_map(path: &Path) -> BTreeMap<String, String> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_else(|_| "{}".into())).unwrap_or_default();
    v.as_object().map(|o| o.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect()).unwrap_or_default()
}

fn langs(root: &Path) -> Vec<String> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(root.join("locales/index.json")).unwrap()).unwrap();
    v.as_array().unwrap().iter().filter_map(|l| l["code"].as_str()).filter(|c| *c != "en").map(String::from).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = root();
    let loc = root.join("locales");
    match args.first().map(String::as_str) {
        Some("strings") => {
            let keys: Vec<String> = strings(&root.join("native/src")).into_iter().collect();
            let mut buf = Vec::new();
            let mut ser = serde_json::Serializer::with_formatter(&mut buf, serde_json::ser::PrettyFormatter::with_indent(b" "));
            serde::Serialize::serialize(&keys, &mut ser).unwrap();
            std::fs::write(loc.join("en.keys.json"), buf).unwrap();
            println!("{} textov → locales/en.keys.json", keys.len());
        }
        Some("check") => {
            let all = args.iter().any(|a| a == "--all");
            let keys: Vec<String> = serde_json::from_str(&std::fs::read_to_string(loc.join("en.keys.json")).unwrap()).unwrap();
            let mut bad = 0;
            for code in langs(&root) {
                let m = read_map(&loc.join(format!("{code}.json")));
                for (k, v) in &m {
                    if placeholders(k) != placeholders(v) {
                        println!("[{code}] zástupné znaky nesedia: {k:?} -> {v:?}");
                        bad += 1;
                    }
                }
                let missing: Vec<&String> = keys.iter().filter(|k| !m.contains_key(*k) && !skip(k)).collect();
                let shown: Vec<&&String> = if all { missing.iter().collect() } else { missing.iter().take(8).collect() };
                println!("{code}: {} preložených, chýba {} {:?}", m.len(), missing.len(), shown);
            }
            if bad > 0 {
                std::process::exit(1);
            }
        }
        Some("add") => {
            let Some(file) = args.get(1) else {
                eprintln!("použitie: flux-tools add <súbor.json>  ({{\"sk\": {{\"Text\": \"Preklad\"}}, …}})");
                std::process::exit(2);
            };
            let v: Value = serde_json::from_str(&std::fs::read_to_string(file).expect("súbor")).expect("JSON");
            for (code, tr) in v.as_object().expect("objekt jazykov") {
                let path = loc.join(format!("{code}.json"));
                if !path.exists() {
                    eprintln!("neznámy jazyk {code}");
                    continue;
                }
                let mut m = read_map(&path);
                let mut n = 0;
                for (k, t) in tr.as_object().into_iter().flatten() {
                    let t = t.as_str().unwrap_or("").to_string();
                    if placeholders(k) != placeholders(&t) {
                        println!("[{code}] zástupné znaky nesedia: {k:?} -> {t:?}");
                    }
                    m.insert(k.clone(), t);
                    n += 1;
                }
                write_json(&path, &m);
                println!("{code}: +{n}");
            }
        }
        Some("fmt") => {
            for code in langs(&root) {
                let p = loc.join(format!("{code}.json"));
                write_json(&p, &read_map(&p));
            }
            println!("ok");
        }
        Some("review") => std::process::exit(review::run()),
        _ => {
            println!("flux-tools strings | check [--all] | add <súbor.json> | fmt | review");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_strings_found() {
        let keys = strings(&root().join("native/src"));
        assert!(keys.contains("Python libraries"));
        assert!(keys.len() > 300);
        assert!(!keys.contains("Python"));
    }

    #[test]
    fn tools_placeholders() {
        assert_eq!(placeholders("Updating {n} % of {v}"), ["{n}", "{v}"]);
        assert!(skip("if"));
        assert!(!skip("Install"));
    }
}
