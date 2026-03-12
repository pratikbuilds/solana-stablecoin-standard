use solana_sdk::signature::Signer;
use spl_token_2022::{extension::StateWithExtensionsOwned, state::Account as TokenAccount};
use stablecoin::instructions::initialize::InitializeParams;

use sss_litesvm_tests::common::{
    add_to_blacklist_ix, create_token2022_ata_ix, funded_keypair, initialize_stablecoin, mint_ix,
    new_svm, remove_from_blacklist_ix, send_tx, token2022_ata, transfer_checked_with_hook_ix,
    update_minter_ix,
};

fn token_amount(svm: &litesvm::LiteSVM, address: &solana_sdk::pubkey::Pubkey) -> u64 {
    let account = svm
        .get_account(address)
        .expect("token account should exist");
    StateWithExtensionsOwned::<TokenAccount>::unpack(account.data)
        .expect("token account should unpack")
        .base
        .amount
}

fn setup_hook_transfer_fixture() -> (
    litesvm::LiteSVM,
    solana_sdk::signature::Keypair,
    solana_sdk::signature::Keypair,
    solana_sdk::signature::Keypair,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::pubkey::Pubkey,
    solana_sdk::pubkey::Pubkey,
) {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let sender = funded_keypair(&mut svm);
    let receiver = funded_keypair(&mut svm);
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
    let sender_ata = token2022_ata(&sender.pubkey(), &mint);
    let receiver_ata = token2022_ata(&receiver.pubkey(), &mint);

    assert!(send_tx(
        &mut svm,
        &authority,
        &[
            create_token2022_ata_ix(authority.pubkey(), sender.pubkey(), mint),
            create_token2022_ata_ix(authority.pubkey(), receiver.pubkey(), mint),
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
            1_000,
            true,
        )],
        &[&authority],
    )
    .is_ok());
    assert!(send_tx(
        &mut svm,
        &minter,
        &[mint_ix(minter.pubkey(), mint, sender_ata, 100)],
        &[&minter],
    )
    .is_ok());

    (
        svm,
        authority,
        sender,
        receiver,
        mint,
        sender_ata,
        receiver_ata,
    )
}

#[test]
fn transfer_hook_allows_unblacklisted_transfers_and_blocks_blacklisted_sender() {
    let (mut svm, authority, sender, receiver, mint, sender_ata, receiver_ata) =
        setup_hook_transfer_fixture();

    let allowed_transfer = send_tx(
        &mut svm,
        &sender,
        &[transfer_checked_with_hook_ix(
            sender_ata,
            mint,
            receiver_ata,
            sender.pubkey(),
            sender.pubkey(),
            receiver.pubkey(),
            25,
            6,
        )],
        &[&sender],
    );
    assert!(
        allowed_transfer.is_ok(),
        "unblacklisted transfer should succeed"
    );
    assert_eq!(token_amount(&svm, &sender_ata), 75);
    assert_eq!(token_amount(&svm, &receiver_ata), 25);

    assert!(send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            mint,
            sender.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    )
    .is_ok());

    let blocked_transfer = send_tx(
        &mut svm,
        &sender,
        &[transfer_checked_with_hook_ix(
            sender_ata,
            mint,
            receiver_ata,
            sender.pubkey(),
            sender.pubkey(),
            receiver.pubkey(),
            5,
            6,
        )],
        &[&sender],
    );
    assert!(
        blocked_transfer.is_err(),
        "blacklisted sender should be blocked"
    );
}

#[test]
fn transfer_hook_blocks_blacklisted_destination() {
    let (mut svm, authority, sender, receiver, mint, sender_ata, receiver_ata) =
        setup_hook_transfer_fixture();

    assert!(send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            mint,
            receiver.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    )
    .is_ok());

    let blocked_transfer = send_tx(
        &mut svm,
        &sender,
        &[transfer_checked_with_hook_ix(
            sender_ata,
            mint,
            receiver_ata,
            sender.pubkey(),
            sender.pubkey(),
            receiver.pubkey(),
            10,
            6,
        )],
        &[&sender],
    );
    assert!(
        blocked_transfer.is_err(),
        "blacklisted destination should be blocked"
    );
}

#[test]
fn transfer_hook_allows_transfers_again_after_blacklist_removal() {
    let (mut svm, authority, sender, receiver, mint, sender_ata, receiver_ata) =
        setup_hook_transfer_fixture();

    assert!(send_tx(
        &mut svm,
        &authority,
        &[add_to_blacklist_ix(
            authority.pubkey(),
            mint,
            sender.pubkey(),
            "sanctions".to_string(),
        )],
        &[&authority],
    )
    .is_ok());

    let blocked_transfer = send_tx(
        &mut svm,
        &sender,
        &[transfer_checked_with_hook_ix(
            sender_ata,
            mint,
            receiver_ata,
            sender.pubkey(),
            sender.pubkey(),
            receiver.pubkey(),
            10,
            6,
        )],
        &[&sender],
    );
    assert!(
        blocked_transfer.is_err(),
        "blacklisted sender should be blocked"
    );

    assert!(send_tx(
        &mut svm,
        &authority,
        &[remove_from_blacklist_ix(
            authority.pubkey(),
            mint,
            sender.pubkey(),
        )],
        &[&authority],
    )
    .is_ok());

    let restored_transfer = send_tx(
        &mut svm,
        &sender,
        &[transfer_checked_with_hook_ix(
            sender_ata,
            mint,
            receiver_ata,
            sender.pubkey(),
            sender.pubkey(),
            receiver.pubkey(),
            11,
            6,
        )],
        &[&sender],
    );
    assert!(
        restored_transfer.is_ok(),
        "transfer should succeed after blacklist removal: {restored_transfer:?}"
    );
    assert_eq!(token_amount(&svm, &sender_ata), 89);
    assert_eq!(token_amount(&svm, &receiver_ata), 11);
}
