// Flux v Ruste – spoločná logika pre Tauri verziu (desktop/) aj natívnu verziu (native/).
pub mod fsops;
pub mod pty;
pub mod python;
pub mod runner;
pub mod settings;

use serde_json::Value;
use std::sync::{Arc, Mutex};

// Stav, ktorý zdieľajú všetky časti: nastavenia (settings.json) a otvorený projekt.
pub struct Core {
    pub settings: Mutex<Value>,
    pub workspace: Mutex<Option<String>>,
}

impl Core {
    pub fn load() -> Self {
        Core { settings: Mutex::new(settings::load()), workspace: Mutex::new(None) }
    }
    pub fn setting(&self, key: &str) -> Value {
        self.settings.lock().unwrap().get(key).cloned().unwrap_or(Value::Null)
    }
}

// Udalosť pre okno (kanál + dáta) – Tauri ju pošle cez emit, natívna verzia do svojej fronty.
pub type Emit = Arc<dyn Fn(&str, Value) + Send + Sync>;
