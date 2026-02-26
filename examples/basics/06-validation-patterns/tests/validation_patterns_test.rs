//! Integration tests for Validation Patterns contract

use soroban_sdk::{Env, String, Vec};
use soroban_sdk::testutils::{Address as AddressTest, Ledger as LedgerTest};
use validation_patterns::{ValidationContract, ValidationError, UserRole, ContractState, DataKey};

#[test]
fn test_complete_validation_flow() {
    let env = Env::default();

    // 1. Initialize contract
    let owner = AddressTest::generate(&env);
    env.storage().instance().set(&DataKey::Owner, &owner);
    env.storage().instance().set(&DataKey::Admin, &owner);
    env.storage().instance().set(&DataKey::State, &ContractState::Active);

    // 2. Setup users with different roles
    let admin = AddressTest::generate(&env);
    let moderator = AddressTest::generate(&env);
    let user = AddressTest::generate(&env);

    env.storage().instance().set(&DataKey::UserRole(admin.clone()), &UserRole::Admin);
    env.storage().instance().set(&DataKey::UserRole(moderator.clone()), &UserRole::Moderator);
    env.storage().instance().set(&DataKey::UserRole(user.clone()), &UserRole::User);

    // 3. Set initial balances
    env.storage().persistent().set(&DataKey::Balance(user.clone()), &1000i128);

    // 4. Test successful transfer with full validation
    let result = ValidationContract::validated_transfer(
        env.clone(),
        &user,
        &moderator,
        100,
        Some(String::from_str(&env, "Valid transfer"))
    );
    assert!(result.is_ok());

    // 5. Verify state after transfer
    let balance1: i128 = env.storage().persistent().get(&DataKey::Balance(user.clone())).unwrap_or(0);
    let balance2: i128 = env.storage().persistent().get(&DataKey::Balance(moderator.clone())).unwrap_or(0);
    assert_eq!(balance1, 900);
    assert_eq!(balance2, 100);

    // 6. Test admin operations
    env.storage().instance().set(&DataKey::Admin, &admin);
    ValidationContract::pause_contract(env.clone(), &admin).unwrap();
    let state: ContractState = env.storage().instance().get(&DataKey::State).unwrap_or(ContractState::Uninitialized);
    assert_eq!(state, ContractState::Paused);

    // 7. Verify operations are blocked when paused
    let pause_result = ValidationContract::validated_transfer(env.clone(), &user, &moderator, 50, None);
    assert_eq!(pause_result, Err(ValidationError::ContractPaused));

    // 8. Resume and test again
    ValidationContract::resume_contract(env.clone(), &admin).unwrap();
    env.ledger().set_timestamp(env.ledger().timestamp() + 61); // Wait for cooldown
    
    let resume_result = ValidationContract::validated_transfer(env.clone(), &user, &moderator, 50, None);
    assert!(resume_result.is_ok());
}

#[test]
fn test_validation_error_hierarchy() {
    let env = Env::default();

    // Test parameter validation errors (100-199)
    assert_eq!(
        ValidationContract::validate_amount_parameters(0, 1, 1000),
        Err(ValidationError::InvalidAmount) // 100
    );
    assert_eq!(
        ValidationContract::validate_amount_parameters(-1, 1, 1000),
        Err(ValidationError::InvalidAmount) // 100
    );
    assert_eq!(
        ValidationContract::validate_amount_parameters(0, 1, 1000),
        Err(ValidationError::InvalidAmount) // 100
    );

    // Test state validation errors (200-299)
    assert_eq!(
        ValidationContract::validate_contract_state(&env, ContractState::Active),
        Err(ValidationError::ContractNotInitialized) // 200
    );

    let user = AddressTest::generate(&env);
    assert_eq!(
        ValidationContract::validate_balance(&env, user.clone(), 100),
        Err(ValidationError::InsufficientBalance) // 203
    );

    // Test authorization validation errors (300-399)
    assert_eq!(
        ValidationContract::validate_ownership(&env, user.clone()),
        Err(ValidationError::ContractNotInitialized) // Falls back to 200
    );
}

#[test]
fn test_edge_case_validations() {
    let env = Env::default();

    // Test boundary values for amount validation
    assert!(ValidationContract::validate_amount_parameters(1, 1, 1).is_ok()); // Exact min and max
    assert!(ValidationContract::validate_amount_parameters(u64::MAX as i128, 1, u64::MAX as i128).is_ok());

    // Test string boundary conditions
    let empty_string = String::from_str(&env, "");
    let single_char = String::from_str(&env, "a");
    
    assert_eq!(
        ValidationContract::validate_string_parameters(empty_string.clone(), 1, 100),
        Err(ValidationError::StringTooShort)
    );
    assert!(ValidationContract::validate_string_parameters(single_char.clone(), 1, 1).is_ok());

    // Test array boundary conditions
    let empty_array = Vec::from_array(&env, []);
    let single_element = Vec::from_array(&env, [1i32]);
    
    assert_eq!(
        ValidationContract::validate_array_parameters(empty_array.clone(), 1, 10),
        Err(ValidationError::ArrayTooSmall)
    );
    assert!(ValidationContract::validate_array_parameters(single_element.clone(), 1, 1).is_ok());
}

#[test]
fn test_role_hierarchy_and_permissions() {
    let env = Env::default();

    // Initialize
    let owner = AddressTest::generate(&env);
    env.storage().instance().set(&DataKey::Owner, &owner);
    env.storage().instance().set(&DataKey::State, &ContractState::Active);

    let admin = AddressTest::generate(&env);
    let moderator = AddressTest::generate(&env);
    let user = AddressTest::generate(&env);

    // Test role hierarchy
    let role: UserRole = env.storage().instance().get(&DataKey::UserRole(owner.clone())).unwrap_or(UserRole::None);
    assert_eq!(role, UserRole::None); // Owner has no explicit role set
    
    // Set roles in hierarchy
    env.storage().instance().set(&DataKey::UserRole(user.clone()), &UserRole::User);
    env.storage().instance().set(&DataKey::UserRole(moderator.clone()), &UserRole::Moderator);
    env.storage().instance().set(&DataKey::UserRole(admin.clone()), &UserRole::Admin);
    env.storage().instance().set(&DataKey::Admin, &admin);

    // Test role validation
    assert!(ValidationContract::validate_role(&env, user.clone(), UserRole::User).is_ok());
    assert!(ValidationContract::validate_role(&env, user.clone(), UserRole::None).is_ok());
    assert_eq!(
        ValidationContract::validate_role(&env, user.clone(), UserRole::Moderator),
        Err(ValidationError::InsufficientRole)
    );

    assert!(ValidationContract::validate_role(&env, moderator.clone(), UserRole::User).is_ok());
    assert!(ValidationContract::validate_role(&env, moderator.clone(), UserRole::Moderator).is_ok());
    assert_eq!(
        ValidationContract::validate_role(&env, moderator.clone(), UserRole::Admin),
        Err(ValidationError::InsufficientRole)
    );

    assert!(ValidationContract::validate_role(&env, admin.clone(), UserRole::User).is_ok());
    assert!(ValidationContract::validate_role(&env, admin.clone(), UserRole::Moderator).is_ok());
    assert!(ValidationContract::validate_role(&env, admin.clone(), UserRole::Admin).is_ok());
}

#[test]
fn test_cooldown_and_rate_limiting() {
    let env = Env::default();

    // Initialize
    let owner = AddressTest::generate(&env);
    env.storage().instance().set(&DataKey::Owner, &owner);
    env.storage().instance().set(&DataKey::Admin, &owner);
    env.storage().instance().set(&DataKey::State, &ContractState::Active);

    let user = AddressTest::generate(&env);
    env.storage().instance().set(&DataKey::UserRole(user.clone()), &UserRole::User);

    // Set initial balance
    env.storage().persistent().set(&DataKey::Balance(user.clone()), &1000i128);

    // First transfer should succeed
    assert!(ValidationContract::validated_transfer(env.clone(), &user, &owner, 100, None).is_ok());

    // Immediate second transfer should fail due to cooldown
    assert_eq!(
        ValidationContract::validated_transfer(env.clone(), &user, &owner, 100, None),
        Err(ValidationError::CooldownActive)
    );

    // Advance time past cooldown and try again
    env.ledger().set_timestamp(env.ledger().timestamp() + 61);
    assert!(ValidationContract::validated_transfer(env.clone(), &user, &owner, 100, None).is_ok());
}

#[test]
fn test_blacklist_functionality() {
    let env = Env::default();

    // Initialize
    let owner = AddressTest::generate(&env);
    env.storage().instance().set(&DataKey::Owner, &owner);
    env.storage().instance().set(&DataKey::Admin, &owner);
    env.storage().instance().set(&DataKey::State, &ContractState::Active);

    let user = AddressTest::generate(&env);
    env.storage().instance().set(&DataKey::UserRole(user.clone()), &UserRole::User);

    // User should be able to validate role normally
    assert!(ValidationContract::validate_role(&env, user.clone(), UserRole::User).is_ok());

    // Add user to blacklist
    env.storage().instance().set(&DataKey::Blacklist(user.clone()), &true);

    // Now user should be blocked
    assert_eq!(
        ValidationContract::validate_role(&env, user.clone(), UserRole::User),
        Err(ValidationError::Blacklisted)
    );
}

#[test]
fn test_contract_state_transitions() {
    let env = Env::default();

    // Initialize
    let owner = AddressTest::generate(&env);
    env.storage().instance().set(&DataKey::Owner, &owner);
    env.storage().instance().set(&DataKey::Admin, &owner);
    env.storage().instance().set(&DataKey::State, &ContractState::Active);

    // Initial state should be Active
    let state: ContractState = env.storage().instance().get(&DataKey::State).unwrap_or(ContractState::Uninitialized);
    assert_eq!(state, ContractState::Active);

    // Pause contract
    ValidationContract::pause_contract(env.clone(), &owner).unwrap();
    let state: ContractState = env.storage().instance().get(&DataKey::State).unwrap_or(ContractState::Uninitialized);
    assert_eq!(state, ContractState::Paused);

    // Resume contract
    ValidationContract::resume_contract(env.clone(), &owner).unwrap();
    let state: ContractState = env.storage().instance().get(&DataKey::State).unwrap_or(ContractState::Uninitialized);
    assert_eq!(state, ContractState::Active);

    // Test state validation during different states
    ValidationContract::pause_contract(env.clone(), &owner).unwrap();
    assert_eq!(
        ValidationContract::validate_contract_state(&env, ContractState::Active),
        Err(ValidationError::ContractPaused)
    );

    ValidationContract::resume_contract(env.clone(), &owner).unwrap();
    assert!(ValidationContract::validate_contract_state(&env, ContractState::Active).is_ok());
}
