#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    Bytes, Env, String,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, Address, GmpHubClient<'static>) {
    let env  = Env::default();
    let id   = env.register_contract(None, GmpHub);
    let client = GmpHubClient::new(&env, &id);
    let admin  = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin).unwrap();

    (env, admin, client)
}

fn str(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

fn bytes(env: &Env, data: &[u8]) -> Bytes {
    Bytes::from_slice(env, data)
}

// ── Initialization ────────────────────────────────────────────────────────────

#[test]
fn initialize_sets_admin() {
    let (_, admin, client) = setup();
    assert_eq!(client.admin().unwrap(), admin);
}

#[test]
fn initialize_twice_returns_error() {
    let (env, admin, client) = setup();
    let result = client.try_initialize(&admin);
    assert!(result.is_err());
    let _ = env; // silence unused warning
}

#[test]
fn message_count_starts_at_zero() {
    let (_, _, client) = setup();
    assert_eq!(client.message_count(), 0);
}

// ── Admin transfer ────────────────────────────────────────────────────────────

#[test]
fn transfer_admin_updates_admin() {
    let (env, _old_admin, client) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&new_admin).unwrap();
    assert_eq!(client.admin().unwrap(), new_admin);
}

// ── send_message ──────────────────────────────────────────────────────────────

#[test]
fn send_message_returns_msg_id_and_increments_count() {
    let (env, _admin, client) = setup();

    let sender = Address::generate(&env);
    let msg_id = client
        .send_message(
            &sender,
            &str(&env, "ethereum"),
            &str(&env, "0xDeadBeef"),
            &bytes(&env, b"hello cross-chain"),
        )
        .unwrap();

    // msg_id must be non-zero (ledger sequence is 1 by default in tests)
    assert_ne!(msg_id, soroban_sdk::BytesN::from_array(&env, &[0u8; 32]));
    assert_eq!(client.message_count(), 1);
}

#[test]
fn send_message_increments_count_each_call() {
    let (env, _admin, client) = setup();
    let sender = Address::generate(&env);

    for i in 1..=3u64 {
        client
            .send_message(
                &sender,
                &str(&env, "polygon"),
                &str(&env, "0xCafe"),
                &bytes(&env, b"msg"),
            )
            .unwrap();
        assert_eq!(client.message_count(), i);
    }
}

// ── receive_message ───────────────────────────────────────────────────────────

#[test]
fn receive_message_succeeds_for_new_id() {
    let (env, _admin, client) = setup();

    let msg_id = soroban_sdk::BytesN::from_array(&env, &[1u8; 32]);
    let result = client.receive_message(
        &str(&env, "ethereum"),
        &msg_id,
        &bytes(&env, b"inbound payload"),
    );
    assert!(result.is_ok());
}

#[test]
fn receive_message_rejects_duplicate_id() {
    let (env, _admin, client) = setup();

    let msg_id = soroban_sdk::BytesN::from_array(&env, &[2u8; 32]);
    client
        .receive_message(
            &str(&env, "ethereum"),
            &msg_id,
            &bytes(&env, b"first delivery"),
        )
        .unwrap();

    // Second call with same msg_id must fail.
    let result = client.try_receive_message(
        &str(&env, "ethereum"),
        &msg_id,
        &bytes(&env, b"duplicate"),
    );
    assert!(result.is_err());
}
