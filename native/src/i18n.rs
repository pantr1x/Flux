// Preklady – tie isté súbory ako Electron Flux (locales/*.json, pribalené v programe). Novšia kópia
// v userData/locales (stiahnutá Electron Fluxom) má prednosť. Text je anglický kľúč: t("Run").
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

static STRINGS: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

fn bundled(lang: &str) -> Option<&'static str> {
    Some(match lang {
        "sk" => include_str!("../../locales/sk.json"),
        "de" => include_str!("../../locales/de.json"),
        "es" => include_str!("../../locales/es.json"),
        "fr" => include_str!("../../locales/fr.json"),
        "it" => include_str!("../../locales/it.json"),
        "pl" => include_str!("../../locales/pl.json"),
        "pt" => include_str!("../../locales/pt.json"),
        "uk" => include_str!("../../locales/uk.json"),
        _ => return None,
    })
}

pub const LANGS: &str = include_str!("../../locales/index.json");

fn parse(text: &str, into: &mut HashMap<String, String>) {
    if let Ok(Value::Object(m)) = serde_json::from_str::<Value>(text) {
        let m = match m.get("strings") {
            Some(Value::Object(s)) => s.clone(),
            _ => m,
        };
        for (k, v) in m {
            if let Some(s) = v.as_str() {
                into.insert(k, s.to_string());
            }
        }
    }
}

pub fn set_language(lang: &str) {
    let mut map = HashMap::new();
    if let Some(b) = bundled(lang) {
        parse(b, &mut map);
        let newer = flux_core::settings::user_data().join("locales").join(format!("{lang}.json"));
        if let Ok(t) = std::fs::read_to_string(newer) {
            parse(&t, &mut map);
        }
    }
    *STRINGS.write().unwrap() = if map.is_empty() { None } else { Some(map) };
}

pub fn t(s: &str) -> String {
    STRINGS.read().unwrap().as_ref().and_then(|m| m.get(s).cloned()).unwrap_or_else(|| s.to_string())
}

// t so zástupnými znakmi: tf("Version {v}", &[("v", "1.2")])
#[allow(dead_code)]
pub fn tf(s: &str, args: &[(&str, &str)]) -> String {
    let mut out = t(s);
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}
