use std::path::PathBuf;

use crate::types::Settings;

fn env_dir(env: &str, fallback: &str) -> PathBuf {
    std::env::var_os(env)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(fallback))
                .unwrap_or_else(std::env::temp_dir)
        })
}

pub fn config_dir() -> PathBuf {
    env_dir("XDG_CONFIG_HOME", ".config").join("convert")
}

pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join(".cache"))
                .unwrap_or_else(std::env::temp_dir)
        });
    base.join("convert")
}

pub fn validate(settings: &mut Settings) {
    if settings.output_mode != "custom" {
        settings.output_mode = "source".into();
    }
    if !matches!(settings.overwrite.as_str(), "rename" | "overwrite" | "skip") {
        settings.overwrite = "rename".into();
    }
    if !matches!(
        settings.theme.as_str(),
        "system" | "dark" | "light" | "ink" | "midnight" | "ember" | "paper" | "linen"
    ) {
        settings.theme = "dark".into();
    }
    settings.concurrency = settings.concurrency.clamp(1, 6);
    settings.default_quality = settings.default_quality.clamp(1, 100);
    if settings.filename_template.trim().is_empty() {
        settings.filename_template = "{name}".into();
    }
    if settings.output_mode != "custom" {
        settings.output_dir = settings.output_dir.trim().to_string();
    }
    // Drop preferences for formats that no longer exist or cannot be written.
    settings.default_targets.retain(|source, target| {
        source != target
            && crate::formats::lookup(source).is_some()
            && crate::formats::lookup(target)
                .map(|spec| !spec.read_only)
                .unwrap_or(false)
    });
}

pub fn load_settings() -> Settings {
    let path = config_dir().join("settings.json");
    let mut settings = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .unwrap_or_default();
    validate(&mut settings);
    settings
}

fn write_atomic(path: &PathBuf, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {}", parent.display(), e))?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("Could not write settings: {}", e))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Could not save settings: {}", e))?;
    Ok(())
}

pub fn save_settings(settings: &Settings) -> Result<(), String> {
    let text = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    write_atomic(&config_dir().join("settings.json"), &text)
}

pub fn clear_cache() -> Result<(), String> {
    let cache = cache_dir();
    for sub in ["previews", "filmstrips", "clipboard"] {
        let path = cache.join(sub);
        if path.exists() {
            std::fs::remove_dir_all(&path).map_err(|e| format!("Could not clear cache: {}", e))?;
        }
    }
    Ok(())
}
