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

pub fn prepare_init(args: &InitArgs) -> Result<InitPlan> {
    if args.wizard {
        if args.config.is_some() || args.preset.is_some() {
            bail!("init wizard cannot be combined with --config or --preset");
        }
        return build_inline_config(args, InitSource::Wizard);
    }

    if let Some(path) = &args.config {
        if path.exists() {
            if args.preset.is_some() || args.name.is_some() || args.symbol.is_some() || args.decimals.is_some() || args.uri.is_some() {
                bail!("existing init config cannot be combined with inline preset or metadata flags");
            }
            return Ok(InitPlan {
                config: load_init_config(path)?,
                source: InitSource::ExistingConfig { path: path.clone() },
            });
        }

        let plan = build_inline_config(args, InitSource::MissingConfig { path: path.clone() })?;
        plan.config.validate()?;
        return Ok(plan);
    }

    if args.preset.is_none() {
        bail!("init requires --preset when no config file is provided");
    }

    build_inline_config(args, InitSource::Preset)
}

fn build_inline_config(args: &InitArgs, source: InitSource) -> Result<InitPlan> {
    let preset = args
        .preset
        .or(match source {
            InitSource::Wizard => Some(Preset::Sss1),
            _ => None,
        })
        .context("init preset is required for inline initialization")?;
    let name = args
        .name
        .clone()
        .context("init name is required when config file is missing")?;
    let symbol = args
        .symbol
        .clone()
        .context("init symbol is required when config file is missing")?;
    let decimals = args
        .decimals
        .context("init decimals are required when config file is missing")?;
    let uri = args
        .uri
        .clone()
        .context("init uri is required when config file is missing")?;

    let config = InitConfigFile::from_preset(preset, name, symbol, decimals, uri);
    let mut config = config;
    config.authority_keypair = args.authority_keypair.clone();
    config.rpc_url = args.rpc_url.clone();
    config.api_url = args.api_url.clone();
    config.validate()?;
    Ok(InitPlan { config, source })
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
            wizard: false,
            name: Some("Acme USD".into()),
            symbol: Some("AUSD".into()),
            decimals: Some(6),
            uri: Some("https://example.com/ausd.json".into()),
            authority_keypair: Some("/tmp/id.json".into()),
            rpc_url: Some("https://api.devnet.solana.com".into()),
            api_url: Some("http://127.0.0.1:8080".into()),
            dry_run: false,
            yes: false,
        }
    }

    #[test]
    fn builds_init_plan_from_preset_args() {
        let plan = prepare_init(&inline_args()).unwrap();
        assert_eq!(plan.config.preset, Preset::Sss2);
        assert_eq!(plan.config.uri, "https://example.com/ausd.json");
    }

    #[test]
    fn missing_inline_uri_is_rejected() {
        let mut args = inline_args();
        args.uri = None;
        assert!(prepare_init(&args).is_err());
    }

    #[test]
    fn persists_missing_config_after_prepare() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut args = inline_args();
        args.config = Some(path.clone());
        args.preset = Some(Preset::Sss1);

        let plan = prepare_init(&args).unwrap();
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

        let mut plan = prepare_init(&args).unwrap();
        plan.persist_with_mint("Mint111111111111111111111111111111111111111")
            .unwrap();

        let written = std::fs::read_to_string(path).unwrap();
        assert!(written.contains("mint = \"Mint111111111111111111111111111111111111111\""));
    }
}
