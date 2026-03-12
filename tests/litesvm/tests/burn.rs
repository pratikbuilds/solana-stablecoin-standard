use solana_sdk::{pubkey::Pubkey, signature::Signer};
use spl_token_2022::{extension::StateWithExtensionsOwned, state::Account as TokenAccount};
use stablecoin::{
    instructions::{initialize::InitializeParams, roles::UpdateRolesParams},
    state::StablecoinConfig,
};

use sss_litesvm_tests::common::{
    burn_ix, create_token2022_ata_ix, deserialize_anchor_account, funded_keypair,
    initialize_stablecoin, mint_ix, new_svm, pause_ix, send_tx, token2022_ata, update_minter_ix,
    update_roles_ix,
};

fn token_amount(svm: &litesvm::LiteSVM, address: &Pubkey) -> u64 {
    let account = svm
        .get_account(address)
        .expect("token account should exist");
    StateWithExtensionsOwned::<TokenAccount>::unpack(account.data)
        .expect("token account should unpack")
        .base
        .amount
}

#[test]
fn burn_succeeds_for_authorized_burner_and_updates_counters() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let burner = funded_keypair(&mut svm);
    let mint_authority = funded_keypair(&mut svm);
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

    let burner_ata = token2022_ata(&burner.pubkey(), &mint);
    assert!(send_tx(
        &mut svm,
        &authority,
        &[create_token2022_ata_ix(
            authority.pubkey(),
            burner.pubkey(),
            mint,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[update_roles_ix(
            authority.pubkey(),
            mint,
            UpdateRolesParams {
                pauser: None,
                burner: Some(burner.pubkey()),
                blacklister: None,
                seizer: None,
            },
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            mint_authority.pubkey(),
            100,
            true,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &mint_authority,
        &[mint_ix(mint_authority.pubkey(), mint, burner_ata, 80)],
        &[&mint_authority],
    )
    .is_ok());

    let burn_result = send_tx(
        &mut svm,
        &burner,
        &[burn_ix(burner.pubkey(), mint, burner_ata, 30)],
        &[&burner],
    );
    assert!(burn_result.is_ok(), "authorized burn should succeed");

    let config: StablecoinConfig =
        deserialize_anchor_account(&svm, &sss_litesvm_tests::common::config_pda(&mint));
    assert_eq!(token_amount(&svm, &burner_ata), 50);
    assert_eq!(config.total_burned, 30);
}

#[test]
fn burn_rejects_unauthorized_or_paused_calls() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let burner = funded_keypair(&mut svm);
    let outsider = funded_keypair(&mut svm);
    let mint_authority = funded_keypair(&mut svm);
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

    let burner_ata = token2022_ata(&burner.pubkey(), &mint);
    assert!(send_tx(
        &mut svm,
        &authority,
        &[create_token2022_ata_ix(
            authority.pubkey(),
            burner.pubkey(),
            mint,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[update_roles_ix(
            authority.pubkey(),
            mint,
            UpdateRolesParams {
                pauser: None,
                burner: Some(burner.pubkey()),
                blacklister: None,
                seizer: None,
            },
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            mint_authority.pubkey(),
            100,
            true,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &mint_authority,
        &[mint_ix(mint_authority.pubkey(), mint, burner_ata, 40)],
        &[&mint_authority],
    )
    .is_ok());

    let unauthorized = send_tx(
        &mut svm,
        &outsider,
        &[burn_ix(outsider.pubkey(), mint, burner_ata, 10)],
        &[&outsider],
    );
    assert!(unauthorized.is_err(), "outsider burn should fail");

    assert!(send_tx(
        &mut svm,
        &authority,
        &[pause_ix(authority.pubkey(), mint)],
        &[&authority],
    )
    .is_ok());

    let paused_burn = send_tx(
        &mut svm,
        &burner,
        &[burn_ix(burner.pubkey(), mint, burner_ata, 10)],
        &[&burner],
    );
    assert!(paused_burn.is_err(), "paused burn should fail");
}
