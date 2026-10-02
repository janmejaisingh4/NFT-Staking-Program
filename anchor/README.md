# Anchor Vault Program

This template includes a simple SOL vault program built with [Anchor](https://www.anchor-lang.com/).

## Pre-deployed Program

The vault program is deployed on **devnet** at:

```
F4jZpgbtTb6RWNWq6v35fUeiAsRJMrDczVPv9U23yXjB
```

You can interact with it immediately by connecting your wallet to devnet.

## Deploying Your Own Program

To deploy your own version of the program:

### 1. Generate a new program keypair

```bash
cd anchor
solana-keygen new -o target/deploy/vault-keypair.json
```

### 2. Get the new program ID

```bash
solana address -k target/deploy/vault-keypair.json
```

### 3. Update the program ID

Update the program ID in these files:

- `anchor/Anchor.toml` - Update `vault = "..."` under `[programs.devnet]`
- `anchor/programs/vault/src/lib.rs` - Update `declare_id!("...")`

### 4. Build and deploy

```bash
# Build the program
anchor build

# Get devnet SOL for deployment (~2 SOL needed)
solana airdrop 2 --url devnet

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

### 5. Regenerate the TypeScript client

```bash
cd ..
npm run codama:js
```

This updates the generated client code in `app/generated/vault/` with your new program ID.

## Program Overview

The program retains the original SOL vault instructions and adds Metaplex Core NFT staking:

- `initialize_pool`: Create a pool and its 9-decimal SPL reward mint for a Core collection. The collection receives an `Attributes` plugin with `total_staked = "0"`.
- `stake_nft`: Create a stake record, install pool-controlled `FreezeDelegate` and `BurnDelegate` plugins on the asset, freeze it, and increment the collection counter.
- `claim_rewards`: Mint accrued rewards to the owner's associated token account without changing the stake record or thawing the NFT.
- `unstake_nft`: Mint any remaining rewards, thaw the NFT, close the stake record, and decrement the collection counter.
- `burn_staked_nft`: Burn the staked Core asset and mint accrued rewards plus the pool's configured one-time bonus.

Rewards accrue in base units per second. The pool administrator chooses both the reward rate and burn bonus when initializing the pool. Pool, authority, and stake-record PDAs use the seeds `pool + collection`, `pool-authority + collection`, and `stake + asset` respectively. The collection must be a Metaplex Core collection whose update authority can add the initial Attributes plugin.

The optional time-window transfer Oracle plugin is not included.

The original `deposit` and `withdraw` instructions continue to manage a per-user SOL vault PDA.

## Testing

Run the Anchor tests:

```bash
anchor test --skip-deploy
```
