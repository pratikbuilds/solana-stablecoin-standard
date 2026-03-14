use base64::Engine;
use carbon_stablecoin_decoder::instructions::cpi_event::CpiEvent;
use chrono::Utc;
use serde_json::Value;
use solana_sdk::instruction::AccountMeta;
use sss_domain::InsertEvent;

pub fn synthesize_transfer_hook_event(
    mint: String,
    tx_signature: String,
    slot: i64,
    event_type: &str,
    payload: Value,
) -> InsertEvent {
    InsertEvent {
        event_type: event_type.to_string(),
        program_id: Some("transfer-hook".to_string()),
        mint: Some(mint),
        tx_signature,
        slot,
        block_time: Some(Utc::now()),
        instruction_index: 0,
        data: payload,
    }
}

pub(crate) fn decode_stablecoin_cpi_event(
    stablecoin_program_id: &str,
    tx_signature: &str,
    slot: i64,
    block_time: Option<i64>,
    instruction_index: i32,
    data_base64: &str,
) -> Option<InsertEvent> {
    let bytes = base64::engine::general_purpose::STANDARD.decode(data_base64).ok()?;
    let (event_type, mint, payload) = decode_stablecoin_event_payload(&bytes)?;

    Some(InsertEvent {
        event_type: event_type.to_string(),
        program_id: Some(stablecoin_program_id.to_string()),
        mint: Some(mint),
        tx_signature: tx_signature.to_string(),
        slot,
        block_time: block_time.and_then(timestamp_to_datetime),
        instruction_index,
        data: payload,
    })
}

pub(crate) fn synthesize_transfer_hook_from_instruction(
    tx_signature: &str,
    slot: i64,
    block_time: Option<i64>,
    logs: &[String],
    accounts: &[AccountMeta],
    transfer_hook_program_id: &str,
) -> Option<InsertEvent> {
    let mint = accounts.get(1)?.pubkey.to_string();
    let source = accounts.first().map(|account| account.pubkey.to_string());
    let destination = accounts.get(2).map(|account| account.pubkey.to_string());
    let authority = accounts.get(3).map(|account| account.pubkey.to_string());

    let event_type = if logs.iter().any(|log| log.contains("Source address is blacklisted")) {
        "transfer_rejected_source_blacklisted"
    } else if logs.iter().any(|log| log.contains("Destination address is blacklisted")) {
        "transfer_rejected_destination_blacklisted"
    } else {
        "transfer_checked"
    };

    Some(InsertEvent {
        event_type: event_type.to_string(),
        program_id: Some(transfer_hook_program_id.to_string()),
        mint: Some(mint.clone()),
        tx_signature: tx_signature.to_string(),
        slot,
        block_time: block_time.and_then(timestamp_to_datetime),
        instruction_index: 0,
        data: serde_json::json!({
            "mint": mint,
            "source_token_account": source,
            "destination_token_account": destination,
            "authority": authority,
        }),
    })
}

fn decode_stablecoin_event_payload(bytes: &[u8]) -> Option<(&'static str, String, Value)> {
    let cpi_event = CpiEvent::decode(bytes)?;

    match cpi_event {
        CpiEvent::StablecoinInitialized(value) => Some((
            "StablecoinInitialized",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "authority": value.authority.to_string(),
                "preset": value.preset,
            }),
        )),
        CpiEvent::MinterUpdated(value) => Some((
            "MinterUpdated",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "minter": value.minter.to_string(),
                "quota": value.quota.to_string(),
                "active": value.active,
            }),
        )),
        CpiEvent::RolesUpdated(value) => Some((
            "RolesUpdated",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "master_authority": value.authority.to_string(),
            }),
        )),
        CpiEvent::AuthorityTransferred(value) => Some((
            "AuthorityTransferred",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "master_authority": value.new_authority.to_string(),
                "authority": value.new_authority.to_string(),
            }),
        )),
        CpiEvent::TokensMinted(value) => Some((
            "TokensMinted",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "to": value.to.to_string(),
                "authority": value.authority.to_string(),
                "amount": value.amount.to_string(),
            }),
        )),
        CpiEvent::TokensBurned(value) => Some((
            "TokensBurned",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "from": value.from.to_string(),
                "authority": value.authority.to_string(),
                "amount": value.amount.to_string(),
            }),
        )),
        CpiEvent::PauseChanged(value) => Some((
            "PauseChanged",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "paused": value.paused,
                "authority": value.authority.to_string(),
            }),
        )),
        CpiEvent::AddressBlacklisted(value) => Some((
            "AddressBlacklisted",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "wallet": value.wallet.to_string(),
                "authority": value.authority.to_string(),
                "reason": value.reason,
            }),
        )),
        CpiEvent::AddressUnblacklisted(value) => Some((
            "AddressUnblacklisted",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "wallet": value.wallet.to_string(),
                "authority": value.authority.to_string(),
            }),
        )),
        CpiEvent::AccountFrozen(value) => Some((
            "AccountFrozen",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "account": value.account.to_string(),
                "authority": value.authority.to_string(),
            }),
        )),
        CpiEvent::AccountThawed(value) => Some((
            "AccountThawed",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "account": value.account.to_string(),
                "authority": value.authority.to_string(),
            }),
        )),
        CpiEvent::TokensSeized(value) => Some((
            "TokensSeized",
            value.mint.to_string(),
            serde_json::json!({
                "mint": value.mint.to_string(),
                "from": value.from.to_string(),
                "to": value.to.to_string(),
                "authority": value.authority.to_string(),
                "amount": value.amount.to_string(),
            }),
        )),
    }
}

fn timestamp_to_datetime(value: i64) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::<Utc>::from_timestamp(value, 0)
}
