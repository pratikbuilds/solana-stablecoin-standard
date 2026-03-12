use solana_sdk::signature::Signer;
use spl_token_2022::{extension::StateWithExtensionsOwned, state::Account as TokenAccount};
use stablecoin::{
    instructions::initialize::InitializeParams,
    state::{MinterQuota, StablecoinConfig},
};

use sss_litesvm_tests::common::{
    create_token2022_ata_ix, deserialize_anchor_account, funded_keypair, initialize_stablecoin,
    mint_ix, minter_quota_pda, new_svm, pause_ix, send_tx, token2022_ata, update_minter_ix,
};

fn read_token_account(
    svm: &litesvm::LiteSVM,
    address: &solana_sdk::pubkey::Pubkey,
) -> TokenAccount {
    let account = svm
        .get_account(address)
        .expect("token account should exist");
    StateWithExtensionsOwned::<TokenAccount>::unpack(account.data)
        .expect("token account should unpack")
        .base
}

#[test]
fn mint_respects_quota_and_records_supply() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let minter = funded_keypair(&mut svm);
    let recipient = funded_keypair(&mut svm);
    let mint = initialize_stablecoin(
        &mut svm,
        &authority,
        InitializeParams {
            name: "Simple USD".to_string(),
            symbol: "SUSD".to_string(),
            uri: "https://example.com/simple.json".to_string(),
            decimals: 6,
            enable_permanent_delegate: false,
            enable_transfer_hook: false,
            default_account_frozen: false,
        },
    );

    let recipient_ata = token2022_ata(&recipient.pubkey(), &mint);
    let create_ata_result = send_tx(
        &mut svm,
        &authority,
        &[create_token2022_ata_ix(
            authority.pubkey(),
            recipient.pubkey(),
            mint,
        )],
        &[&authority],
    );
    assert!(create_ata_result.is_ok(), "ATA creation should succeed");

    let minter_update_result = send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            minter.pubkey(),
            100,
            true,
        )],
        &[&authority],
    );
    assert!(minter_update_result.is_ok(), "minter update should succeed");

    let mint_result = send_tx(
        &mut svm,
        &minter,
        &[mint_ix(minter.pubkey(), mint, recipient_ata, 60)],
        &[&minter],
    );
    assert!(mint_result.is_ok(), "mint should succeed");

    let destination = read_token_account(&svm, &recipient_ata);
    let quota: MinterQuota =
        deserialize_anchor_account(&svm, &minter_quota_pda(&mint, &minter.pubkey()));
    let config: StablecoinConfig =
        deserialize_anchor_account(&svm, &sss_litesvm_tests::common::config_pda(&mint));

    assert_eq!(destination.amount, 60);
    assert_eq!(quota.minted, 60);
    assert_eq!(config.total_minted, 60);
}

#[test]
fn mint_rejects_quota_exceeded_unauthorized_and_paused_paths() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let minter = funded_keypair(&mut svm);
    let outsider = funded_keypair(&mut svm);
    let recipient = funded_keypair(&mut svm);
    let mint = initialize_stablecoin(
        &mut svm,
        &authority,
        InitializeParams {
            name: "Simple USD".to_string(),
            symbol: "SUSD".to_string(),
            uri: "https://example.com/simple.json".to_string(),
            decimals: 6,
            enable_permanent_delegate: false,
            enable_transfer_hook: false,
            default_account_frozen: false,
        },
    );

    let recipient_ata = token2022_ata(&recipient.pubkey(), &mint);
    let create_ata_result = send_tx(
        &mut svm,
        &authority,
        &[create_token2022_ata_ix(
            authority.pubkey(),
            recipient.pubkey(),
            mint,
        )],
        &[&authority],
    );
    assert!(create_ata_result.is_ok(), "ATA creation should succeed");

    let minter_update_result = send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            minter.pubkey(),
            100,
            true,
        )],
        &[&authority],
    );
    assert!(minter_update_result.is_ok(), "minter update should succeed");

    let first_mint = send_tx(
        &mut svm,
        &minter,
        &[mint_ix(minter.pubkey(), mint, recipient_ata, 60)],
        &[&minter],
    );
    assert!(first_mint.is_ok(), "initial mint should succeed");

    let quota_exceeded = send_tx(
        &mut svm,
        &minter,
        &[mint_ix(minter.pubkey(), mint, recipient_ata, 50)],
        &[&minter],
    );
    assert!(quota_exceeded.is_err(), "quota exceeded mint should fail");

    let unauthorized = send_tx(
        &mut svm,
        &outsider,
        &[mint_ix(outsider.pubkey(), mint, recipient_ata, 1)],
        &[&outsider],
    );
    assert!(unauthorized.is_err(), "unauthorized minter should fail");

    let pause_result = send_tx(
        &mut svm,
        &authority,
        &[pause_ix(authority.pubkey(), mint)],
        &[&authority],
    );
    assert!(pause_result.is_ok(), "pause should succeed");

    let paused_mint = send_tx(
        &mut svm,
        &minter,
        &[mint_ix(minter.pubkey(), mint, recipient_ata, 1)],
        &[&minter],
    );
    assert!(paused_mint.is_err(), "paused mint should fail");

    let config: StablecoinConfig =
        deserialize_anchor_account(&svm, &sss_litesvm_tests::common::config_pda(&mint));
    assert!(config.paused);
}
