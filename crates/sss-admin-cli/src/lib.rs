mod backend;
mod chain;
mod cli;
mod config;
mod init;

pub use cli::{Cli, Command};
pub use config::{InitConfigFile, Preset, PresetDetails, ProfileConfig};

use anyhow::Result;
use clap::Parser;
use sss_domain::LifecycleRequest;

use crate::backend::BackendClient;
use crate::chain::ChainClient;
use crate::config::load_runtime_config;

pub fn run() -> Result<()> {
    run_with_args(std::env::args_os())
}

pub fn run_with_args<I, T>(args: I) -> Result<()>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = Cli::parse_from(args);
    let runtime_config = load_runtime_config()?;
    match cli.command {
        Command::Init(args) => {
            let mut plan = init::prepare_init(&args)?;
            println!("{}", plan.render_human());
            if !args.dry_run {
                confirm_or_abort(args.yes, "Initialize stablecoin mint")?;
                let chain = ChainClient::from_runtime(Some(&plan.config))?;
                let execution = chain.init(&plan)?;
                println!(
                    "mint: {}\ninitialize_signature: {}\ndefault_minter_signature: {}",
                    execution.mint, execution.initialize_signature, execution.minter_signature
                );
                plan.persist_with_mint(&execution.mint.to_string())?;
            }
        }
        Command::Status(args) => {
            let chain = ChainClient::from_runtime(runtime_config.as_ref())?;
            let mint = parse_pubkey(&resolve_mint(args.mint, runtime_config.as_ref())?)?;
            let holders = chain.list_holders(mint, None)?;
            let supply: u64 = holders.iter().map(|h| h.amount).sum();
            println!("mint: {}\nsupply: {}", mint, supply);
        }
        Command::Supply(args) => {
            let chain = ChainClient::from_runtime(runtime_config.as_ref())?;
            let mint = parse_pubkey(&resolve_mint(args.mint, runtime_config.as_ref())?)?;
            let holders = chain.list_holders(mint, None)?;
            let supply: u64 = holders.iter().map(|h| h.amount).sum();
            println!("mint: {}\nsupply: {}", mint, supply);
        }
        Command::Mint(args) => {
            confirm_or_abort(
                args.yes,
                &format!("Mint {} tokens to {}", args.amount, args.recipient),
            )?;
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let request = client.create_mint_request(
                mint,
                args.recipient,
                parse_amount(&args.amount)?,
                args.reason,
            )?;
            print_lifecycle_request(&request);
        }
        Command::Burn(args) => {
            confirm_or_abort(args.yes, &format!("Burn {} tokens", args.amount))?;
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let request = client.create_burn_request(
                mint,
                args.account,
                parse_amount(&args.amount)?,
                args.reason,
            )?;
            print_lifecycle_request(&request);
        }
        Command::Freeze(_) => {
            anyhow::bail!("compliance endpoints removed; use chain client for freeze");
        }
        Command::Thaw(_) => {
            anyhow::bail!("compliance endpoints removed; use chain client for thaw");
        }
        Command::Blacklist { .. } => {
            anyhow::bail!("compliance endpoints removed; use chain client for blacklist");
        }
        Command::Seize(_) => {
            anyhow::bail!("compliance endpoints removed; use chain client for seize");
        }
        Command::Pause(args) => {
            confirm_or_abort(args.yes, "Pause mint operations")?;
            let chain = ChainClient::from_runtime(runtime_config.as_ref())?;
            let signature = chain.pause(parse_pubkey(&resolve_mint(args.mint, runtime_config.as_ref())?)?)?;
            println!("signature: {signature}");
        }
        Command::Unpause(args) => {
            confirm_or_abort(args.yes, "Unpause mint operations")?;
            let chain = ChainClient::from_runtime(runtime_config.as_ref())?;
            let signature = chain.unpause(parse_pubkey(&resolve_mint(args.mint, runtime_config.as_ref())?)?)?;
            println!("signature: {signature}");
        }
        Command::Minters { command } => match command {
            cli::MintersCommand::List(args) => {
                let chain = ChainClient::from_runtime(runtime_config.as_ref())?;
                let mint = parse_pubkey(&resolve_mint(args.mint, runtime_config.as_ref())?)?;
                for minter in chain.list_minters(mint)? {
                    println!(
                        "minter: {}\nquota: {}\nminted: {}\nactive: {}\n",
                        minter.minter, minter.quota, minter.minted, minter.active
                    );
                }
            }
            cli::MintersCommand::Add(args) => {
                confirm_or_abort(args.yes, &format!("Add minter {}", args.address))?;
                let chain = ChainClient::from_runtime(runtime_config.as_ref())?;
                let mint = parse_pubkey(&resolve_mint(args.mint, runtime_config.as_ref())?)?;
                let signature = chain.add_minter(
                    mint,
                    parse_pubkey(&args.address)?,
                    parse_amount_u64(&args.quota)?,
                )?;
                println!("signature: {signature}");
            }
            cli::MintersCommand::Remove(args) => {
                confirm_or_abort(args.yes, &format!("Remove minter {}", args.address))?;
                let chain = ChainClient::from_runtime(runtime_config.as_ref())?;
                let mint = parse_pubkey(&resolve_mint(args.mint, runtime_config.as_ref())?)?;
                let signature = chain.remove_minter(mint, parse_pubkey(&args.address)?)?;
                println!("signature: {signature}");
            }
        },
        Command::Operation { command } => match command {
            cli::OperationCommand::Get { id } => {
                let client = BackendClient::from_runtime(runtime_config.as_ref())?;
                let request = client.get_operation(&id)?;
                print_lifecycle_request(&request);
            }
            cli::OperationCommand::Approve { id, approved_by } => {
                let approved_by = approved_by
                    .or_else(|| std::env::var("USER").ok())
                    .unwrap_or_else(|| "sss-token".to_string());
                let client = BackendClient::from_runtime(runtime_config.as_ref())?;
                let request = client.approve_operation(&id, &approved_by)?;
                print_lifecycle_request(&request);
            }
            cli::OperationCommand::Execute { id } => {
                let client = BackendClient::from_runtime(runtime_config.as_ref())?;
                let request = client.execute_operation(&id)?;
                print_lifecycle_request(&request);
            }
        },
        Command::Holders(args) => {
            let chain = ChainClient::from_runtime(runtime_config.as_ref())?;
            let mint = parse_pubkey(&resolve_mint(args.mint, runtime_config.as_ref())?)?;
            let min_balance = args.min_balance.as_deref().map(parse_amount_u64).transpose()?;
            let holders = chain.list_holders(mint, min_balance)?;
            for holder in holders.into_iter().take(args.limit.unwrap_or(100) as usize) {
                println!(
                    "owner: {}\ntoken_account: {}\nbalance: {}\n",
                    holder.owner, holder.token_account, holder.amount
                );
            }
        }
        Command::AuditLog(args) => {
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let event_type = args.action.as_ref().map(|a| audit_action_name(a.clone()));
            let limit = args.limit.map(|l| l as u32);
            let mut events = client.list_mint_events(
                &mint,
                event_type.as_deref(),
                args.from.as_deref(),
                args.to.as_deref(),
                limit.or(Some(100)),
            )?;
            if let Some(wallet) = args.wallet {
                events.retain(|event| event.data.to_string().contains(&wallet));
            }
            for event in events.into_iter().take(args.limit.unwrap_or(100) as usize) {
                println!(
                    "event_type: {}\nslot: {}\ntx_signature: {}\ndata: {}\n",
                    event.event_type, event.slot, event.tx_signature, event.data
                );
            }
        }
    }

    Ok(())
}

fn resolve_mint(cli_mint: Option<String>, runtime_config: Option<&InitConfigFile>) -> Result<String> {
    cli_mint
        .or_else(|| runtime_config.and_then(|cfg| cfg.mint.clone()))
        .or_else(|| std::env::var("SSS_MINT").ok())
        .ok_or_else(|| anyhow::anyhow!("mint must be provided via --mint, config.mint, or SSS_MINT"))
}

fn parse_amount(value: &str) -> Result<i128> {
    value
        .parse::<i128>()
        .map_err(|error| anyhow::anyhow!("invalid amount {value}: {error}"))
}

fn parse_amount_u64(value: &str) -> Result<u64> {
    value
        .parse::<u64>()
        .map_err(|error| anyhow::anyhow!("invalid amount {value}: {error}"))
}

fn parse_pubkey(value: &str) -> Result<solana_sdk::pubkey::Pubkey> {
    value
        .parse()
        .map_err(|error| anyhow::anyhow!("invalid pubkey {value}: {error}"))
}

fn confirm_or_abort(skip: bool, summary: &str) -> Result<()> {
    if skip {
        return Ok(());
    }
    println!("Confirm action: {summary}");
    println!("Type 'yes' to continue:");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim() != "yes" {
        anyhow::bail!("aborted by operator");
    }
    Ok(())
}

fn print_lifecycle_request(request: &LifecycleRequest) {
    println!(
        "request_id: {}\ntype: {}\nstatus: {}\nmint: {}",
        request.id,
        request.type_.as_str(),
        request.status.as_str(),
        request.mint
    );
}

fn audit_action_name(action: cli::AuditAction) -> String {
    match action {
        cli::AuditAction::Mint => "TokensMinted",
        cli::AuditAction::Burn => "TokensBurned",
        cli::AuditAction::Freeze => "AccountFrozen",
        cli::AuditAction::Thaw => "AccountThawed",
        cli::AuditAction::Pause => "PauseChanged",
        cli::AuditAction::Unpause => "PauseChanged",
        cli::AuditAction::BlacklistAdd => "AddressBlacklisted",
        cli::AuditAction::BlacklistRemove => "AddressUnblacklisted",
        cli::AuditAction::Seize => "TokensSeized",
    }
    .to_string()
}
