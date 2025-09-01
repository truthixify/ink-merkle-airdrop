# Merkle Airdrop dApp

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Built with ink!](https://raw.githubusercontent.com/paritytech/ink/master/.images/badge.svg)](https://use.ink)
![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-000000?logo=typescript&logoColor=white)
![Next.js](https://img.shields.io/badge/Next.js-000000?logo=next.js&logoColor=white)

> A decentralized airdrop system using Merkle trees for efficient and secure token distribution on Polkadot

## Features

- 🌳 **Merkle Tree Verification**: Efficient proof-based token claiming system
- 🪙 **ERC20 Integration**: Built-in ERC20 token contract for airdrop distribution  
- 🔒 **Secure Claims**: Cryptographic proofs prevent double-claiming and unauthorized access
- 🎯 **Gas Efficient**: Minimal on-chain storage using Merkle root verification
- 🌐 **Full-Stack**: Complete dApp with smart contracts and modern web interface

## How It Works

The Merkle Airdrop system allows efficient distribution of tokens to a large number of recipients:

1. **Merkle Tree Generation**: Create a Merkle tree with recipient addresses and amounts
2. **Contract Deployment**: Deploy the airdrop contract with the Merkle root
3. **Token Funding**: Fund the contract with ERC20 tokens for distribution
4. **Claim Process**: Recipients provide Merkle proofs to claim their allocated tokens
5. **Verification**: Smart contract verifies proofs against the stored Merkle root

## Quickstart ⚡

> [!IMPORTANT]
>
> - Setup Node.js v20+ (recommended via [nvm](https://github.com/nvm-sh/nvm))
> - Install [Bun](https://bun.sh/)
> - Install [Rust](https://rustup.rs/) and [cargo-contract](https://github.com/paritytech/cargo-contract)

Clone and setup the project:

```bash
git clone <repository-url>
cd merkle_airdrop

bun install
bun run dev
```

This will start both the local Substrate node and the Next.js frontend.

## Smart Contracts

### MerkleAirdrop Contract

The main contract handles the airdrop logic:

- **Constructor**: Initialize with ERC20 contract and Merkle root
- **fund()**: Add tokens to the airdrop pool
- **claim()**: Verify Merkle proof and distribute tokens
- **is_claimed()**: Check if an address has already claimed
- **root()**: Get the stored Merkle root

### ERC20 Contract

Standard ERC20 implementation for the airdrop tokens with:
- Standard transfer functionality
- Allowance system for contract interactions
- Mint capability for initial token creation

## Project Structure

```
merkle_airdrop/
├── contracts/           # ink! Smart Contracts
│   ├── src/
│   │   ├── erc20/      # ERC20 token contract
│   │   └── merkle_airdrop/  # Main airdrop contract
│   └── scripts/        # Deployment scripts
├── frontend/           # Next.js Frontend
│   ├── src/
│   │   ├── components/ # React components
│   │   ├── pages/      # Next.js pages
│   │   └── lib/        # Utilities and contract interactions
│   └── public/         # Static assets
└── package.json        # Workspace configuration
```

## Development Commands

```bash
# Start development environment
bun run dev

# Start local Substrate node
bun run node

# Build smart contracts
bun run -F contracts build

# Generate contract types
bun run codegen

# Deploy contracts
bun run -F contracts deploy

# Build frontend for production
bun run build

# Run tests
bun run test

# Lint and format code
bun run lint:fix
```

## Technology Stack

- **Smart Contracts**: ink! v6 with PolkaVM compatibility
- **Frontend**: Next.js 15, React 19, TypeScript
- **Styling**: Tailwind CSS v4
- **Blockchain Interaction**: Polkadot API (PAPI), ReactiveDOT
- **Development**: Bun, Docker support
- **UI Components**: Radix UI primitives
