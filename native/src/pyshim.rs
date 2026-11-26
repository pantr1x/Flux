// Python v termináli Fluxu: `pip install` funguje aj na Ubuntu/Debiane (PEP 668 – systémový Python je
// „externally managed“ a pip doňho inštalovať nedovolí). Terminál projektu dostane na začiatok PATH malé
// skripty pip/pip3/python/python3: ak má projekt .venv (venv, env), použijú ho; `pip install` bez neho
// najprv vytvorí .venv. Vlastné aktivované prostredie (source …/activate) má prednosť – je v PATH skôr.
use std::path::{Path, PathBuf};

const SHIM: &str = r#"#!/bin/sh
# Flux: pip/python v termináli projektu – použije .venv projektu, pri `pip install` ho vytvorí
me=$(basename "$0")
root="${FLUX_PROJECT:-$PWD}"
venv=""
for n in .venv venv env; do
  if [ -x "$root/$n/bin/python" ]; then venv="$root/$n"; break; fi
done
# skutočný príkaz systému (PATH bez priečinka Fluxu)
real() {
  PATH=$(printf '%s' "$PATH" | tr ':' '\n' | grep -vxF "$FLUX_SHIM" | paste -sd: -) command -v "$1"
}
make_venv() {
  py=$(real python3 || real python)
  if [ -z "$py" ]; then echo "Flux: Python was not found." >&2; exit 1; fi
  printf '\033[36mFlux: creating a virtual environment (.venv) for this project – packages go there, not into the system Python.\033[0m\n' >&2
  if ! "$py" -m venv "$root/.venv"; then
    rm -rf "$root/.venv"
    printf '\033[31mFlux: the virtual environment could not be created. Install it once with:\033[0m\n  sudo apt install python3-venv python3-full\n' >&2
    exit 1
  fi
  venv="$root/.venv"
}
case "$me" in
  pip|pip3)
    if [ -z "$venv" ] && [ "$1" = "install" ]; then make_venv; fi
    if [ -n "$venv" ]; then exec "$venv/bin/python" -m pip "$@"; fi
    p=$(real "$me") || { echo "Flux: $me was not found." >&2; exit 127; }
    exec "$p" "$@" ;;
  *)
    if [ -z "$venv" ] && [ "$1" = "-m" ] && [ "$2" = "pip" ] && [ "$3" = "install" ]; then make_venv; fi
    if [ -n "$venv" ]; then exec "$venv/bin/python" "$@"; fi
    p=$(real "$me") || { echo "Flux: $me was not found." >&2; exit 127; }
    exec "$p" "$@" ;;
esac
"#;

pub const NAMES: [&str; 4] = ["pip", "pip3", "python", "python3"];

// priečinok so skriptmi (userData/bin/pyshim); zapíše ich, keď chýbajú alebo sa zmenili
#[cfg(unix)]
pub fn ensure(dir: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).ok()?;
    for n in NAMES {
        let f = dir.join(n);
        if std::fs::read_to_string(&f).ok().as_deref() != Some(SHIM) {
            std::fs::write(&f, SHIM).ok()?;
        }
        let _ = std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o755));
    }
    Some(dir.to_path_buf())
}

#[cfg(not(unix))]
pub fn ensure(_dir: &Path) -> Option<PathBuf> {
    None
}

// priečinok s príkazmi existujúceho prostredia projektu (Windows: Scripts)
pub fn venv_bin(project: &Path) -> Option<PathBuf> {
    let sub = if cfg!(windows) { "Scripts" } else { "bin" };
    let exe = if cfg!(windows) { "python.exe" } else { "python" };
    [".venv", "venv", "env"].iter().map(|n| project.join(n)).find(|v| v.join(sub).join(exe).is_file()).map(|v| v.join(sub))
}

// premenné prostredia pre terminál projektu
pub fn shell_env(project: Option<&Path>, shim: Option<&Path>) -> Vec<(String, String)> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut front: Vec<String> = vec![];
    let mut env = vec![];
    if let Some(s) = shim {
        front.push(s.to_string_lossy().to_string());
        env.push(("FLUX_SHIM".to_string(), s.to_string_lossy().to_string()));
    }
    if let Some(p) = project {
        env.push(("FLUX_PROJECT".to_string(), p.to_string_lossy().to_string()));
        // Windows: hotové .venv rovno „aktivovať“ (pip tam PEP 668 nemá, stačí PATH)
        if shim.is_none() {
            if let Some(b) = venv_bin(p) {
                env.push(("VIRTUAL_ENV".to_string(), b.parent().unwrap_or(&b).to_string_lossy().to_string()));
                front.push(b.to_string_lossy().to_string());
            }
        }
    }
    if !front.is_empty() {
        let old = std::env::var("PATH").unwrap_or_default();
        env.push(("PATH".to_string(), format!("{}{sep}{old}", front.join(sep))));
    }
    env
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::Command;

    // skutočný beh skriptu: bez .venv ide pip na systémový príkaz; s .venv sa použije jeho Python
    #[test]
    fn pyshim_uses_project_venv() {
        let base = std::env::temp_dir().join(format!("flux-pyshim-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let shim = ensure(&base.join("shim")).unwrap();
        let proj = base.join("proj");
        // falošné .venv: python vypíše, že je z .venv
        std::fs::create_dir_all(proj.join(".venv/bin")).unwrap();
        let fake = proj.join(".venv/bin/python");
        std::fs::write(&fake, "#!/bin/sh\necho VENV \"$@\"\n").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
        let env = shell_env(Some(&proj), Some(&shim));
        let run = |cmd: &str| {
            let mut c = Command::new("sh");
            c.arg("-c").arg(cmd).current_dir(&proj);
            for (k, v) in &env {
                c.env(k, v);
            }
            String::from_utf8_lossy(&c.output().unwrap().stdout).trim().to_string()
        };
        assert_eq!(run("pip install requests"), "VENV -m pip install requests");
        assert_eq!(run("python3 main.py"), "VENV main.py");
        let _ = std::fs::remove_dir_all(&base);
    }
}
