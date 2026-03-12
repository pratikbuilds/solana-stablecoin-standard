use solana_sdk::{pubkey::Pubkey, signature::Signer};
use stablecoin::{
    instructions::{initialize::InitializeParams, roles::UpdateRolesParams},
    state::{RoleConfig, StablecoinConfig},
};

use sss_litesvm_tests::common::{
    config_pda, deserialize_anchor_account, funded_keypair, initialize_stablecoin, new_svm,
    roles_pda, send_tx, transfer_authority_ix, update_roles_ix,
};

#[test]
fn update_roles_requires_master_authority() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let outsider = funded_keypair(&mut svm);
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

    let result = send_tx(
        &mut svm,
        &outsider,
        &[update_roles_ix(
            outsider.pubkey(),
            mint,
            UpdateRolesParams {
                pauser: Some(Pubkey::new_unique()),
                burner: None,
                blacklister: None,
                seizer: None,
            },
        )],
        &[&outsider],
    );

    assert!(result.is_err(), "non-master role update should fail");

    let roles: RoleConfig = deserialize_anchor_account(&svm, &roles_pda(&mint));
    assert_eq!(roles.master_authority, authority.pubkey());
    assert_eq!(roles.pauser, authority.pubkey());
}

#[test]
fn update_roles_rejects_compliance_roles_when_sss2_features_are_disabled() {
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

    let result = send_tx(
        &mut svm,
        &authority,
        &[update_roles_ix(
            authority.pubkey(),
            mint,
            UpdateRolesParams {
                pauser: None,
                burner: None,
                blacklister: Some(Pubkey::new_unique()),
                seizer: Some(Pubkey::new_unique()),
            },
        )],
        &[&authority],
    );

    assert!(
        result.is_err(),
        "SSS-1 should reject compliance role updates"
    );

    let roles: RoleConfig = deserialize_anchor_account(&svm, &roles_pda(&mint));
    assert_eq!(roles.blacklister, Pubkey::default());
    assert_eq!(roles.seizer, Pubkey::default());
}

#[test]
fn transfer_authority_hands_off_master_role_immediately() {
    let mut svm = new_svm();
    let authority = funded_keypair(&mut svm);
    let new_authority = funded_keypair(&mut svm);
    let new_pauser = Pubkey::new_unique();
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

    let transfer_result = send_tx(
        &mut svm,
        &authority,
        &[transfer_authority_ix(
            authority.pubkey(),
            mint,
            new_authority.pubkey(),
        )],
        &[&authority],
    );
    assert!(transfer_result.is_ok(), "authority transfer should succeed");

    let config: StablecoinConfig = deserialize_anchor_account(&svm, &config_pda(&mint));
    let roles: RoleConfig = deserialize_anchor_account(&svm, &roles_pda(&mint));
    assert_eq!(config.authority, new_authority.pubkey());
    assert_eq!(roles.master_authority, new_authority.pubkey());

    let old_authority_result = send_tx(
        &mut svm,
        &authority,
        &[update_roles_ix(
            authority.pubkey(),
            mint,
            UpdateRolesParams {
                pauser: Some(Pubkey::new_unique()),
                burner: None,
                blacklister: None,
                seizer: None,
            },
        )],
        &[&authority],
    );
    assert!(
        old_authority_result.is_err(),
        "previous master should lose admin privileges"
    );

    let new_authority_result = send_tx(
        &mut svm,
        &new_authority,
        &[update_roles_ix(
            new_authority.pubkey(),
            mint,
            UpdateRolesParams {
                pauser: Some(new_pauser),
                burner: None,
                blacklister: None,
                seizer: None,
            },
        )],
        &[&new_authority],
    );
    assert!(
        new_authority_result.is_ok(),
        "new master should gain admin privileges immediately"
    );

    let updated_roles: RoleConfig = deserialize_anchor_account(&svm, &roles_pda(&mint));
    assert_eq!(updated_roles.pauser, new_pauser);
}
