use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
pub enum Preset {
    #[value(name = "sss-1")]
    #[serde(rename = "sss-1")]
    Sss1,
    #[value(name = "sss-2")]
    #[serde(rename = "sss-2")]
    Sss2,
}

impl Preset {
    pub fn details(self) -> PresetDetails {
        match self {
            Self::Sss1 => PresetDetails {
                preset: self,
                description: "Standard operational preset with pause/freeze controls and simpler policy surface.",
                enable_permanent_delegate: false,
                enable_transfer_hook: false,
                default_account_frozen: false,
            },
            Self::Sss2 => PresetDetails {
                preset: self,
                description: "Compliance preset with transfer-hook enforcement and frozen-by-default accounts.",
                enable_permanent_delegate: true,
                enable_transfer_hook: true,
                default_account_frozen: true,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetDetails {
    pub preset: Preset,
    pub description: &'static str,
    pub enable_permanent_delegate: bool,
    pub enable_transfer_hook: bool,
    pub default_account_frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureFlags {
    pub enable_permanent_delegate: bool,
    pub enable_transfer_hook: bool,
    pub default_account_frozen: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitConfigFile {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub uri: String,
    pub preset: Preset,
    #[serde(default)]
    pub authority_keypair: Option<String>,
    #[serde(default)]
    pub rpc_url: Option<String>,
    #[serde(default)]
    pub api_url: Option<String>,
    #[serde(default)]
    pub mint: Option<String>,
    pub features: FeatureFlags,
}

impl InitConfigFile {
    pub fn from_preset(
        preset: Preset,
        name: String,
        symbol: String,
        decimals: u8,
        uri: String,
    ) -> Self {
        let details = preset.details();
        Self {
            name,
            symbol,
            decimals,
            uri,
            preset,
            authority_keypair: None,
            rpc_url: None,
            api_url: None,
            mint: None,
            features: FeatureFlags {
                enable_permanent_delegate: details.enable_permanent_delegate,
                enable_transfer_hook: details.enable_transfer_hook,
                default_account_frozen: details.default_account_frozen,
            },
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            bail!("init config name is required");
        }
        if self.symbol.trim().is_empty() {
            bail!("init config symbol is required");
        }
        if self.uri.trim().is_empty() {
            bail!("init config uri is required");
        }
        if self.decimals > 18 {
            bail!("init config decimals must be <= 18");
        }
        if self.features.enable_transfer_hook && !self.features.enable_permanent_delegate {
            bail!("transfer hook requires permanent delegate");
        }
        Ok(())
    }

    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).context("serialize init config to toml")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProfileConfig {
    pub cluster: Option<String>,
    pub rpc_url: Option<String>,
    pub api_url: Option<String>,
    pub mint: Option<String>,
    pub execution_policy: Option<String>,
}

pub fn load_init_config(path: &Path) -> Result<InitConfigFile> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("read init config {}", path.display()))?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let config: InitConfigFile = match extension {
        "json" => serde_json::from_str(&contents)
            .with_context(|| format!("parse json init config {}", path.display()))?,
        _ => toml::from_str(&contents)
            .with_context(|| format!("parse toml init config {}", path.display()))?,
    };
    config.validate()?;
    Ok(config)
}

pub fn write_init_config(path: &Path, config: &InitConfigFile) -> Result<()> {
    let contents = match path.extension().and_then(|value| value.to_str()) {
        Some("json") => serde_json::to_string_pretty(config).context("serialize json init config")?,
        _ => config.to_toml_string()?,
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create parent dir for {}", path.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("write init config {}", path.display()))?;
    Ok(())
}

pub fn default_config_path() -> std::path::PathBuf {
    std::env::var_os("SSS_CONFIG")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("config.toml"))
}

pub fn load_runtime_config() -> Result<Option<InitConfigFile>> {
    let path = default_config_path();
    if path.exists() {
        return load_init_config(&path).map(Some);
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn preset_sss2_sets_expected_flags() {
        let config = InitConfigFile::from_preset(
            Preset::Sss2,
            "Acme USD".into(),
            "AUSD".into(),
            6,
            "https://example.com".into(),
        );

        assert!(config.features.enable_permanent_delegate);
        assert!(config.features.enable_transfer_hook);
        assert!(config.features.default_account_frozen);
    }

    #[test]
    fn validate_requires_uri() {
        let config = InitConfigFile::from_preset(
            Preset::Sss1,
            "Acme USD".into(),
            "AUSD".into(),
            6,
            String::new(),
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn loads_toml_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            r#"
name = "Acme USD"
symbol = "AUSD"
decimals = 6
uri = "https://example.com/ausd.json"
preset = "sss-2"
rpc_url = "https://api.devnet.solana.com"
api_url = "http://127.0.0.1:8080"
mint = "Mint111111111111111111111111111111111111111"

[features]
enable_permanent_delegate = true
enable_transfer_hook = true
default_account_frozen = true
"#,
        )
        .unwrap();

        let config = load_init_config(&path).unwrap();
        assert_eq!(config.preset, Preset::Sss2);
        assert_eq!(config.uri, "https://example.com/ausd.json");
        assert_eq!(config.rpc_url.as_deref(), Some("https://api.devnet.solana.com"));
        assert_eq!(config.api_url.as_deref(), Some("http://127.0.0.1:8080"));
    }
}
