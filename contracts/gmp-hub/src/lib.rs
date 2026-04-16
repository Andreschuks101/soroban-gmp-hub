#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracterror, contracttype, symbol_short,
    Address, Bytes, BytesN, Env, String,
};

// ── Errors ────────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized    = 1,
    NotInitialized        = 2,
    MessageAlreadyProcessed = 3,
}

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    MessageCount,
    /// Marks a (source_chain, msg_id) pair as processed to prevent replays.
    ProcessedMsg(BytesN<32>),
}

// ── Event payloads ────────────────────────────────────────────────────────────

/// Emitted when a cross-chain message is dispatched from this hub.
#[contracttype]
#[derive(Clone)]
pub struct MessageSentEvent {
    pub msg_id:              BytesN<32>,
    pub sender:              Address,
    pub destination_chain:   String,
    pub destination_address: String,
    pub payload:             Bytes,
}

/// Emitted when a cross-chain message is delivered to this hub.
#[contracttype]
#[derive(Clone)]
pub struct MessageReceivedEvent {
    pub msg_id:       BytesN<32>,
    pub source_chain: String,
    pub payload:      Bytes,
}

// ── Contract ──────────────────────────────────────────────────────────────────

#[contract]
pub struct GmpHub;

#[contractimpl]
impl GmpHub {
    // ── Admin ─────────────────────────────────────────────────────────────────

    /// One-time initializer. Must be called before any other function.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::MessageCount, &0u64);
        Ok(())
    }

    /// Return the current admin address.
    pub fn admin(env: Env) -> Result<Address, Error> {
        env.storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)
    }

    /// Transfer the admin role. Requires auth from the current admin.
    pub fn transfer_admin(env: Env, new_admin: Address) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &new_admin);
        Ok(())
    }

    // ── Messaging ─────────────────────────────────────────────────────────────

    /// Dispatch a cross-chain message.
    ///
    /// `sender` must authorize this call. A unique `msg_id` is returned that
    /// can be used to track the message on the destination chain.
    ///
    /// # Stub note
    /// Production will forward `payload` (and optional token transfers) to a
    /// registered chain-specific adapter (e.g. Axelar, LayerZero). The adapter
    /// registry and token escrow logic will be added in a future iteration.
    pub fn send_message(
        env: Env,
        sender: Address,
        destination_chain: String,
        destination_address: String,
        payload: Bytes,
    ) -> Result<BytesN<32>, Error> {
        sender.require_auth();

        let count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MessageCount)
            .unwrap_or(0);

        let msg_id = Self::derive_msg_id(&env, count);

        env.storage()
            .instance()
            .set(&DataKey::MessageCount, &(count + 1));

        env.events().publish(
            (symbol_short!("MSG_SENT"), destination_chain.clone()),
            MessageSentEvent {
                msg_id: msg_id.clone(),
                sender,
                destination_chain,
                destination_address,
                payload,
            },
        );

        Ok(msg_id)
    }

    /// Receive and record an inbound cross-chain message.
    ///
    /// In the foundation only the admin can call this (acting as a trusted
    /// relayer). A future iteration will replace admin auth with a signed proof
    /// verified against a registered adapter — preserving the same interface.
    ///
    /// Reverts if `msg_id` has already been processed (replay protection).
    pub fn receive_message(
        env: Env,
        source_chain: String,
        msg_id: BytesN<32>,
        payload: Bytes,
    ) -> Result<(), Error> {
        // Placeholder adapter auth: admin acts as the sole trusted relayer.
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        // Replay guard.
        if env
            .storage()
            .persistent()
            .has(&DataKey::ProcessedMsg(msg_id.clone()))
        {
            return Err(Error::MessageAlreadyProcessed);
        }
        env.storage()
            .persistent()
            .set(&DataKey::ProcessedMsg(msg_id.clone()), &true);

        env.events().publish(
            (symbol_short!("MSG_RECV"), source_chain.clone()),
            MessageReceivedEvent {
                msg_id,
                source_chain,
                payload,
            },
        );

        Ok(())
    }

    /// Return the running count of outbound messages.
    pub fn message_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::MessageCount)
            .unwrap_or(0)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Build a 32-byte message ID from the current ledger sequence and a
    /// per-hub monotonic counter. Production will SHA-256 over all message
    /// fields for a globally unique, collision-resistant ID.
    fn derive_msg_id(env: &Env, count: u64) -> BytesN<32> {
        let ledger = env.ledger().sequence();
        let mut raw = [0u8; 32];
        raw[0..4].copy_from_slice(&ledger.to_be_bytes());
        raw[4..12].copy_from_slice(&count.to_be_bytes());
        BytesN::from_array(env, &raw)
    }
}

#[cfg(test)]
mod test;
