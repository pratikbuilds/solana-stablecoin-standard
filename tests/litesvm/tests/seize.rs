use solana_sdk::signature::Signer;
use spl_token_2022::{
    extension::StateWithExtensionsOwned,
    state::{Account as TokenAccount, AccountState},
};
use stablecoin::instructions::initialize::InitializeParams;

use sss_litesvm_tests::common::{
    add_to_blacklist_ix, create_token2022_ata_ix, freeze_account_ix, funded_keypair,
    initialize_stablecoin, mint_ix, new_svm, pause_ix, seize_ix_with_amount, send_tx,
    token2022_ata, update_minter_ix,
};

fn token_account(svm: &litesvm::LiteSVM, address: &solana_sdk::pubkey::Pubkey) -> TokenAccount {
    let account = svm
        .get_account(address)
        .expect("token account should exist");
    StateWithExtensionsOwned::<TokenAccount>::unpack(account.data)
        .expect("token account should unpack")
        .base
}

#[test]
fn seize_moves_funds_from_frozen_blacklisted_account_to_treasury() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let victim = funded_keypair(&mut svm);
    let minter = funded_keypair(&mut svm);
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

    let victim_ata = token2022_ata(&victim.pubkey(), &mint);
    let treasury_ata = token2022_ata(&authority.pubkey(), &mint);
    assert!(send_tx(
        &mut svm,
        &authority,
        &[
            create_token2022_ata_ix(authority.pubkey(), victim.pubkey(), mint),
            create_token2022_ata_ix(authority.pubkey(), authority.pubkey(), mint),
        ],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            minter.pubkey(),
            500,
            true,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &minter,
        &[mint_ix(minter.pubkey(), mint, victim_ata, 100)],
        &[&minter],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            mint,
            victim.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[freeze_account_ix(authority.pubkey(), mint, victim_ata)],
        &[&authority],
    )
    .is_ok());

    let seize_result = send_tx(
        &mut svm,
        &authority,
        &[seize_ix_with_amount(
            authority.pubkey(),
            mint,
            victim_ata,
            treasury_ata,
            victim.pubkey(),
            authority.pubkey(),
            40,
        )],
        &[&authority],
    );
    assert!(
        seize_result.is_ok(),
        "authorized seizure should succeed: {seize_result:?}"
    );

    let victim_account = token_account(&svm, &victim_ata);
    let treasury_account = token_account(&svm, &treasury_ata);
    assert_eq!(victim_account.amount, 60);
    assert_eq!(treasury_account.amount, 40);
    assert_eq!(victim_account.state, AccountState::Frozen);
}

#[test]
fn seize_rejects_unfrozen_or_unauthorized_requests() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let outsider = funded_keypair(&mut svm);
    let victim = funded_keypair(&mut svm);
    let minter = funded_keypair(&mut svm);
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

    let victim_ata = token2022_ata(&victim.pubkey(), &mint);
    let treasury_ata = token2022_ata(&authority.pubkey(), &mint);
    assert!(send_tx(
        &mut svm,
        &authority,
        &[
            create_token2022_ata_ix(authority.pubkey(), victim.pubkey(), mint),
            create_token2022_ata_ix(authority.pubkey(), authority.pubkey(), mint),
        ],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            minter.pubkey(),
            500,
            true,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &minter,
        &[mint_ix(minter.pubkey(), mint, victim_ata, 50)],
        &[&minter],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            mint,
            victim.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    )
    .is_ok());

    let not_frozen = send_tx(
        &mut svm,
        &authority,
        &[seize_ix_with_amount(
            authority.pubkey(),
            mint,
            victim_ata,
            treasury_ata,
            victim.pubkey(),
            authority.pubkey(),
            10,
        )],
        &[&authority],
    );
    assert!(not_frozen.is_err(), "unfrozen seize should fail");

    assert!(send_tx(
        &mut svm,
        &authority,
        &[freeze_account_ix(authority.pubkey(), mint, victim_ata)],
        &[&authority],
    )
    .is_ok());

    let unauthorized = send_tx(
        &mut svm,
        &outsider,
        &[seize_ix_with_amount(
            outsider.pubkey(),
            mint,
            victim_ata,
            treasury_ata,
            victim.pubkey(),
            authority.pubkey(),
            10,
        )],
        &[&outsider],
    );
    assert!(unauthorized.is_err(), "unauthorized seizer should fail");
}

#[test]
fn seize_is_blocked_while_paused() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let victim = funded_keypair(&mut svm);
    let minter = funded_keypair(&mut svm);
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

    let victim_ata = token2022_ata(&victim.pubkey(), &mint);
    let treasury_ata = token2022_ata(&authority.pubkey(), &mint);
    assert!(send_tx(
        &mut svm,
        &authority,
        &[
            create_token2022_ata_ix(authority.pubkey(), victim.pubkey(), mint),
            create_token2022_ata_ix(authority.pubkey(), authority.pubkey(), mint),
        ],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[update_minter_ix(
            authority.pubkey(),
            mint,
            minter.pubkey(),
            500,
            true,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &minter,
        &[mint_ix(minter.pubkey(), mint, victim_ata, 50)],
        &[&minter],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &authority,
        &[
            add_to_blacklist_ix(
                authority.pubkey(),
                mint,
                victim.pubkey(),
                "sanctions".to_string(),
            ),
            freeze_account_ix(authority.pubkey(), mint, victim_ata),
            pause_ix(authority.pubkey(), mint),
        ],
        &[&authority],
    )
    .is_ok());

    let paused = send_tx(
        &mut svm,
        &authority,
        &[seize_ix_with_amount(
            authority.pubkey(),
            mint,
            victim_ata,
            treasury_ata,
            victim.pubkey(),
            authority.pubkey(),
            10,
        )],
        &[&authority],
    );
    assert!(paused.is_err(), "paused seize should fail");
}
