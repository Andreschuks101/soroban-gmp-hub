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
    AlreadyInitialized      = 1,
    NotInitialized          = 2,
    MessageAlreadyProcessed = 3,
    AdapterAlreadyRegistered = 4,
    Unauthorized            = 5,
}

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Admin,
    MessageCount,
    /// Trusted relayer address for a given chain name.
    /// e.g. DataKey::Adapter(String::from_str(&env, "ethereum"))
    Adapter(String),
    /// Marks a msg_id as processed to prevent replays.
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
    pub relayer:      Address,
    pub payload:      Bytes,
}

/// Emitted when an adapter is registered for a chain.
#[contracttype]
#[derive(Clone)]
pub struct AdapterRegisteredEvent {
    pub chain:   String,
    pub adapter: Address,
}

/// Emitted when an adapter is removed from a chain.
#[contracttype]
#[derive(Clone)]
pub struct AdapterRemovedEvent {
    pub chain: String,
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

    // ── Adapter registry ──────────────────────────────────────────────────────

    /// Register a trusted relayer address for a given chain.
    ///
    /// Only one adapter per chain is supported in this iteration.
    /// Contributors: extend this to support multiple adapters per chain,
    /// adapter metadata (protocol name, version), and capability flags.
    pub fn register_adapter(
        env: Env,
        chain: String,
        adapter: Address,
    ) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        if env.storage().instance().has(&DataKey::Adapter(chain.clone())) {
            return Err(Error::AdapterAlreadyRegistered);
        }

        env.storage()
            .instance()
            .set(&DataKey::Adapter(chain.clone()), &adapter);

        env.events().publish(
            (symbol_short!("ADPT_REG"), chain.clone()),
            AdapterRegisteredEvent { chain, adapter },
        );

        Ok(())
    }

    /// Remove the adapter for a given chain.
    ///
    /// Contributors: consider emitting a deprecation window event so
    /// in-flight messages can still be settled before removal takes effect.
    pub fn remove_adapter(env: Env, chain: String) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;
        admin.require_auth();

        env.storage()
            .instance()
            .remove(&DataKey::Adapter(chain.clone()));

        env.events().publish(
            (symbol_short!("ADPT_RM"), chain.clone()),
            AdapterRemovedEvent { chain },
        );

        Ok(())
    }

    /// Return the registered adapter address for a chain, or None.
    pub fn get_adapter(env: Env, chain: String) -> Option<Address> {
        env.storage()
            .instance()
            .get(&DataKey::Adapter(chain))
    }

    // ── Messaging ─────────────────────────────────────────────────────────────

    /// Dispatch a cross-chain message.
    ///
    /// `sender` must authorize this call. A unique `msg_id` is returned and
    /// can be used to track the message on the destination chain.
    ///
    /// Contributors: route `payload` to the chain's registered adapter here,
    /// and add optional token escrow (SAC lock) before the event is emitted.
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
    /// `relayer` must be either:
    ///   a) the hub admin, or
    ///   b) the registered adapter for `source_chain`.
    ///
    /// Reverts if `msg_id` has already been processed (replay protection).
    ///
    /// Contributors: replace the address-equality check with a cryptographic
    /// proof verification step (e.g. verify a threshold signature from the
    /// adapter's validator set before accepting the message).
    pub fn receive_message(
        env: Env,
        relayer: Address,
        source_chain: String,
        msg_id: BytesN<32>,
        payload: Bytes,
    ) -> Result<(), Error> {
        relayer.require_auth();

        // Allow admin or the chain's registered adapter.
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::NotInitialized)?;

        let registered: Option<Address> = env
            .storage()
            .instance()
            .get(&DataKey::Adapter(source_chain.clone()));

        let authorized = relayer == admin
            || registered.map(|a| a == relayer).unwrap_or(false);

        if !authorized {
            return Err(Error::Unauthorized);
        }

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
                relayer,
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
    /// per-hub monotonic counter.
    ///
    /// Contributors: replace with SHA-256 over (ledger, count, sender, chain,
    /// destination_address, payload) for a globally collision-resistant ID.
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
