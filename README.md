# soroban-gmp-hub

> **Status: Foundation / pre-alpha** — core skeleton only. Not production-ready.

A **Generalized Cross-Chain Messaging Protocol (GMP) Hub** built on Stellar using Soroban smart contracts. It acts as a unified on-chain endpoint for sending and receiving arbitrary messages and token transfers across heterogeneous blockchains.

---

## Problem

Stellar has no native standard for general-purpose cross-chain messaging. Developers who need to communicate with EVM chains, Cosmos, or Solana must integrate each bridge protocol (Axelar, Wormhole, LayerZero, etc.) individually, leading to:

- Fragmented UX and duplicated integration effort
- No common message format or replay protection
- No unified event log for cross-chain activity
- Protocol lock-in — switching bridges requires rewriting contracts

---

## Solution

`soroban-gmp-hub` provides a single Soroban contract that:

1. **Accepts outbound messages** from any Stellar dApp via a stable `send_message` API
2. **Receives inbound messages** from registered adapters (one per external protocol)
3. **Enforces replay protection** on inbound messages via persistent message-ID tracking
4. **Emits structured events** (`MSG_SENT`, `MSG_RECV`) consumable by indexers and UI

The hub is **protocol-agnostic** — the adapter layer is kept separate from the core hub so new bridge protocols can be added without changing the hub interface.

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   Stellar / Soroban                  │
│                                                      │
│   dApp / User                                        │
│       │                                              │
│       │  send_message(dest_chain, dest_addr, payload)│
│       ▼                                              │
│  ┌──────────────────────────────────┐               │
│  │          GmpHub Contract         │               │
│  │                                  │               │
│  │  • Admin / access control        │               │
│  │  • Outbound message sequencing   │               │
│  │  • Inbound replay guard          │               │
│  │  • Event emission                │               │
│  └───────────┬──────────────────────┘               │
│              │                                       │
└──────────────┼───────────────────────────────────────┘
               │ (future: adapter registry)
    ┌──────────┴────────────┐
    │                       │
┌───▼───┐            ┌──────▼──────┐
│Axelar │            │  Wormhole   │  ← future adapters
│Adapter│            │  Adapter    │
└───┬───┘            └──────┬──────┘
    │                       │
  Ethereum / Cosmos      Solana / EVM
```

### Storage model

| Key | Storage tier | Value |
|-----|-------------|-------|
| `Admin` | Instance | `Address` |
| `MessageCount` | Instance | `u64` |
| `Adapter(chain)` | Instance | `Address` |
| `ProcessedMsg(msg_id)` | Persistent | `bool` |

---

## Features (Foundation)

- [x] One-time contract initialization with admin address
- [x] Admin transfer with auth guard
- [x] `send_message` — dispatch with event emission and monotonic counter
- [x] `receive_message` — inbound with replay protection and event emission
- [x] `message_count` — query total outbound messages
- [x] Adapter registry — `register_adapter` / `remove_adapter` / `get_adapter` (one adapter per chain, admin-gated)
- [x] `receive_message` accepts registered adapter as authorized relayer
- [ ] Multi-adapter support per chain (next iteration)
- [ ] Adapter signature / proof verification (next iteration)
- [ ] Token escrow / SAC integration (next iteration)
- [ ] On-chain fee configuration (next iteration)
- [ ] Multi-sig admin (next iteration)

---

## Quickstart

### Prerequisites

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked stellar-cli
```

### Build

```sh
git clone https://github.com/<your-org>/soroban-gmp-hub.git
cd soroban-gmp-hub
cargo build
stellar contract build
```

### Test

```sh
cargo test -p soroban-gmp-hub
```

### Deploy to testnet

```sh
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/soroban_gmp_hub.wasm \
  --source <your-secret-key> \
  --network testnet
```

### Initialize

```sh
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <your-secret-key> \
  --network testnet \
  -- initialize \
  --admin <ADMIN_ADDRESS>
```

### Send a message

```sh
stellar contract invoke \
  --id <CONTRACT_ID> \
  --source <sender-key> \
  --network testnet \
  -- send_message \
  --sender <SENDER_ADDRESS> \
  --destination_chain '"ethereum"' \
  --destination_address '"0xYourContractAddress"' \
  --payload '"68656c6c6f"'
```

---

## Integration Guide

### For Stellar dApps

1. Deploy or reference the hub contract address.
2. Call `send_message` with your destination chain, destination contract address (as hex string), and ABI-encoded payload.
3. Listen for the `MSG_SENT` event to get the `msg_id` for tracking.

### For Relayers / Adapters (future)

Once the adapter registry is implemented, a relayer will:

1. Watch the source chain for your protocol's cross-chain event.
2. Decode the payload and reconstruct the `msg_id`.
3. Call `receive_message` on the hub, authenticated as a registered adapter.
4. The hub emits `MSG_RECV` — downstream contracts subscribe to this event.

---

## Roadmap

### Phase 1 — Foundation (current)
- Core contract skeleton
- Admin control
- `send_message` / `receive_message` stubs
- Replay protection
- Basic tests

### Phase 2 — Adapter Registry
- On-chain adapter registration (admin-gated)
- Per-adapter authorization (signature verification)
- Upgrade path for adapter keys

### Phase 3 — Token Support
- Stellar Asset Contract (SAC) integration
- Lock-and-mint token escrow pattern
- Multi-token support

### Phase 4 — Fee & Config
- On-chain fee per destination chain
- Fee recipient configuration
- Minimum/maximum payload size limits

### Phase 5 — Security Hardening
- Formal verification targets
- Rate limiting per sender
- Multi-sig admin transition
- Third-party audit

---

## Project Structure

```
soroban-gmp-hub/
├── Cargo.toml                  # workspace
├── contracts/
│   └── gmp-hub/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs          # contract (send_message, receive_message, admin)
│           └── test.rs         # unit tests
├── README.md
├── CONTRIBUTING.md
└── .gitignore
```

---

## License

To be added by the project maintainer.
