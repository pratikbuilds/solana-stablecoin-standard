use trident_fuzz::fuzzing::*;

/// Storage for all account addresses used in fuzz testing.
///
/// This struct serves as a centralized repository for account addresses,
/// enabling their reuse across different instruction flows and test scenarios.
///
/// Docs: https://ackee.xyz/trident/docs/latest/trident-api-macro/trident-types/fuzz-accounts/
#[allow(dead_code)]
#[derive(Default)]
pub struct AccountAddresses {
    pub authority: AddressStorage,

    pub config: AddressStorage,

    pub role_config: AddressStorage,

    pub wallet: AddressStorage,

    pub blacklist_entry: AddressStorage,

    pub system_program: AddressStorage,

    pub mint: AddressStorage,

    pub from: AddressStorage,

    pub token_program: AddressStorage,

    pub account: AddressStorage,

    pub extra_account_meta_list: AddressStorage,

    pub transfer_hook_program: AddressStorage,

    pub rent: AddressStorage,

    pub minter_quota: AddressStorage,

    pub to: AddressStorage,

    pub stablecoin_program: AddressStorage,

    pub destination_blacklist: AddressStorage,

    pub minter: AddressStorage,

    pub payer: AddressStorage,

    pub source: AddressStorage,

    pub destination: AddressStorage,

    pub source_blacklist: AddressStorage,
}
