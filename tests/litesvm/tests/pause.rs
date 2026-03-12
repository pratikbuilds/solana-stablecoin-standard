use solana_sdk::signature::Signer;
use stablecoin::{instructions::initialize::InitializeParams, state::StablecoinConfig};

use sss_litesvm_tests::common::{
    create_token2022_ata_ix, deserialize_anchor_account, freeze_account_ix, funded_keypair,
    initialize_stablecoin, new_svm, pause_ix, send_tx, thaw_account_ix, token2022_ata, unpause_ix,
};

#[test]
fn pause_and_unpause_toggle_pause_state() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
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

    assert!(send_tx(
        &mut svm,
        &authority,
        &[pause_ix(authority.pubkey(), mint)],
        &[&authority],
    )
    .is_ok());
    let paused: StablecoinConfig =
        deserialize_anchor_account(&svm, &sss_litesvm_tests::common::config_pda(&mint));
    assert!(paused.paused);

    assert!(send_tx(
        &mut svm,
        &authority,
        &[unpause_ix(authority.pubkey(), mint)],
        &[&authority],
    )
    .is_ok());
    let unpaused: StablecoinConfig =
        deserialize_anchor_account(&svm, &sss_litesvm_tests::common::config_pda(&mint));
    assert!(!unpaused.paused);
}

#[test]
fn freeze_and_thaw_remain_available_while_paused() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let user = funded_keypair(&mut svm);
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

    let user_ata = token2022_ata(&user.pubkey(), &mint);
    assert!(send_tx(
        &mut svm,
        &authority,
        &[create_token2022_ata_ix(
            authority.pubkey(),
            user.pubkey(),
            mint,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[pause_ix(authority.pubkey(), mint)],
        &[&authority],
    )
    .is_ok());

    assert!(send_tx(
        &mut svm,
        &authority,
        &[freeze_account_ix(authority.pubkey(), mint, user_ata)],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[thaw_account_ix(authority.pubkey(), mint, user_ata)],
        &[&authority],
    )
    .is_ok());
}
