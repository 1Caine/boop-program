# Boop program

The on-chain program behind [boop.cash](https://www.boop.cash): payments on Solana in SOL,
USDC and EURC, Booptags, savings, shared goals and two-of-three vaults.

| Network | Program ID |
| --- | --- |
| Mainnet | `AzskZwMoTeXbz6pDby3vNq3nthYWcT59D61pfLEoThwX` |
| Devnet | `2MAFKZZyVE3J5zjjZMSTdBXPL4B7R82AiBgM6eYFiaf9` |

This repository holds only the program source. The website and its backend are not here.

## What it does

- Payments in SOL and in classic SPL USDC and EURC, with a fee bounded at 5% (0.75% by default).
- Booptags: the first is free, additional ones are paid in USDC, and owners can list one for sale.
- Savings in USDC, owned by the depositing wallet.
- Shared goals in USDC, where each member owns their own contribution.
- Vaults in USDC that need any two of three signers and have a daily limit.

See [SECURITY.md](SECURITY.md) for the invariants the program enforces.

## Build

The default build is the mainnet release. `--features devnet` builds the devnet program
(its own program ID and Circle's devnet USDC mint).

```sh
cargo build-sbf --manifest-path programs/boop_program/Cargo.toml
cargo test --workspace --locked
```

Anchor 1.1.2. Accepted mints: USDC `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` and
EURC `HzwqbKZw8HxMN6bF2yFZNrht3c2iXXzpKcFu7uBEDKtr`.

## Verify the deployed program

The mainnet program is built by the `verify` workflow in this repository inside a pinned
Docker image, and that exact binary is what is deployed.

```sh
cargo install solana-verify
solana-verify get-program-hash -u mainnet-beta AzskZwMoTeXbz6pDby3vNq3nthYWcT59D61pfLEoThwX
solana-verify verify-from-repo -u mainnet-beta \
  --program-id AzskZwMoTeXbz6pDby3vNq3nthYWcT59D61pfLEoThwX \
  --library-name boop_program \
  --base-image solanafoundation/solana-verifiable-build@sha256:16053d845922e798ab1852d3fe222faf5a23eeb1db3a13d30b70d6b6e82184ae \
  https://github.com/1Caine/boop-program
```

The first command prints the hash of the program on chain. It matches the hash printed by
the workflow run for the deployed commit.
