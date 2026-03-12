use solana_sdk::{pubkey::Pubkey, signature::Signer};
use spl_token_2022::{
    extension::StateWithExtensionsOwned,
    state::{Account as TokenAccount, AccountState},
};
use stablecoin::instructions::initialize::InitializeParams;

use sss_litesvm_tests::common::{
    create_token2022_ata_ix, freeze_account_ix, funded_keypair, initialize_stablecoin, new_svm,
    send_tx, thaw_account_ix, token2022_ata,
};

fn token_state(svm: &litesvm::LiteSVM, address: &Pubkey) -> AccountState {
    let account = svm
        .get_account(address)
        .expect("token account should exist");
    StateWithExtensionsOwned::<TokenAccount>::unpack(account.data)
        .expect("token account should unpack")
        .base
        .state
}

#[test]
fn freeze_and_thaw_change_token_account_state() {
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
    assert_eq!(token_state(&svm, &user_ata), AccountState::Initialized);

    assert!(send_tx(
        &mut svm,
        &authority,
        &[freeze_account_ix(authority.pubkey(), mint, user_ata)],
        &[&authority],
    )
    .is_ok());
    assert_eq!(token_state(&svm, &user_ata), AccountState::Frozen);

    assert!(send_tx(
        &mut svm,
        &authority,
        &[thaw_account_ix(authority.pubkey(), mint, user_ata)],
        &[&authority],
    )
    .is_ok());
    assert_eq!(token_state(&svm, &user_ata), AccountState::Initialized);
}

#[test]
fn freeze_rejects_unauthorized_caller() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let outsider = funded_keypair(&mut svm);
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

    let result = send_tx(
        &mut svm,
        &outsider,
        &[freeze_account_ix(outsider.pubkey(), mint, user_ata)],
        &[&outsider],
    );
    assert!(result.is_err(), "outsider freeze should fail");
    assert_eq!(token_state(&svm, &user_ata), AccountState::Initialized);
}
