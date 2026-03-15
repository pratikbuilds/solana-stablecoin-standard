use anyhow::{bail, Context, Result};
use std::path::PathBuf;

use crate::{
    cli::InitArgs,
    config::{default_config_path, load_init_config, write_init_config, InitConfigFile, Preset},
};

#[derive(Debug, Clone)]
pub struct InitPlan {
    pub config: InitConfigFile,
    pub source: InitSource,
}

#[derive(Debug, Clone)]
pub enum InitSource {
    ExistingConfig { path: PathBuf },
    MissingConfig { path: PathBuf },
    Preset,
    Wizard,
}

impl InitPlan {
    pub fn render_human(&self) -> String {
        let details = self.config.preset.details();
        let source = match &self.source {
            InitSource::ExistingConfig { path } => format!("config: {}", path.display()),
            InitSource::MissingConfig { path } => {
                format!("new config will be written to {}", path.display())
            }
            InitSource::Preset => "inline preset arguments".to_string(),
            InitSource::Wizard => "wizard".to_string(),
        };

        format!(
            concat!(
                "Init plan\n",
                "  source: {source}\n",
                "  preset: {preset:?}\n",
                "  description: {description}\n",
                "  name: {name}\n",
                "  symbol: {symbol}\n",
                "  decimals: {decimals}\n",
                "  uri: {uri}\n",
                "  permanent delegate: {permanent_delegate}\n",
                "  transfer hook: {transfer_hook}\n",
                "  frozen by default: {default_frozen}\n"
            ),
            source = source,
            preset = self.config.preset,
            description = details.description,
            name = self.config.name,
            symbol = self.config.symbol,
            decimals = self.config.decimals,
            uri = self.config.uri,
            permanent_delegate = self.config.features.enable_permanent_delegate,
            transfer_hook = self.config.features.enable_transfer_hook,
            default_frozen = self.config.features.default_account_frozen,
        )
    }

    pub fn maybe_persist_config(&self) -> Result<()> {
        write_init_config(&self.persist_path(), &self.config)?;
        Ok(())
    }

    pub fn persist_with_mint(&mut self, mint: &str) -> Result<()> {
        self.config.mint = Some(mint.to_string());
        write_init_config(&self.persist_path(), &self.config)
    }

    pub fn persist_path(&self) -> PathBuf {
        match &self.source {
            InitSource::ExistingConfig { path } | InitSource::MissingConfig { path } => path.clone(),
            InitSource::Preset | InitSource::Wizard => default_config_path(),
        }
    }
}

pub fn prepare_init(args: &InitArgs, rpc_override: Option<&str>) -> Result<InitPlan> {
    let config_path = args.config.clone().or_else(|| args.custom.clone());

    if args.wizard {
        if config_path.is_some() || args.preset.is_some() {
            bail!("init wizard cannot be combined with --config/--custom or --preset");
        }
        return build_inline_config(args, InitSource::Wizard, rpc_override);
    }

    if let Some(path) = &config_path {
        if path.exists() {
            if args.preset.is_some() || args.name.is_some() || args.symbol.is_some() || args.decimals.is_some() || args.uri.is_some() {
                bail!("existing init config cannot be combined with inline preset or metadata flags");
            }
            let mut config = load_init_config(path)?;
            apply_runtime_overrides(&mut config, args, rpc_override);
            return Ok(InitPlan {
                config,
                source: InitSource::ExistingConfig { path: path.clone() },
            });
        }

        let plan = build_inline_config(args, InitSource::MissingConfig { path: path.clone() }, rpc_override)?;
        plan.config.validate()?;
        return Ok(plan);
    }

    if args.preset.is_none() {
        bail!("init requires --preset when no config file is provided");
    }

    build_inline_config(args, InitSource::Preset, rpc_override)
}

fn build_inline_config(args: &InitArgs, source: InitSource, rpc_override: Option<&str>) -> Result<InitPlan> {
    let preset = args
        .preset
        .or(match source {
            InitSource::Wizard => Some(Preset::Sss1),
            _ => None,
        })
        .context("init preset is required for inline initialization")?;
    let defaults = default_metadata_for_preset(preset);
    let name = args.name.clone().unwrap_or_else(|| defaults.name.to_string());
    let symbol = args
        .symbol
        .clone()
        .unwrap_or_else(|| defaults.symbol.to_string());
    let decimals = args.decimals.unwrap_or(defaults.decimals);
    let uri = args.uri.clone().unwrap_or_else(|| defaults.uri.to_string());

    let config = InitConfigFile::from_preset(preset, name, symbol, decimals, uri);
    let mut config = config;
    apply_runtime_overrides(&mut config, args, rpc_override);
    config.validate()?;
    Ok(InitPlan { config, source })
}

fn apply_runtime_overrides(config: &mut InitConfigFile, args: &InitArgs, rpc_override: Option<&str>) {
    if let Some(authority_keypair) = &args.authority_keypair {
        config.authority_keypair = Some(authority_keypair.clone());
    }
    if let Some(rpc_url) = rpc_override {
        config.rpc_url = Some(rpc_url.to_string());
    }
    if let Some(api_url) = &args.api_url {
        config.api_url = Some(api_url.clone());
    }
}

struct PresetMetadataDefaults {
    name: &'static str,
    symbol: &'static str,
    decimals: u8,
    uri: &'static str,
}

fn default_metadata_for_preset(preset: Preset) -> PresetMetadataDefaults {
    match preset {
        Preset::Sss1 => PresetMetadataDefaults {
            name: "Simple USD",
            symbol: "SUSD",
            decimals: 6,
            uri: "https://example.com/sss1.json",
        },
        Preset::Sss2 => PresetMetadataDefaults {
            name: "Regulated USD",
            symbol: "RUSD",
            decimals: 6,
            uri: "https://example.com/sss2.json",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::InitArgs;
    use tempfile::tempdir;

    fn inline_args() -> InitArgs {
        InitArgs {
            preset: Some(Preset::Sss2),
            config: None,
            custom: None,
            wizard: false,
            name: Some("Acme USD".into()),
            symbol: Some("AUSD".into()),
            decimals: Some(6),
            uri: Some("https://example.com/ausd.json".into()),
            authority_keypair: Some("/tmp/id.json".into()),
            api_url: Some("http://127.0.0.1:8080".into()),
            dry_run: false,
            yes: false,
        }
    }

    #[test]
    fn builds_init_plan_from_preset_args() {
        let plan = prepare_init(&inline_args(), Some("https://api.devnet.solana.com")).unwrap();
        assert_eq!(plan.config.preset, Preset::Sss2);
        assert_eq!(plan.config.uri, "https://example.com/ausd.json");
        assert_eq!(plan.config.rpc_url.as_deref(), Some("https://api.devnet.solana.com"));
    }

    #[test]
    fn missing_inline_uri_uses_preset_default() {
        let mut args = inline_args();
        args.uri = None;
        let plan = prepare_init(&args, Some("https://api.devnet.solana.com")).unwrap();
        assert_eq!(plan.config.uri, "https://example.com/sss2.json");
    }

    #[test]
    fn preset_only_init_uses_defaults() {
        let args = InitArgs {
            preset: Some(Preset::Sss1),
            config: None,
            custom: None,
            wizard: false,
            name: None,
            symbol: None,
            decimals: None,
            uri: None,
            authority_keypair: None,
            api_url: None,
            dry_run: false,
            yes: false,
        };

        let plan = prepare_init(&args, None).unwrap();
        assert_eq!(plan.config.name, "Simple USD");
        assert_eq!(plan.config.symbol, "SUSD");
        assert_eq!(plan.config.decimals, 6);
        assert_eq!(plan.config.uri, "https://example.com/sss1.json");
    }

    #[test]
    fn persists_missing_config_after_prepare() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut args = inline_args();
        args.config = Some(path.clone());
        args.preset = Some(Preset::Sss1);

        let plan = prepare_init(&args, Some("https://api.devnet.solana.com")).unwrap();
        assert!(!path.exists());

        plan.maybe_persist_config().unwrap();

        assert!(path.exists());
        let written = std::fs::read_to_string(path).unwrap();
        assert!(written.contains("uri = \"https://example.com/ausd.json\""));
        assert!(written.contains("rpc_url = \"https://api.devnet.solana.com\""));
    }

    #[test]
    fn persists_mint_into_config() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut args = inline_args();
        args.config = Some(path.clone());

        let mut plan = prepare_init(&args, None).unwrap();
        plan.persist_with_mint("Mint111111111111111111111111111111111111111")
            .unwrap();

        let written = std::fs::read_to_string(path).unwrap();
        assert!(written.contains("mint = \"Mint111111111111111111111111111111111111111\""));
    }

    #[test]
    fn existing_config_accepts_global_rpc_override() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
name = "Acme USD"
symbol = "AUSD"
decimals = 6
uri = "https://example.com/ausd.json"
preset = "sss-2"
rpc_url = "https://old-rpc.test"

[features]
enable_permanent_delegate = true
enable_transfer_hook = true
default_account_frozen = true
"#,
        )
        .unwrap();

        let args = InitArgs {
            preset: None,
            config: Some(path),
            custom: None,
            wizard: false,
            name: None,
            symbol: None,
            decimals: None,
            uri: None,
            authority_keypair: None,
            api_url: None,
            dry_run: false,
            yes: false,
        };

        let plan = prepare_init(&args, Some("https://override-rpc.test")).unwrap();
        assert_eq!(plan.config.rpc_url.as_deref(), Some("https://override-rpc.test"));
    }
}
