use anchor_lang::prelude::*;

#[error]
pub enum StakingError {
    #[msg("The provided reward mint doesn't have the correct minting authority")]
    RewarderNotMintAuthority,

    #[msg("The provided authority is not valid for the rewarder")]
    InvalidRewarderAuthority,

    #[msg("The provided rewarder does not match the stake account")]
    InvalidRewarder,

    #[msg("The provided owner does not own the stake account")]
    InvalidOwnerForStakeAccount,

    #[msg("The provided Mint is not valid for the provided Rewarder")]
    InvalidRewardMint,

    #[msg("The provided reward token account is not owned by the provided owner")]
    InvalidOwnerForRewardToken,

    #[msg("The provided reward token account is not for the reward token mint")]
    InvalidRewardTokenAccount,

    #[msg("The provided NFT Mint has a supply that isn't 1")]
    InvalidNFTMintSupply,

    #[msg("The provided NFT token account is not owned by the provided owner")]
    InvalidNFTOwner,

    #[msg("The provided NFT token account is not for the NFT mint")]
    InvalidNFTAccountMint,

    #[msg("The provided NFT token account does not have the token")]
    NFTAccountEmpty,

    #[msg("The provided NFT vault token account is not the associated token account for the provided NFT mint and stake account")]
    InvalidNFTVaultAddress,

    #[msg("The provided NFT vault token account does not have the token")]
    NFTVaultEmpty,
}
