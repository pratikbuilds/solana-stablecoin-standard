mod backend;
mod chain;
mod cli;
mod config;
mod init;

pub use cli::{Cli, Command};
pub use config::{InitConfigFile, Preset, PresetDetails, ProfileConfig};

use anyhow::Result;
use clap::Parser;
use sss_domain::{MintRecord, OperationRequest};

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
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let record = client.get_mint(&mint)?;
            print_mint_status(&record);
        }
        Command::Supply(args) => {
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let record = client.get_mint(&mint)?;
            println!(
                "mint: {}\nname: {}\nsymbol: {}\ncirculating_supply: {}",
                record.mint,
                record.name,
                record.symbol,
                record.total_minted - record.total_burned
            );
        }
        Command::Mint(args) => {
            confirm_or_abort(
                args.yes,
                &format!("Mint {} tokens to {}", args.amount, args.recipient),
            )?;
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let op = client.create_mint_request(
                mint,
                args.recipient,
                parse_amount(&args.amount)?,
                args.reason,
            )?;
            print_operation(&op);
        }
        Command::Burn(args) => {
            confirm_or_abort(args.yes, &format!("Burn {} tokens", args.amount))?;
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let op = client.create_burn_request(
                mint,
                args.account,
                parse_amount(&args.amount)?,
                args.reason,
            )?;
            print_operation(&op);
        }
        Command::Freeze(args) => {
            confirm_or_abort(args.yes, &format!("Freeze {}", args.address))?;
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let op = client.create_freeze_request(mint, args.address, args.reason)?;
            print_operation(&op);
        }
        Command::Thaw(args) => {
            confirm_or_abort(args.yes, &format!("Thaw {}", args.address))?;
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let op = client.create_thaw_request(mint, args.address, args.reason)?;
            print_operation(&op);
        }
        Command::Blacklist { command } => match command {
            cli::BlacklistCommand::Add(args) => {
                confirm_or_abort(args.yes, &format!("Blacklist {}", args.address))?;
                let client = BackendClient::from_runtime(runtime_config.as_ref())?;
                let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
                let op = client.create_blacklist_add_request(mint, args.address, args.reason)?;
                print_operation(&op);
            }
            cli::BlacklistCommand::Remove(args) => {
                confirm_or_abort(args.yes, &format!("Remove {} from blacklist", args.address))?;
                let client = BackendClient::from_runtime(runtime_config.as_ref())?;
                let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
                let op = client.create_blacklist_remove_request(mint, args.address)?;
                print_operation(&op);
            }
        },
        Command::Seize(args) => {
            confirm_or_abort(
                args.yes,
                &format!("Seize {} and send to {}", args.address, args.to),
            )?;
            let client = BackendClient::from_runtime(runtime_config.as_ref())?;
            let mint = resolve_mint(args.mint, runtime_config.as_ref())?;
            let op = client.create_seize_request(
                mint,
                args.address,
                args.to,
                args.amount.as_deref().map(parse_amount).transpose()?,
                args.reason,
            )?;
            print_operation(&op);
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
            let mut events = client.list_mint_events(&mint)?;
            if let Some(action) = args.action {
                let action_name = audit_action_name(action);
                events.retain(|event| event.event_type == action_name);
            }
            if let Some(wallet) = args.wallet {
                events.retain(|event| event.payload.to_string().contains(&wallet));
            }
            for event in events.into_iter().take(args.limit.unwrap_or(100) as usize) {
                println!(
                    "event_type: {}\nslot: {}\ntx_signature: {}\npayload: {}\n",
                    event.event_type, event.slot, event.tx_signature, event.payload
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

fn print_mint_status(record: &MintRecord) {
    println!(
        concat!(
            "mint: {}\n",
            "name: {}\n",
            "symbol: {}\n",
            "uri: {}\n",
            "preset: {}\n",
            "paused: {}\n",
            "permanent_delegate: {}\n",
            "transfer_hook: {}\n",
            "default_account_frozen: {}\n",
            "total_minted: {}\n",
            "total_burned: {}\n",
            "circulating_supply: {}\n"
        ),
        record.mint,
        record.name,
        record.symbol,
        record.uri,
        record.preset,
        record.paused,
        record.enable_permanent_delegate,
        record.enable_transfer_hook,
        record.default_account_frozen,
        record.total_minted,
        record.total_burned,
        record.total_minted - record.total_burned,
    );
}

fn print_operation(operation: &OperationRequest) {
    println!(
        "operation_id: {}\nkind: {}\nstatus: {}\nmint: {}",
        operation.id,
        operation.kind.as_str(),
        operation.status.as_str(),
        operation.mint
    );
}

fn audit_action_name(action: cli::AuditAction) -> String {
    match action {
        cli::AuditAction::Mint => "tokens_minted",
        cli::AuditAction::Burn => "tokens_burned",
        cli::AuditAction::Freeze => "account_frozen",
        cli::AuditAction::Thaw => "account_thawed",
        cli::AuditAction::Pause => "pause_changed",
        cli::AuditAction::Unpause => "pause_changed",
        cli::AuditAction::BlacklistAdd => "address_blacklisted",
        cli::AuditAction::BlacklistRemove => "address_unblacklisted",
        cli::AuditAction::Seize => "tokens_seized",
    }
    .to_string()
}
