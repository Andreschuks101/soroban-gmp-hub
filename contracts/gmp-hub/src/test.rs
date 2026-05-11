#![cfg(test)]

use super::*;
use soroban_sdk::{
    testutils::Address as _,
    Bytes, BytesN, Env, String,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, Address, GmpHubClient<'static>) {
    let env    = Env::default();
    let id     = env.register_contract(None, GmpHub);
    let client = GmpHubClient::new(&env, &id);
    let admin  = Address::generate(&env);

    env.mock_all_auths();
    client.initialize(&admin).unwrap();

    (env, admin, client)
}

fn chain(env: &Env, s: &str) -> String  { String::from_str(env, s) }
fn bytes(env: &Env, d: &[u8]) -> Bytes  { Bytes::from_slice(env, d) }
fn msg_id(env: &Env, b: u8) -> BytesN<32> { BytesN::from_array(env, &[b; 32]) }

// ── Initialization ────────────────────────────────────────────────────────────

#[test]
fn initialize_sets_admin() {
    let (_, admin, client) = setup();
    assert_eq!(client.admin().unwrap(), admin);
}

#[test]
fn initialize_twice_returns_error() {
    let (env, admin, client) = setup();
    let _ = env;
    assert!(client.try_initialize(&admin).is_err());
}

#[test]
fn message_count_starts_at_zero() {
    let (_, _, client) = setup();
    assert_eq!(client.message_count(), 0);
}

// ── Admin transfer ────────────────────────────────────────────────────────────

#[test]
fn transfer_admin_updates_admin() {
    let (env, _old, client) = setup();
    let new_admin = Address::generate(&env);
    client.transfer_admin(&new_admin).unwrap();
    assert_eq!(client.admin().unwrap(), new_admin);
}

// ── Adapter registry ──────────────────────────────────────────────────────────

#[test]
fn register_adapter_stores_address() {
    let (env, _admin, client) = setup();
    let adapter = Address::generate(&env);

    client.register_adapter(&chain(&env, "ethereum"), &adapter).unwrap();

    assert_eq!(
        client.get_adapter(&chain(&env, "ethereum")),
        Some(adapter)
    );
}

#[test]
fn register_adapter_twice_returns_error() {
    let (env, _admin, client) = setup();
    let adapter = Address::generate(&env);

    client.register_adapter(&chain(&env, "ethereum"), &adapter).unwrap();

    assert!(client
        .try_register_adapter(&chain(&env, "ethereum"), &adapter)
        .is_err());
}

#[test]
fn remove_adapter_clears_entry() {
    let (env, _admin, client) = setup();
    let adapter = Address::generate(&env);

    client.register_adapter(&chain(&env, "polygon"), &adapter).unwrap();
    client.remove_adapter(&chain(&env, "polygon")).unwrap();

    assert_eq!(client.get_adapter(&chain(&env, "polygon")), None);
}

#[test]
fn get_adapter_returns_none_for_unknown_chain() {
    let (env, _, client) = setup();
    assert_eq!(client.get_adapter(&chain(&env, "solana")), None);
}

// ── send_message ──────────────────────────────────────────────────────────────

#[test]
fn send_message_returns_msg_id_and_increments_count() {
    let (env, _admin, client) = setup();
    let sender = Address::generate(&env);

    let id = client
        .send_message(
            &sender,
            &chain(&env, "ethereum"),
            &String::from_str(&env, "0xDeadBeef"),
            &bytes(&env, b"hello cross-chain"),
        )
        .unwrap();

    assert_ne!(id, msg_id(&env, 0));
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
                &chain(&env, "polygon"),
                &String::from_str(&env, "0xCafe"),
                &bytes(&env, b"msg"),
            )
            .unwrap();
        assert_eq!(client.message_count(), i);
    }
}

// ── receive_message ───────────────────────────────────────────────────────────

#[test]
fn receive_message_succeeds_for_admin_relayer() {
    let (env, admin, client) = setup();

    client
        .receive_message(
            &admin,
            &chain(&env, "ethereum"),
            &msg_id(&env, 1),
            &bytes(&env, b"inbound payload"),
        )
        .unwrap();
}

#[test]
fn receive_message_succeeds_for_registered_adapter() {
    let (env, _admin, client) = setup();
    let adapter = Address::generate(&env);

    client.register_adapter(&chain(&env, "ethereum"), &adapter).unwrap();

    client
        .receive_message(
            &adapter,
            &chain(&env, "ethereum"),
            &msg_id(&env, 2),
            &bytes(&env, b"via adapter"),
        )
        .unwrap();
}

#[test]
fn receive_message_rejects_unknown_relayer() {
    let (env, _admin, client) = setup();
    let stranger = Address::generate(&env);

    assert!(client
        .try_receive_message(
            &stranger,
            &chain(&env, "ethereum"),
            &msg_id(&env, 3),
            &bytes(&env, b"unauthorized"),
        )
        .is_err());
}

#[test]
fn receive_message_rejects_duplicate_id() {
    let (env, admin, client) = setup();
    let id = msg_id(&env, 4);

    client
        .receive_message(&admin, &chain(&env, "ethereum"), &id, &bytes(&env, b"first"))
        .unwrap();

    assert!(client
        .try_receive_message(&admin, &chain(&env, "ethereum"), &id, &bytes(&env, b"dup"))
        .is_err());
}
