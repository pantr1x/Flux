// Python knižnice: ktoré importy chýbajú, ich názov v pipe a zastarané balíčky (pip list --outdated).
use crate::i18n::tf;
use crate::lint::Diag;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};

pub struct Imp {
    pub line: usize,
    pub col: usize,
    pub len: usize,
    pub module: String, // prvá časť názvu (`os` z `os.path`)
}

// import a, b as c  /  from a.b import x  (relatívne importy sa preskočia)
pub fn imports(text: &str) -> Vec<Imp> {
    let mut v = Vec::new();
    for (i, raw) in text.split('\n').enumerate() {
        let code = raw.split('#').next().unwrap_or("");
        let trimmed = code.trim_start();
        let lead = code.len() - trimmed.len();
        let (rest, off) = if let Some(r) = trimmed.strip_prefix("import ") {
            (r, lead + 7)
        } else if let Some(r) = trimmed.strip_prefix("from ") {
            let end = r.find(" import ").unwrap_or(r.len());
            (&r[..end], lead + 5)
        } else {
            continue;
        };
        let is_from = trimmed.starts_with("from ");
        let mut pos = off;
        for part in rest.split(',') {
            let name = part.trim_start();
            let start = pos + (part.len() - name.len());
            pos += part.len() + 1;
            let name = name.split_whitespace().next().unwrap_or("");
            let first = name.split('.').next().unwrap_or("");
            if first.is_empty() || name.starts_with('.') || !first.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let col = code[..start.min(code.len())].chars().count();
            v.push(Imp { line: i, col, len: first.chars().count(), module: first.to_string() });
            if is_from {
                break;
            }
        }
    }
    v
}

const CHECK: &str = r#"import sys, json, importlib.util
sys.path.insert(0, sys.argv[1])
std = getattr(sys, 'stdlib_module_names', ())
out = []
for m in json.load(sys.stdin):
    if m in std or m == '__future__':
        continue
    try:
        if importlib.util.find_spec(m) is None:
            out.append(m)
    except Exception:
        out.append(m)
print(json.dumps(out))
"#;

fn cmd(py: &str) -> Command {
    #[allow(unused_mut)]
    let mut c = Command::new(py);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // bez okna konzoly
    }
    c
}

fn cache() -> &'static Mutex<HashMap<String, Vec<String>>> {
    static C: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

// po inštalácii / aktualizácii sa výsledky zahodia
pub fn forget() {
    cache().lock().unwrap().clear();
}

// moduly, ktoré sa nedajú nájsť (mimo štandardnej knižnice a priečinka súboru)
pub fn missing(py: &str, dir: &str, mods: &[String]) -> Vec<String> {
    if mods.is_empty() {
        return vec![];
    }
    let mut sorted = mods.to_vec();
    sorted.sort();
    sorted.dedup();
    let key = format!("{py}|{dir}|{}", sorted.join(","));
    if let Some(v) = cache().lock().unwrap().get(&key) {
        return v.clone();
    }
    let run = || -> Option<Vec<String>> {
        let mut child = cmd(py).args(["-I", "-c", CHECK, dir]).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
        child.stdin.take()?.write_all(serde_json::to_string(&sorted).ok()?.as_bytes()).ok()?;
        let out = child.wait_with_output().ok()?;
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
        Some(v.as_array()?.iter().filter_map(|x| x.as_str().map(String::from)).collect())
    };
    let res = run().unwrap_or_default();
    cache().lock().unwrap().insert(key, res.clone());
    res
}

// názov balíčka pre pip (import cv2 → opencv-python)
pub fn pip_name(module: &str) -> String {
    let n = match module {
        "cv2" => "opencv-python",
        "PIL" => "pillow",
        "yaml" => "pyyaml",
        "sklearn" => "scikit-learn",
        "skimage" => "scikit-image",
        "bs4" => "beautifulsoup4",
        "dotenv" => "python-dotenv",
        "dateutil" => "python-dateutil",
        "serial" => "pyserial",
        "OpenGL" => "PyOpenGL",
        "Crypto" => "pycryptodome",
        "jwt" => "PyJWT",
        "magic" => "python-magic",
        "docx" => "python-docx",
        "pptx" => "python-pptx",
        "fitz" => "PyMuPDF",
        "attr" => "attrs",
        "git" => "GitPython",
        "telegram" => "python-telegram-bot",
        "discord" => "discord.py",
        "win32api" | "win32con" | "win32gui" | "win32com" | "pythoncom" | "pywintypes" => "pywin32",
        "usb" => "pyusb",
        "zmq" => "pyzmq",
        "MySQLdb" => "mysqlclient",
        "psycopg2" => "psycopg2-binary",
        "gi" => "PyGObject",
        "wx" => "wxPython",
        "Xlib" => "python-xlib",
        "ruamel" => "ruamel.yaml",
        _ => module,
    };
    n.to_string()
}

// nálezy pre editor: každý chýbajúci import dostane tip „nainštalovať“; druhá hodnota = balíčky pre pip
pub fn diags(py: &str, text: &str, dir: &str) -> (Vec<Diag>, Vec<String>) {
    let imps = imports(text);
    let mods: Vec<String> = imps.iter().map(|i| i.module.clone()).collect();
    let miss = missing(py, dir, &mods);
    let mut out = Vec::new();
    let mut pkgs: Vec<String> = Vec::new();
    for i in imps.iter().filter(|i| miss.contains(&i.module)) {
        let pkg = pip_name(&i.module);
        out.push(Diag {
            line: i.line,
            col: i.col,
            len: i.len,
            err: false,
            msg: tf("Library \"{name}\" isn't installed – click to install it.", &[("name", &pkg)]),
            color: None,
            pkg: Some(pkg.clone()),
        });
        if !pkgs.contains(&pkg) {
            pkgs.push(pkg);
        }
    }
    (out, pkgs)
}

#[derive(Clone, Debug)]
pub struct Outdated {
    pub name: String,
    pub current: String,
    pub latest: String,
}

pub fn outdated(py: &str) -> Result<Vec<Outdated>, String> {
    let out = cmd(py).args(["-m", "pip", "list", "--outdated", "--format=json", "--disable-pip-version-check", "--timeout", "10", "--retries", "1"]).stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).map_err(|_| String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("pip failed").to_string())?;
    let mut list: Vec<Outdated> = v
        .as_array()
        .map(|a| a.iter().map(|x| Outdated { name: x["name"].as_str().unwrap_or("").into(), current: x["version"].as_str().unwrap_or("").into(), latest: x["latest_version"].as_str().unwrap_or("").into() }).collect())
        .unwrap_or_default();
    list.sort_by_key(|o| o.name.to_lowercase());
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pylibs_imports() {
        let v = imports("import os, numpy as np\nfrom PIL import Image\nfrom . import x\n    import requests.adapters  # c\nx = 1\n");
        let m: Vec<&str> = v.iter().map(|i| i.module.as_str()).collect();
        assert_eq!(m, ["os", "numpy", "PIL", "requests"]);
        assert_eq!((v[1].line, v[1].col, v[1].len), (0, 11, 5));
        assert_eq!((v[2].line, v[2].col), (1, 5));
        assert_eq!(v[3].col, 11);
    }

    #[test]
    fn pylibs_names_and_missing() {
        assert_eq!(pip_name("cv2"), "opencv-python");
        assert_eq!(pip_name("requests"), "requests");
        let py = if cfg!(windows) { "python" } else { "python3" };
        if Command::new(py).arg("--version").output().is_ok() {
            let m = missing(py, ".", &["os".into(), "json".into(), "definitely_not_a_module_xyz".into()]);
            assert_eq!(m, ["definitely_not_a_module_xyz"]);
        }
    }
}
