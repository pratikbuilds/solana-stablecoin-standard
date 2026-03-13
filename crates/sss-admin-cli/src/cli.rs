use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::config::Preset;

#[derive(Debug, Parser)]
#[command(name = "sss-token", about = "Solana Stablecoin Standard admin CLI")]
pub struct Cli {
    #[arg(long, global = true)]
    pub profile: Option<String>,
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init(InitArgs),
    Mint(MintArgs),
    Burn(BurnArgs),
    Freeze(AddressArgs),
    Thaw(AddressArgs),
    Pause(ConfirmArgs),
    Unpause(ConfirmArgs),
    Blacklist {
        #[command(subcommand)]
        command: BlacklistCommand,
    },
    Seize(SeizeArgs),
    Minters {
        #[command(subcommand)]
        command: MintersCommand,
    },
    Holders(HoldersArgs),
    AuditLog(AuditLogArgs),
    Status(ReadArgs),
    Supply(ReadArgs),
}

#[derive(Debug, Args, Clone)]
pub struct ConfirmArgs {
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Args, Clone)]
pub struct ReadArgs {
    #[arg(long)]
    pub mint: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct InitArgs {
    #[arg(long)]
    pub preset: Option<Preset>,
    #[arg(long)]
    pub config: Option<PathBuf>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub wizard: bool,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub symbol: Option<String>,
    #[arg(long)]
    pub decimals: Option<u8>,
    #[arg(long)]
    pub uri: Option<String>,
    #[arg(long)]
    pub authority_keypair: Option<String>,
    #[arg(long)]
    pub rpc_url: Option<String>,
    #[arg(long)]
    pub api_url: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub dry_run: bool,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Args, Clone)]
pub struct MintArgs {
    pub recipient: String,
    pub amount: String,
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Args, Clone)]
pub struct BurnArgs {
    pub amount: String,
    #[arg(long)]
    pub account: Option<String>,
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Args, Clone)]
pub struct AddressArgs {
    pub address: String,
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum BlacklistCommand {
    Add(BlacklistAddArgs),
    Remove(BlacklistRemoveArgs),
}

#[derive(Debug, Args, Clone)]
pub struct BlacklistAddArgs {
    pub address: String,
    #[arg(long)]
    pub reason: String,
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Args, Clone)]
pub struct BlacklistRemoveArgs {
    pub address: String,
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Args, Clone)]
pub struct SeizeArgs {
    pub address: String,
    #[arg(long)]
    pub to: String,
    #[arg(long)]
    pub amount: Option<String>,
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Subcommand, Clone)]
pub enum MintersCommand {
    List(MinterListArgs),
    Add(MinterAddArgs),
    Remove(MinterRemoveArgs),
}

#[derive(Debug, Args, Clone)]
pub struct MinterListArgs {
    #[arg(long)]
    pub mint: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct MinterAddArgs {
    pub address: String,
    #[arg(long)]
    pub quota: String,
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Args, Clone)]
pub struct MinterRemoveArgs {
    pub address: String,
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    pub yes: bool,
}

#[derive(Debug, Args, Clone)]
pub struct HoldersArgs {
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long)]
    pub min_balance: Option<String>,
    #[arg(long)]
    pub limit: Option<u32>,
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum AuditAction {
    Mint,
    Burn,
    Freeze,
    Thaw,
    Pause,
    Unpause,
    BlacklistAdd,
    BlacklistRemove,
    Seize,
}

#[derive(Debug, Args, Clone)]
pub struct AuditLogArgs {
    #[arg(long)]
    pub mint: Option<String>,
    #[arg(long)]
    pub action: Option<AuditAction>,
    #[arg(long)]
    pub wallet: Option<String>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long)]
    pub limit: Option<u32>,
    #[arg(long)]
    pub cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_init_with_preset_and_metadata() {
        let cli = Cli::parse_from([
            "sss-token",
            "init",
            "--preset",
            "sss-2",
            "--name",
            "Acme USD",
            "--symbol",
            "AUSD",
            "--decimals",
            "6",
            "--uri",
            "https://example.com/ausd.json",
        ]);

        match cli.command {
            Command::Init(args) => {
                assert_eq!(args.preset, Some(Preset::Sss2));
                assert_eq!(args.name.as_deref(), Some("Acme USD"));
                assert_eq!(args.symbol.as_deref(), Some("AUSD"));
                assert_eq!(args.decimals, Some(6));
                assert_eq!(args.uri.as_deref(), Some("https://example.com/ausd.json"));
            }
            _ => panic!("expected init command"),
        }
    }

    #[test]
    fn parses_mint_command() {
        let cli = Cli::parse_from(["sss-token", "mint", "Recipient111", "1000"]);
        match cli.command {
            Command::Mint(args) => {
                assert_eq!(args.recipient, "Recipient111");
                assert_eq!(args.amount, "1000");
            }
            _ => panic!("expected mint command"),
        }
    }

    #[test]
    fn parses_blacklist_add_command() {
        let cli = Cli::parse_from([
            "sss-token",
            "blacklist",
            "add",
            "Wallet111",
            "--reason",
            "OFAC match",
        ]);
        match cli.command {
            Command::Blacklist {
                command: BlacklistCommand::Add(args),
            } => {
                assert_eq!(args.address, "Wallet111");
                assert_eq!(args.reason, "OFAC match");
            }
            _ => panic!("expected blacklist add command"),
        }
    }

    #[test]
    fn parses_minters_list() {
        let cli = Cli::parse_from(["sss-token", "minters", "list"]);
        match cli.command {
            Command::Minters {
                command: MintersCommand::List(_),
            } => {}
            _ => panic!("expected minters list command"),
        }
    }
}
