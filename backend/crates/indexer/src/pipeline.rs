use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use base64::Engine;
use carbon_core::{
    collection::InstructionDecoderCollection, error::CarbonResult, instruction::DecodedInstruction,
    instruction::InstructionDecoder, metrics::MetricsCollection, pipeline::Pipeline,
    processor::Processor, transaction::TransactionProcessorInputType,
};
use carbon_rpc_block_crawler_datasource::{RpcBlockConfig, RpcBlockCrawler};
use carbon_rpc_block_subscribe_datasource::{Filters, RpcBlockSubscribe};
use carbon_rpc_transaction_crawler_datasource::{
    ConnectionConfig, Filters as TxCrawlerFilters, RpcTransactionCrawler,
};
use carbon_stablecoin_decoder::{instructions::StablecoinInstruction, StablecoinDecoder};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcBlockSubscribeConfig, RpcBlockSubscribeFilter},
};
use solana_sdk::{commitment_config::CommitmentConfig, instruction::Instruction};
use solana_transaction_status::TransactionDetails;
use solana_transaction_status_client_types::UiTransactionEncoding;
use tracing::info;
use transfer_hook::ID as TRANSFER_HOOK_PROGRAM_ID;

use crate::{
    decode::{decode_stablecoin_cpi_event, synthesize_transfer_hook_from_instruction},
    service::IndexerService,
};

pub(crate) async fn run_live(service: &IndexerService) -> Result<()> {
    service
        .store
        .upsert_checkpoint(
            "stablecoin-main",
            &service.config.stablecoin_program_id,
            service.config.start_slot,
            None,
        )
        .await?;
    service
        .store
        .upsert_checkpoint(
            "transfer-hook-main",
            &service.config.transfer_hook_program_id,
            service.config.start_slot,
            None,
        )
        .await?;

    let mut builder = Pipeline::builder().transaction::<IndexedInstruction, ()>(
        CarbonTransactionProcessor {
            service: service.clone(),
        },
        None,
    );

    let indexer_rpc = service
        .config
        .indexer_rpc_url
        .as_deref()
        .unwrap_or(&service.config.rpc_url);

    if !service.config.disable_block_subscribe {
        if is_helius_rpc(indexer_rpc) {
            let conn = ConnectionConfig {
                batch_limit: 1000,
                polling_interval: std::time::Duration::from_secs(5),
                max_concurrent_requests: 4,
                max_signature_channel_size: None,
                max_transaction_channel_size: None,
                retry_config: carbon_rpc_transaction_crawler_datasource::RetryConfig {
                    max_retries: 5,
                    initial_backoff_ms: 1000,
                    max_backoff_ms: 30_000,
                    backoff_multiplier: 2.0,
                },
                blocking_send: false,
            };
            let filters = TxCrawlerFilters {
                accounts: None,
                before_signature: None,
                until_signature: None,
            };
            if let Ok(pk) = service.config.stablecoin_program_id.parse() {
                let crawler = RpcTransactionCrawler {
                    rpc_url: indexer_rpc.to_string(),
                    account: pk,
                    connection_config: conn.clone(),
                    filters: filters.clone(),
                    commitment: Some(CommitmentConfig::confirmed()),
                };
                builder = builder.datasource(crawler);
            }
            if let Ok(pk) = service.config.transfer_hook_program_id.parse() {
                let crawler = RpcTransactionCrawler {
                    rpc_url: indexer_rpc.to_string(),
                    account: pk,
                    connection_config: conn,
                    filters,
                    commitment: Some(CommitmentConfig::confirmed()),
                };
                builder = builder.datasource(crawler);
            }
            info!("using carbon-transaction-crawler datasource (Helius RPC)");
        } else {
            let block_subscribe = RpcBlockSubscribe::new(
                to_ws_url(indexer_rpc),
                Filters::new(
                    RpcBlockSubscribeFilter::All,
                    Some(RpcBlockSubscribeConfig {
                        commitment: Some(CommitmentConfig::confirmed()),
                        encoding: Some(UiTransactionEncoding::Base64),
                        transaction_details: Some(TransactionDetails::Full),
                        show_rewards: Some(false),
                        max_supported_transaction_version: Some(0),
                    }),
                ),
            );
            builder = builder.datasource(block_subscribe);
        }
    }

    if service.config.start_slot > 0 {
        let latest_slot = RpcClient::new(indexer_rpc.to_string())
            .get_slot()
            .await
            .unwrap_or(service.config.start_slot as u64);
        let block_crawler = RpcBlockCrawler::new(
            indexer_rpc.to_string(),
            service.config.start_slot as u64,
            Some(latest_slot),
            None,
            RpcBlockConfig {
                commitment: Some(CommitmentConfig::confirmed()),
                encoding: Some(UiTransactionEncoding::Base64),
                transaction_details: Some(TransactionDetails::Full),
                rewards: Some(false),
                max_supported_transaction_version: Some(0),
            },
            None,
            None,
        );
        builder = builder.datasource(block_crawler);
    }

    info!(
        indexer_rpc = %indexer_rpc,
        stablecoin_program_id = %service.config.stablecoin_program_id,
        transfer_hook_program_id = %service.config.transfer_hook_program_id,
        start_slot = service.config.start_slot,
        disable_block_subscribe = service.config.disable_block_subscribe,
        "starting Carbon indexer pipeline"
    );

    let mut pipeline = builder.build()?;
    pipeline.run().await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
pub(crate) enum IndexedInstruction {
    Stablecoin { kind: String, data_base64: String },
    TransferHook,
}

impl InstructionDecoderCollection for IndexedInstruction {
    type InstructionType = String;

    fn parse_instruction(instruction: &Instruction) -> Option<DecodedInstruction<Self>> {
        if let Some(decoded) = StablecoinDecoder.decode_instruction(instruction) {
            return Some(DecodedInstruction {
                program_id: decoded.program_id,
                data: Self::Stablecoin {
                    kind: stablecoin_instruction_kind(&decoded.data).to_string(),
                    data_base64: base64::engine::general_purpose::STANDARD
                        .encode(&instruction.data),
                },
                accounts: decoded.accounts,
            });
        }

        if instruction.program_id == TRANSFER_HOOK_PROGRAM_ID {
            return Some(DecodedInstruction {
                program_id: instruction.program_id,
                data: Self::TransferHook,
                accounts: instruction.accounts.clone(),
            });
        }

        None
    }

    fn get_type(&self) -> Self::InstructionType {
        match self {
            Self::Stablecoin { kind, .. } => kind.clone(),
            Self::TransferHook => "TransferHook".to_string(),
        }
    }
}

fn stablecoin_instruction_kind(instruction: &StablecoinInstruction) -> &'static str {
    match instruction {
        StablecoinInstruction::AddToBlacklist(_) => "AddToBlacklist",
        StablecoinInstruction::Burn(_) => "Burn",
        StablecoinInstruction::FreezeAccount(_) => "FreezeAccount",
        StablecoinInstruction::Initialize(_) => "Initialize",
        StablecoinInstruction::Mint(_) => "Mint",
        StablecoinInstruction::Pause(_) => "Pause",
        StablecoinInstruction::RemoveFromBlacklist(_) => "RemoveFromBlacklist",
        StablecoinInstruction::Seize(_) => "Seize",
        StablecoinInstruction::ThawAccount(_) => "ThawAccount",
        StablecoinInstruction::TransferAuthority(_) => "TransferAuthority",
        StablecoinInstruction::Unpause(_) => "Unpause",
        StablecoinInstruction::UpdateMinter(_) => "UpdateMinter",
        StablecoinInstruction::UpdateRoles(_) => "UpdateRoles",
        StablecoinInstruction::CpiEvent(_) => "CpiEvent",
    }
}

struct CarbonTransactionProcessor {
    service: IndexerService,
}

#[async_trait]
impl Processor for CarbonTransactionProcessor {
    type InputType = TransactionProcessorInputType<IndexedInstruction, ()>;

    async fn process(
        &mut self,
        data: Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let (metadata, instructions, _) = data;
        let stablecoin_program_id = self.service.config.stablecoin_program_id.clone();
        let transfer_hook_program_id = self.service.config.transfer_hook_program_id.clone();

        let touches_relevant_program = instructions.iter().any(|(_, instruction)| {
            matches!(
                instruction.data,
                IndexedInstruction::Stablecoin { .. } | IndexedInstruction::TransferHook
            ) || instruction.program_id.to_string() == stablecoin_program_id
                || instruction.program_id.to_string() == transfer_hook_program_id
        });

        if !touches_relevant_program {
            return Ok(());
        }

        let logs = metadata.meta.log_messages.clone().unwrap_or_default();
        for (instruction_metadata, instruction) in instructions
            .iter()
            .filter(|(_, instruction)| matches!(&instruction.data, IndexedInstruction::Stablecoin { kind, .. } if kind == "CpiEvent"))
        {
            if let IndexedInstruction::Stablecoin { data_base64, .. } = &instruction.data {
                if let Some(event) = decode_stablecoin_cpi_event(
                    &stablecoin_program_id,
                    &metadata.signature.to_string(),
                    metadata.slot as i64,
                    metadata.block_time,
                    instruction_metadata.index as i32,
                    data_base64,
                ) {
                    self.service
                        .ingest_chain_event(&event)
                        .await
                        .map_err(|err| carbon_core::error::Error::Custom(err.to_string()))?;
                }
            }
        }

        for (_, instruction) in instructions
            .iter()
            .filter(|(_, instruction)| matches!(instruction.data, IndexedInstruction::TransferHook))
        {
            if let Some(event) = synthesize_transfer_hook_from_instruction(
                &metadata.signature.to_string(),
                metadata.slot as i64,
                metadata.block_time,
                &logs,
                &instruction.accounts,
                &transfer_hook_program_id,
            ) {
                self.service
                    .ingest_chain_event(&event)
                    .await
                    .map_err(|err| carbon_core::error::Error::Custom(err.to_string()))?;
            }
        }

        self.service
            .store
            .upsert_checkpoint(
                "stablecoin-main",
                &stablecoin_program_id,
                metadata.slot as i64,
                Some(&metadata.signature.to_string()),
            )
            .await
            .map_err(|err| carbon_core::error::Error::Custom(err.to_string()))?;

        Ok(())
    }
}

fn is_helius_rpc(rpc_url: &str) -> bool {
    rpc_url.contains("helius-rpc.com")
}

fn to_ws_url(rpc_url: &str) -> String {
    if let Some(value) = rpc_url.strip_prefix("https://") {
        return format!("wss://{value}");
    }
    if let Some(value) = rpc_url.strip_prefix("http://") {
        return format!("ws://{value}");
    }
    rpc_url.to_string()
}
