use solana_sdk::signature::Signer;
use solana_sdk::system_program;
use stablecoin::{instructions::initialize::InitializeParams, state::BlacklistEntry};

use sss_litesvm_tests::common::{
    add_to_blacklist_ix, blacklist_pda, deserialize_anchor_account, funded_keypair,
    initialize_stablecoin, new_svm, remove_from_blacklist_ix, send_tx,
};

#[test]
fn blacklist_operations_are_sss2_only_and_duplicate_adds_fail() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let wallet = funded_keypair(&mut svm);

    let sss1_mint = initialize_stablecoin(
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
    let sss1_result = send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            sss1_mint,
            wallet.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    );
    assert!(
        sss1_result.is_err(),
        "SSS-1 should reject blacklist mutations"
    );

    let sss2_mint = initialize_stablecoin(
        &mut svm,
        &authority,
        InitializeParams {
            name: "Regulated USD".to_string(),
            symbol: "RUSD".to_string(),
            uri: "https://example.com/regulated.json".to_string(),
            decimals: 6,
            enable_permanent_delegate: true,
            enable_transfer_hook: true,
            default_account_frozen: false,
        },
    );
    let add_result = send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            sss2_mint,
            wallet.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    );
    assert!(add_result.is_ok(), "SSS-2 blacklist add should succeed");

    let entry: BlacklistEntry =
        deserialize_anchor_account(&svm, &blacklist_pda(&sss2_mint, &wallet.pubkey()));
    assert_eq!(entry.wallet, wallet.pubkey());
    assert_eq!(entry.reason, "sanctions");

    let duplicate_add = send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            sss2_mint,
            wallet.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    );
    assert!(
        duplicate_add.is_err(),
        "duplicate blacklist add should fail"
    );
}

#[test]
fn remove_from_blacklist_closes_the_wallet_pda() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let wallet = funded_keypair(&mut svm);
    let mint = initialize_stablecoin(
        &mut svm,
        &authority,
        InitializeParams {
            name: "Regulated USD".to_string(),
            symbol: "RUSD".to_string(),
            uri: "https://example.com/regulated.json".to_string(),
            decimals: 6,
            enable_permanent_delegate: true,
            enable_transfer_hook: true,
            default_account_frozen: false,
        },
    );

    assert!(send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            mint,
            wallet.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    )
    .is_ok());
    assert!(svm
        .get_account(&blacklist_pda(&mint, &wallet.pubkey()))
        .is_some());

    let remove_result = send_tx(
        &mut svm,
        &authority,
        &[remove_from_blacklist_ix(
            authority.pubkey(),
            mint,
            wallet.pubkey(),
        )],
        &[&authority],
    );
    assert!(remove_result.is_ok(), "blacklist removal should succeed");
    match svm.get_account(&blacklist_pda(&mint, &wallet.pubkey())) {
        None => {}
        Some(account) => {
            assert_eq!(account.owner, system_program::ID);
            assert_eq!(account.lamports, 0);
            assert!(account.data.is_empty());
        }
    }
}
