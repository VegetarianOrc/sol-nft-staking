# NFT Staking

This repo contains the [Anchor](https://github.com/project-serum/anchor) program that powers NFT staking rewards on [Solana](https://solana.com/). Users opt to lock up their NFTs with the protocol in exchange for periodic rewards in the form of a [Token](https://spl.solana.com/token) created exclusively for the purpose of rewarding NFT stakers.

### **Note**

**The author(s) of this software do not claim that there is any monetary value in the token rewards distributed by this protocol.**

## Deployed Program Addresses

| Cluster | Address                                      |
| ------- | -------------------------------------------- |
| devnet  | D42AsUF2UbUcyBtK2Jvbym2ALfksvgeScNNtMg7KrSfj |

## Overview

This NFT Staking Protocol consists of two main components, the **Rewarder** and user **Stake Accounts**. A **Rewarder** has information on the reward token, the reward rate, and the NFT metada that is allowed to earn rewards. A **Stake Account** has information on the owner of the account, the last time the owner claimed their rewards, and also acts as the wallet for any staked NFTs. NFT Owners can always view staked NFTs by finding associated NFT accounts for their **Stake Account**.

### Rewarder

The **Rewarder** is an on-chain [account](https://docs.solana.com/developing/programming-model/accounts) that stores about the Token that is awarded to stakers, the rate at which tokens are awarded, and the NFTs that are allowed to be staked to earn rewards with the Rewarder.

Rewards are currently calculated in a straightforward fashion where stakers earn the `rewardRate` in the reward token every second per staked NFT. There is no limit on the supply of the reward token as more will always be minted to award to stakers.

```
  rewardEarned = elapsedSeconds * rewardRate * numStakedNFTs
```

Rewarders are created per collection at the [Program Derived Address](https://docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses) derived from the following seeds:

```
[collectionName, StakingProgramID, "rewarder"]
```

#### Verifying NFT Authenticity

To ensure that only NFTs from the desired collection can earn rewards the protocol inspects associated [Metaplex Token Metadata](https://docs.metaplex.com/architecture/contracts#token-metadata) for staked NFTs. When `enforceMetadata` is set to true, the protocol will compare 3 fields from the metadata to verify authenticity:

- UpdateAutority
- Creators
- Collection Name

The metadata for staked NFTs must have matching update authority and creators to those stored in the Rewarder. The name of the NFT is compared to the `collection` field of the Rewarder where the name must begin with the `collection`. For example if the `collection` is `"gmoot"` the an NFT with the name `"gmoot bag #69"` will be allowed. **Rewarder operators should always ensure that at least 1 creator is verified using the [SignMetadata](https://github.com/metaplex-foundation/metaplex/blob/master/rust/token-metadata/program/src/instruction.rs#L148) instruction to ensure only verified NFTs can be staked.**

#### Rewarder Account Layout

| Name                     | Type             | Description                                                                                                                                                                                                                                                                                                                               |
| ------------------------ | ---------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| authority                | Pubkey           | The owner of the Rewarder. Can sign transactions to update the Rewarder                                                                                                                                                                                                                                                                   |
| reward_mint              | Pubkey           | The address of the reward [Token Mint](https://spl.solana.com/token#creating-a-new-token-type) that is used to reward stakers                                                                                                                                                                                                             |
| reward_authority_bump    | u8               | The PDA bump for the address that is used to sign [MintTo](https://github.com/solana-labs/solana-program-library/blob/master/token/program/src/instruction.rs#L174) instructions when rewarding stakers. Stored to save on-chain compute of recalculating                                                                                 |
| reward_rate              | u64              | The amount of reward tokens earned per second per staked NFT                                                                                                                                                                                                                                                                              |
| allowed_update_authority | Pubkey           | The Pubkey required to match the [Metaplex Token Metadata](https://docs.metaplex.com/architecture/contracts#token-metadata) update authority                                                                                                                                                                                              |
| creators                 | Array\<Creator\> | The allowed list of creators for verified NFTs. Creator matches the Metaplex definition of `{address: Pubkey, verified: bool, share: u8}`                                                                                                                                                                                                 |
| collection               | string           | The name of the NFT collection that is allowed to earn rewards. Staked NFTs must have this value as the first part of the name in the [Metaplex Token Metadata](https://docs.metaplex.com/architecture/contracts#token-metadata). For example if the `collection` is `"gmoot"` the an NFT with the name `"gmoot bag #69"` will be allowed |
| enforce_metadata         | bool             | A flag indicating whether or not the [Metaplex Token Metadata](https://docs.metaplex.com/architecture/contracts#token-metadata) is required for the `Stake` instruction. When set to `false` any NFT will be allowed to earn rewards.                                                                                                     |
| total_staked             | u32              | The number of NFTs currently staked to this Rewarder                                                                                                                                                                                                                                                                                      |

#### Creating a Rewarder

A basic typescript client is provided in this repo at `ts/cli.ts` to facilitate the creation and fetching of a Rewarder. From the `ts` directory, you can run `npm i` to install dependencies and then execute the CLI with:

```sh
 npm start -- rewarder create -h

#Output

Usage: cli rewarder create [options]

Options:
  -d, --decimals <number>    The number of decimals for the reward token
  -r, --rewardRate <number>  The number reward per second per nft staked for the rewarder
  -n, --name <string>        The name of the NFT collection the rewarder is for
  -c, --creators <path>      the path to a json array of nft creator objects
  -h, --help                 display help for command
```

### Stake Accounts

The user **Stake Account** is a [PDA](https://docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses) stores the information that is used to calculate the earned rewards for the total number of staked NFTs for the owner. The **Stake Account** holds any locked up NFTs at the [Associated Token Account address](https://spl.solana.com/associated-token-account#finding-the-associated-token-account-address) for the NFT Mint making it easy to list any staked NFTs in the same way you would for another wallet, given the stake account address. The following Typescript code can be used to find the associated token address for the Stake Account:

```typescript
import * as splToken from "@solana/spl-token";

const tokenAccountAddress = await splToken.Token.getAssociatedTokenAddress(
  splToken.ASSOCIATED_TOKEN_PROGRAM_ID,
  splToken.TOKEN_PROGRAM_ID,
  mint, //the pubkey for the NFT Mint
  stakeAccount, // the pubkey for the stake account
  true //allows the owner of the associated token account to be a PDA.
);
```

The Stake Account address is calculated using the following seeds:

```
[collectionName, StakingProgramID, "stake_account", rewarderPubkey, ownerPubkey]
```
