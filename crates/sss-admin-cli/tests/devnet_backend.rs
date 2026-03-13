use anyhow::Result;
use sss_admin_cli::run_with_args;
use std::{
    env,
    fs,
    path::{Path, PathBuf},
};
use tempfile::tempdir;

struct TestEnvGuard {
    cwd: PathBuf,
    vars: Vec<(&'static str, Option<String>)>,
}

impl TestEnvGuard {
    fn enter(config_dir: &Path) -> Result<Self> {
        let cwd = env::current_dir()?;
        env::set_current_dir(config_dir)?;
        let vars = vec![
            ("SSS_CONFIG", env::var("SSS_CONFIG").ok()),
            ("SSS_MINT", env::var("SSS_MINT").ok()),
            ("SSS_API_URL", env::var("SSS_API_URL").ok()),
            ("SOLANA_RPC_URL", env::var("SOLANA_RPC_URL").ok()),
            ("SSS_AUTHORITY_KEYPAIR", env::var("SSS_AUTHORITY_KEYPAIR").ok()),
        ];
        Ok(Self { cwd, vars })
    }
}

impl Drop for TestEnvGuard {
    fn drop(&mut self) {
        let _ = env::set_current_dir(&self.cwd);
        for (key, value) in &self.vars {
            match value {
                Some(value) => env::set_var(key, value),
                None => env::remove_var(key),
            }
        }
    }
}

fn require_env(name: &str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn write_config(
    dir: &Path,
    mint: Option<&str>,
    api_url: Option<&str>,
    rpc_url: Option<&str>,
    authority_keypair: Option<&str>,
) -> Result<PathBuf> {
    let path = dir.join("config.toml");
    let mint_line = mint
        .map(|value| format!("mint = \"{value}\"\n"))
        .unwrap_or_default();
    let api_line = api_url
        .map(|value| format!("api_url = \"{value}\"\n"))
        .unwrap_or_default();
    let rpc_line = rpc_url
        .map(|value| format!("rpc_url = \"{value}\"\n"))
        .unwrap_or_default();
    let authority_line = authority_keypair
        .map(|value| format!("authority_keypair = \"{value}\"\n"))
        .unwrap_or_default();

    fs::write(
        &path,
        format!(
            concat!(
                "name = \"Integration USD\"\n",
                "symbol = \"IUSD\"\n",
                "decimals = 6\n",
                "uri = \"https://example.com/integration.json\"\n",
                "preset = \"sss-2\"\n",
                "{mint_line}",
                "{api_line}",
                "{rpc_line}",
                "{authority_line}",
                "\n[features]\n",
                "enable_permanent_delegate = true\n",
                "enable_transfer_hook = true\n",
                "default_account_frozen = true\n"
            ),
            mint_line = mint_line,
            api_line = api_line,
            rpc_line = rpc_line,
            authority_line = authority_line,
        ),
    )?;
    Ok(path)
}

#[test]
fn devnet_backend_status_and_audit_log_use_config() -> Result<()> {
    let Some(api_url) = require_env("SSS_API_URL") else {
        return Ok(());
    };
    let Some(mint) = require_env("SSS_MINT") else {
        return Ok(());
    };

    let dir = tempdir()?;
    write_config(dir.path(), Some(&mint), Some(&api_url), None, None)?;
    let _guard = TestEnvGuard::enter(dir.path())?;

    run_with_args(["sss-token", "status"])?;
    run_with_args(["sss-token", "supply"])?;
    run_with_args(["sss-token", "audit-log", "--limit", "1"])?;

    Ok(())
}

#[test]
fn devnet_chain_holders_and_minters_use_config() -> Result<()> {
    let Some(rpc_url) = require_env("SOLANA_RPC_URL") else {
        return Ok(());
    };
    let Some(mint) = require_env("SSS_MINT") else {
        return Ok(());
    };
    let authority = require_env("SSS_AUTHORITY_KEYPAIR");

    let dir = tempdir()?;
    write_config(
        dir.path(),
        Some(&mint),
        None,
        Some(&rpc_url),
        authority.as_deref(),
    )?;
    let _guard = TestEnvGuard::enter(dir.path())?;

    run_with_args(["sss-token", "holders", "--limit", "1"])?;
    run_with_args(["sss-token", "minters", "list"])?;

    Ok(())
}

#[test]
#[ignore = "requires explicit devnet mutation opt-in"]
fn devnet_init_persists_created_mint_to_config() -> Result<()> {
    let Some(rpc_url) = require_env("SOLANA_RPC_URL") else {
        return Ok(());
    };
    let Some(authority_keypair) = require_env("SSS_AUTHORITY_KEYPAIR") else {
        return Ok(());
    };
    if require_env("SSS_ADMIN_CLI_RUN_DEVNET_MUTATIONS").is_none() {
        return Ok(());
    }

    let dir = tempdir()?;
    let config_path = dir.path().join("config.toml");
    let _guard = TestEnvGuard::enter(dir.path())?;

    run_with_args([
        "sss-token",
        "init",
        "--config",
        config_path.to_str().unwrap(),
        "--preset",
        "sss-1",
        "--name",
        "Integration USD",
        "--symbol",
        "IUSD",
        "--decimals",
        "6",
        "--uri",
        "https://example.com/integration.json",
        "--rpc-url",
        &rpc_url,
        "--authority-keypair",
        &authority_keypair,
        "--yes",
    ])?;

    let written = fs::read_to_string(&config_path)?;
    assert!(written.contains("mint = \""));
    Ok(())
}
