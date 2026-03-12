use solana_sdk::signature::Signer;
use stablecoin::{instructions::initialize::InitializeParams, state::MinterQuota};

use sss_litesvm_tests::common::{
    deserialize_anchor_account, funded_keypair, initialize_stablecoin, minter_quota_pda, new_svm,
    send_tx, update_minter_ix,
};

#[test]
fn update_minter_creates_updates_and_disables_quota() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let minter = funded_keypair(&mut svm);
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

    let create_result = send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            minter.pubkey(),
            1_000,
            true,
        )],
        &[&authority],
    );
    assert!(create_result.is_ok(), "minter create should succeed");

    let quota_pda = minter_quota_pda(&mint, &minter.pubkey());
    let quota: MinterQuota = deserialize_anchor_account(&svm, &quota_pda);
    assert_eq!(quota.mint, mint);
    assert_eq!(quota.minter, minter.pubkey());
    assert_eq!(quota.quota, 1_000);
    assert_eq!(quota.minted, 0);
    assert!(quota.active);

    let disable_result = send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            minter.pubkey(),
            2_500,
            false,
        )],
        &[&authority],
    );
    assert!(disable_result.is_ok(), "minter update should succeed");

    let updated_quota: MinterQuota = deserialize_anchor_account(&svm, &quota_pda);
    assert_eq!(updated_quota.quota, 2_500);
    assert!(!updated_quota.active);
}
