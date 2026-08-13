//! User preferences — JSON persistence under Application Support.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::theme::AppTheme;

const FILE_NAME: &str = "preferences.json";
const LEGACY_ONBOARDING_FLAG: &str = "onboarding_done";

static STORE: OnceLock<Arc<Mutex<PreferencesStore>>> = OnceLock::new();

/// Persisted preferences (schema v1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default = "default_poll_ms")]
    pub poll_interval_ms: u64,
    #[serde(default = "default_section")]
    pub default_section: String,
    #[serde(default)]
    pub onboarding_done: bool,
    #[serde(default)]
    pub menubar_only: bool,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    AppTheme::default_theme().id().into()
}

fn default_poll_ms() -> u64 {
    1000
}

fn default_section() -> String {
    "overview".into()
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            poll_interval_ms: default_poll_ms(),
            default_section: default_section(),
            onboarding_done: false,
            menubar_only: false,
            theme: default_theme(),
        }
    }
}

impl Preferences {
    pub fn app_theme(&self) -> AppTheme {
        AppTheme::from_id(&self.theme)
    }

    pub fn set_theme(&mut self, theme: AppTheme) {
        self.theme = theme.id().into();
    }
    pub fn poll_interval(&self) -> Duration {
        Duration::from_millis(normalize_poll_ms(self.poll_interval_ms))
    }

    pub fn normalized_poll_ms(&self) -> u64 {
        normalize_poll_ms(self.poll_interval_ms)
    }
}

pub fn normalize_poll_ms(ms: u64) -> u64 {
    match ms {
        0..=749 => 500,
        750..=1499 => 1000,
        _ => 2000,
    }
}

#[derive(Clone, Debug)]
pub struct PreferencesStore {
    path: PathBuf,
    legacy_flag_path: PathBuf,
    pub prefs: Preferences,
}

impl PreferencesStore {
    pub fn load() -> Self {
        Self::load_at(production_prefs_path(), production_legacy_flag_path())
    }

    pub fn load_at(prefs_path: PathBuf, legacy_flag_path: PathBuf) -> Self {
        let mut store = if prefs_path.is_file() {
            match fs::read_to_string(&prefs_path) {
                Ok(text) => match serde_json::from_str(&text) {
                    Ok(prefs) => Self {
                        path: prefs_path,
                        legacy_flag_path,
                        prefs,
                    },
                    Err(err) => {
                        eprintln!("Osman: invalid preferences.json ({err}); using defaults");
                        Self {
                            path: prefs_path,
                            legacy_flag_path,
                            prefs: Preferences::default(),
                        }
                    }
                },
                Err(err) => {
                    eprintln!("Osman: could not read preferences.json ({err}); using defaults");
                    Self {
                        path: prefs_path,
                        legacy_flag_path,
                        prefs: Preferences::default(),
                    }
                }
            }
        } else {
            Self {
                path: prefs_path,
                legacy_flag_path,
                prefs: Preferences::default(),
            }
        };

        if store.migrate_legacy_onboarding_flag() {
            let _ = store.save();
        }

        store
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get(&self) -> &Preferences {
        &self.prefs
    }

    pub fn save(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.prefs)?;
        fs::write(&self.path, json)
    }

    pub fn set_onboarding_done(&mut self) -> io::Result<()> {
        self.prefs.onboarding_done = true;
        self.save()
    }

    pub fn set_theme(&mut self, theme: AppTheme) -> io::Result<()> {
        self.prefs.set_theme(theme);
        self.save()
    }

    pub fn set_poll_interval_ms(&mut self, ms: u64) -> io::Result<()> {
        self.prefs.poll_interval_ms = normalize_poll_ms(ms);
        self.save()
    }

    pub fn set_default_section(&mut self, section: &str) -> io::Result<()> {
        self.prefs.default_section = section.into();
        self.save()
    }

    pub fn set_menubar_only(&mut self, enabled: bool) -> io::Result<()> {
        self.prefs.menubar_only = enabled;
        self.save()
    }

    fn migrate_legacy_onboarding_flag(&mut self) -> bool {
        if self.prefs.onboarding_done {
            let _ = fs::remove_file(&self.legacy_flag_path);
            return false;
        }
        if self.legacy_flag_path.is_file() {
            self.prefs.onboarding_done = true;
            let _ = fs::remove_file(&self.legacy_flag_path);
            return true;
        }
        false
    }
}

pub fn init(store: PreferencesStore) {
    let _ = STORE.set(Arc::new(Mutex::new(store)));
}

pub fn ensure_init() {
    if STORE.get().is_none() {
        init(PreferencesStore::load());
    }
}

pub fn with_store<F, R>(f: F) -> R
where
    F: FnOnce(&mut PreferencesStore) -> R,
{
    let arc = STORE
        .get()
        .expect("preferences::init must run before app launch");
    let mut guard = arc.lock().expect("preferences lock");
    f(&mut guard)
}

pub fn get() -> Preferences {
    STORE
        .get()
        .expect("preferences::init must run before app launch")
        .lock()
        .expect("preferences lock")
        .prefs
        .clone()
}

fn production_support_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join("Library/Application Support/Osman")
    } else {
        PathBuf::from(".local/share/Osman")
    }
}

fn production_prefs_path() -> PathBuf {
    production_support_dir().join(FILE_NAME)
}

fn production_legacy_flag_path() -> PathBuf {
    production_support_dir().join(LEGACY_ONBOARDING_FLAG)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (PreferencesStore, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("osman-prefs-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("temp dir");
        let prefs = base.join("preferences.json");
        let legacy = base.join(LEGACY_ONBOARDING_FLAG);
        let store = PreferencesStore::load_at(prefs.clone(), legacy.clone());
        (store, prefs, legacy)
    }

    #[test]
    fn default_preferences() {
        let prefs = Preferences::default();
        assert_eq!(prefs.poll_interval_ms, 1000);
        assert_eq!(prefs.default_section, "overview");
        assert!(!prefs.onboarding_done);
        assert_eq!(prefs.theme, "clinical_sage");
        assert_eq!(prefs.app_theme(), AppTheme::ClinicalSage);
    }

    #[test]
    fn round_trip_json() {
        let (mut store, path, _) = temp_store();
        store.prefs.poll_interval_ms = 2000;
        store.prefs.default_section = "connections".into();
        store.prefs.onboarding_done = true;
        store.prefs.set_theme(AppTheme::OceanPulse);
        store.save().expect("save");

        let loaded = PreferencesStore::load_at(path, std::env::temp_dir().join("unused"));
        assert_eq!(loaded.prefs, store.prefs);
        let _ = fs::remove_dir_all(store.path.parent().unwrap());
    }

    #[test]
    fn missing_file_loads_defaults() {
        let (store, _, _) = temp_store();
        assert_eq!(store.prefs, Preferences::default());
        let _ = fs::remove_dir_all(store.path.parent().unwrap());
    }

    #[test]
    fn migrate_legacy_onboarding_flag() {
        let base = std::env::temp_dir().join(format!("osman-prefs-migrate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("temp dir");
        let prefs = base.join("preferences.json");
        let legacy = base.join(LEGACY_ONBOARDING_FLAG);
        fs::write(&legacy, "1").expect("legacy flag");

        let store = PreferencesStore::load_at(prefs, legacy.clone());
        assert!(store.prefs.onboarding_done);
        assert!(!legacy.exists());
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn poll_interval_clamps_to_allowed_values() {
        assert_eq!(normalize_poll_ms(100), 500);
        assert_eq!(normalize_poll_ms(900), 1000);
        assert_eq!(normalize_poll_ms(5000), 2000);
    }

    #[test]
    fn store_setters_persist() {
        let (mut store, path, _) = temp_store();
        store.set_poll_interval_ms(500).expect("poll");
        store.set_default_section("processes").expect("section");
        store.set_menubar_only(true).expect("menubar");
        store.set_theme(AppTheme::SolarScope).expect("theme");

        let loaded = PreferencesStore::load_at(path, std::env::temp_dir().join("unused"));
        assert_eq!(loaded.prefs.poll_interval_ms, 500);
        assert_eq!(loaded.prefs.default_section, "processes");
        assert!(loaded.prefs.menubar_only);
        assert_eq!(loaded.prefs.app_theme(), AppTheme::SolarScope);
        let _ = fs::remove_dir_all(store.path.parent().unwrap());
    }
}
