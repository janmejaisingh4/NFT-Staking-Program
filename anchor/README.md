# NFT Staking Core — Task 1

A Solana program (Anchor) that lets users stake **Metaplex Core** NFTs and earn SPL reward tokens.
It uses Core plugins so the NFT never leaves the user's wallet while staked: it is simply **frozen**.

---

## 1. What this program does

| # | Feature | Instruction | Result |
|---|---------|-------------|--------|
| 1 | Claim rewards without unstaking | `claim_rewards` | Reward tokens are minted to the user's ATA. The NFT stays staked and frozen. |
| 2 | Burn-to-Earn with BurnDelegate | `burn_staked_nft` | The NFT is permanently burned. The user gets pending rewards plus a one-time bonus. |
| 3 | Collection-level staking stats | (built into stake, unstake, burn) | The `total_staked` Attribute on the Collection account is updated automatically. |

Supporting instructions: `initialize`, `mint_nft`, `stake`, `unstake`.

---

## 2. How it works (big picture)

```
            ┌──────────────────────────── Core Collection ────────────────────────────┐
            │ update authority = program PDA ("authority")                              │
            │ Attributes plugin:  total_staked = "N"   <── updated on stake/unstake/burn │
            └────────────────────────────────────────────────────────────────────────────┘

 User wallet owns NFT ──stake──►  NFT gets 2 plugins, both delegated to the program PDA:
                                    • FreezeDelegate { frozen: true }   → NFT can't move
                                    • BurnDelegate                      → program can burn it
```

**Why a PDA?** The program PDA (`authority`) is the collection's update authority, the freeze delegate, the burn delegate and the reward mint authority. This lets the program sign every Core action itself, without any private key.

---

## 3. Accounts and PDAs

| Account | Seeds | Purpose |
|---------|-------|---------|
| `authority` | `["authority", collection]` | Signs all Core CPIs and is the reward mint authority |
| `config` | `["config", collection]` | Stores admin, collection, reward mint, reward rate, burn bonus |
| `reward_mint` | `["rewards", collection]` | SPL mint for reward tokens (6 decimals) |
| `stake_record` | `["stake", asset]` | One per staked NFT: owner, `staked_at`, `last_claim` |

**Config fields**

- `reward_per_second` — reward tokens (in base units) earned per second per NFT
- `burn_bonus` — extra reward tokens paid once when an NFT is burned

---

## 4. Instruction reference

### `initialize(name, uri, reward_per_second, burn_bonus)`
Run once by the admin.
1. Creates the Core Collection with the program PDA as update authority.
2. Adds the Attributes plugin with `total_staked = "0"`.
3. Creates the `config` account and the `reward_mint`.

The collection is a **new keypair that must sign** the transaction.

### `mint_nft(name, uri)`
Admin only. Mints an NFT into the collection and sends it to a recipient. This instruction is needed because only the PDA can create assets in this collection. The asset is a new keypair that must sign.

### `stake`
1. Checks that the signer owns the NFT and that the NFT belongs to the collection.
2. Adds **BurnDelegate** with the PDA as authority.
3. Adds **FreezeDelegate** (`frozen: true`) with the PDA as authority.
4. Increments `total_staked` on the Collection.
5. Creates the `stake_record` with `last_claim = now`.

### `claim_rewards` (Task 1.1)
1. Calculates `pending = (now − last_claim) × reward_per_second`.
2. Fails with `NothingToClaim` if `pending` is 0.
3. Sets `last_claim = now`.
4. Mints `pending` tokens to the owner's ATA. The ATA is created if it doesn't exist.

The NFT is untouched: it stays staked and frozen.

### `unstake`
1. Thaws the NFT (`frozen: false`, signed by the PDA).
2. Removes the FreezeDelegate and BurnDelegate plugins.
3. Decrements `total_staked`.
4. Mints pending rewards.
5. Closes the `stake_record`. Its rent returns to the owner.

### `burn_staked_nft` (Task 1.2)
1. Thaws the NFT. A frozen asset cannot be burned.
2. Burns the NFT using the **BurnDelegate** authority (the PDA).
3. Decrements `total_staked`.
4. Mints `pending + burn_bonus` to the owner's ATA.
5. Closes the `stake_record`.

All steps happen in a single transaction, so they either all succeed or all fail.

### Collection stats (Task 1.3)
`total_staked` is stored as an **Attribute on the Collection account**. After every stake, unstake or burn, the program reads the current list, changes only the `total_staked` value, and writes the list back. This keeps any other attributes you add later.

You can read it from any client:
```ts
// Umi example
const collection = await fetchCollection(umi, collectionAddress);
const total = collection.attributes?.attributeList.find(a => a.key === "total_staked")?.value;
```

---

## 5. Project structure

```
nft-staking-core/
├── README.md
└── programs/nft-staking-core/
    ├── Cargo.toml
    └── src/lib.rs
```

---

## 6. Setup and build

**Prerequisites:** Rust, Solana CLI, Anchor 0.30.1, Node.js (for the client or tests).

```bash
# 1. Put the files in an Anchor workspace (anchor init nft-staking-core), then:
anchor keys sync        # replaces the placeholder program id in lib.rs
anchor build
anchor deploy           # on devnet: solana config set -u devnet
```

**Dependencies** (already in `Cargo.toml`):
- `anchor-lang` 0.30.1 with the `init-if-needed` feature
- `anchor-spl` 0.30.1
- `mpl-core` 0.8.0

---

## 7. Step-by-step usage

1. **Initialize** (admin)
   Call `initialize` with a new collection keypair. Example values: `reward_per_second = 1_000_000` (1 token/sec) and `burn_bonus = 1_000_000_000` (1,000 tokens).
2. **Mint NFTs** (admin)
   Call `mint_nft` with a new asset keypair and the recipient's wallet.
3. **Stake** (user)
   Call `stake`. The NFT is now frozen and `total_staked` goes up by 1.
4. **Claim** (user)
   Wait a few seconds and call `claim_rewards`. Tokens arrive in the user's ATA and the NFT is still frozen.
5. **Finish** (user): choose one of two paths:
   - `unstake` to get the NFT back plus remaining rewards, or
   - `burn_staked_nft` to destroy the NFT for rewards plus the bonus.
6. **Verify**
   Check the Collection's `total_staked` attribute and the reward token balance.

---

## 8. Accounts required per instruction

| Instruction | Accounts to pass |
|-------------|------------------|
| `initialize` | admin, collection (signer), authority, config, reward_mint, core_program, token_program, system_program |
| `mint_nft` | admin, asset (signer), collection, authority, config, recipient, core_program, system_program |
| `stake` | owner, asset, collection, authority, config, stake_record, core_program, system_program |
| `claim_rewards` | owner, asset, collection, authority, config, stake_record, reward_mint, owner_ata, token_program, associated_token_program, system_program |
| `unstake` / `burn_staked_nft` | owner, asset, collection, authority, config, stake_record, reward_mint, owner_ata, core_program, token_program, associated_token_program, system_program |

`core_program` is `CoreSo11CPtxo9zkAHnPQe38EHfTNbNHDVHJkRNPqgY` (mpl-core).

---

## 9. Errors

| Error | Meaning |
|-------|---------|
| `InvalidAsset` | The account is not a valid Core asset |
| `NotOwner` | The signer doesn't own the asset |
| `InvalidCollection` | The asset isn't in this collection |
| `NothingToClaim` | No rewards have accrued yet |
| `MathOverflow` | Arithmetic overflow, or `total_staked` would go below 0 |

---

## 10. Design notes

- **Non-custodial:** the NFT stays in the user's wallet and is frozen, never transferred to an escrow.
- **Safe burn:** thaw and burn happen in one transaction, so the NFT can never be left in a half-state.
- **Stake record:** tracking time in a small PDA keeps reward math simple and cheap, and the account is closed on exit.
- **Order of plugin adds:** BurnDelegate is added before FreezeDelegate, since adding plugins to a frozen asset may be rejected.

---

## 11. Troubleshooting

| Problem | Fix |
|---------|-----|
| `declared program id does not match` | Run `anchor keys sync`, then rebuild |
| Compile errors on CPI builders | Your `mpl-core` version differs; check the builder method signatures in its docs |
| `NothingToClaim` right after staking | Wait at least 1 second. Rewards accrue per second |
| Burn fails | Confirm that the NFT was staked through this program, so the BurnDelegate exists |

---