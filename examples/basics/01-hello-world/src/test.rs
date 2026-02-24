//! Unit tests for the Hello World contract

#![cfg(test)]

use super::*;
use soroban_sdk::{symbol_short, Env, String};

#[test]
fn test_hello_returns_greeting_string() {
    // Set up the simulated blockchain environment.
    let env = Env::default();

    // Register the contract and obtain an auto-generated contract ID.
    let contract_id = env.register_contract(None, HelloContract);

    // Build a typed client so we can call contract methods directly in tests.
    let client = HelloContractClient::new(&env, &contract_id);

    // Call hello() with "World".
    let result = client.hello(&symbol_short!("World"));

    // The contract should return the full greeting string.
    assert_eq!(result, String::from_str(&env, "Hello, World!"));
}

#[test]
fn test_hello_with_different_names() {
    let env = Env::default();
    let contract_id = env.register_contract(None, HelloContract);
    let client = HelloContractClient::new(&env, &contract_id);

    // Verify the greeting is correct for several different names.
    for (sym, expected) in [
        (symbol_short!("Alice"), "Hello, Alice!"),
        (symbol_short!("Bob"), "Hello, Bob!"),
        (symbol_short!("Stellar"), "Hello, Stellar!"),
    ] {
        let result = client.hello(&sym);
        assert_eq!(result, String::from_str(&env, expected));
    }
}

#[test]
fn test_hello_starts_with_hello() {
    let env = Env::default();
    let contract_id = env.register_contract(None, HelloContract);
    let client = HelloContractClient::new(&env, &contract_id);

    let result = client.hello(&symbol_short!("Test"));

    // Copy the response bytes into a local buffer so we can inspect them.
    let mut buf = [0u8; 40];
    let len = result.len() as usize;
    result.copy_into_slice(&mut buf[..len]);

    let result_str = core::str::from_utf8(&buf[..len]).unwrap();
    assert!(
        result_str.starts_with("Hello, "),
        "Expected greeting to begin with 'Hello, ', got: {result_str}"
    );
}
