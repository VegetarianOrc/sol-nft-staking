use anchor_lang::prelude::*;

pub mod anchor_metaplex;
pub mod errors;
pub mod state;

use anchor_metaplex::MetadataAccount;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::{self, Mint, MintTo, SetAuthority, Token, TokenAccount};
use errors::*;
use spl_token::instruction::AuthorityType;
use state::*;

const REWARDER_PREFIX: &[u8] = b"rewarder";
const ACCOUNT_PREFIX: &[u8] = b"stake_account";

declare_id!("3zPPaZhN3tAkSJhjcEcyT7kAM6b2stQmJf65Fw9sMZa3");

#[program]
pub mod sol_nft_staking {

    use super::*;
    pub fn initialize_rewarder(
        ctx: Context<InitializeRewarder>,
        _rewarder_bump: u8,
        reward_authority_bump: u8,
        reward_rate: u64,
        collection: String,
        creators: Vec<CreatorStruct>,
        nft_update_authority: Pubkey,
        enforce_metadata: bool,
    ) -> ProgramResult {
        let rewarder = &mut ctx.accounts.rewarder;

        rewarder.authority = ctx.accounts.authority.key();
        rewarder.reward_mint = ctx.accounts.reward_mint.key();
        rewarder.reward_authority_bump = reward_authority_bump;
        rewarder.reward_rate = reward_rate;
        rewarder.allowed_update_authority = nft_update_authority;
        rewarder.creators = creators;
        rewarder.collection = collection;
        rewarder.total_staked = 0;
        rewarder.enforce_metadata = enforce_metadata;

        Ok(())
    }

    pub fn update_reward_rate(ctx: Context<UpdateRewardRate>, new_rate: u64) -> ProgramResult {
        let rewarder = &mut ctx.accounts.rewarder;

        rewarder.reward_rate = new_rate;

        Ok(())
    }

    pub fn initialize_stake_account(
        ctx: Context<InitializeStakeAccount>,
        bump: u8,
    ) -> ProgramResult {
        let stake_account = &mut ctx.accounts.stake_account;

        stake_account.owner = ctx.accounts.owner.key();
        stake_account.rewarder = ctx.accounts.rewarder.key();
        stake_account.num_staked = 0;
        stake_account.bump = bump;
        stake_account.last_claimed = 0;

        Ok(())
    }

    pub fn stake_nft(ctx: Context<StakeNft>) -> ProgramResult {
        let owner = &ctx.accounts.owner;
        let rewarder = &mut ctx.accounts.rewarder;
        let stake_account = &mut ctx.accounts.stake_account;
        let reward_mint = &ctx.accounts.reward_mint;
        let reward_autority = &ctx.accounts.reward_authority;
        let reward_token_account = &ctx.accounts.reward_token_account;
        let nft_mint = &ctx.accounts.nft_mint;
        let nft_token_account = &ctx.accounts.nft_token_account;
        // let nft_vault = &ctx.accounts.nft_vault;

        let token_program = &ctx.accounts.token_program;
        let clock = &ctx.accounts.clock;

        if rewarder.enforce_metadata {
            let remaining = ctx.remaining_accounts;
            let metadata = get_metadata_account(remaining)?;
            check_metadata(&metadata, &nft_mint.key(), rewarder)?;
        }

        // Calculate and claim any pending rewards
        let to_reward = calculate_reward(
            rewarder.reward_rate,
            stake_account.num_staked,
            stake_account.last_claimed,
            clock.unix_timestamp,
        );

        transfer_reward(
            to_reward,
            rewarder,
            reward_mint,
            reward_token_account,
            reward_autority,
            token_program,
        )?;
        stake_account.last_claimed = clock.unix_timestamp;

        //increase the number of staked nfts by 1
        stake_account.num_staked += 1;
        rewarder.total_staked += 1;

        //transfer nft ownership to vault
        let authority_accounts = SetAuthority {
            current_authority: owner.to_account_info(),
            account_or_mint: nft_token_account.to_account_info(),
        };
        let authority_ctx = CpiContext::new(token_program.to_account_info(), authority_accounts);
        token::set_authority(
            authority_ctx,
            AuthorityType::AccountOwner,
            Some(stake_account.key()),
        )?;

        Ok(())
    }

    pub fn unstake_nft(ctx: Context<UnstakeNft>) -> ProgramResult {
        let owner = &ctx.accounts.owner;
        let rewarder = &mut ctx.accounts.rewarder;
        let stake_account = &mut ctx.accounts.stake_account;
        let reward_mint = &ctx.accounts.reward_mint;
        let reward_autority = &ctx.accounts.reward_authority;
        let reward_token_account = &ctx.accounts.reward_token_account;
        let nft_token_account = &ctx.accounts.nft_token_account;
        // let nft_vault = &ctx.accounts.nft_vault;

        let token_program = &ctx.accounts.token_program;
        let clock = &ctx.accounts.clock;

        // Calculate and claim any pending rewards

        let to_reward = calculate_reward(
            rewarder.reward_rate,
            stake_account.num_staked,
            stake_account.last_claimed,
            clock.unix_timestamp,
        );

        transfer_reward(
            to_reward,
            rewarder,
            reward_mint,
            reward_token_account,
            reward_autority,
            token_program,
        )?;
        stake_account.last_claimed = clock.unix_timestamp;

        //descrease the number of staked nfts by 1
        stake_account.num_staked = stake_account.num_staked.checked_sub(1).unwrap_or(0);
        rewarder.total_staked = rewarder.total_staked.checked_sub(1).unwrap_or(0);

        let stake_account_seeds = &[
            rewarder.collection.as_bytes(),
            &id().to_bytes(),
            ACCOUNT_PREFIX,
            &rewarder.key().to_bytes(),
            &owner.key().to_bytes(),
            &[stake_account.bump],
        ];

        let stake_account_signer = &[&stake_account_seeds[..]];

        //transfer nft to vault
        let authority_accounts = SetAuthority {
            current_authority: stake_account.to_account_info(),
            account_or_mint: nft_token_account.to_account_info(),
        };
        let authority_ctx = CpiContext::new_with_signer(
            token_program.to_account_info(),
            authority_accounts,
            stake_account_signer,
        );
        token::set_authority(
            authority_ctx,
            AuthorityType::AccountOwner,
            Some(owner.key()),
        )?;

        Ok(())
    }

    pub fn claim(ctx: Context<Claim>) -> ProgramResult {
        let rewarder = &ctx.accounts.rewarder;
        let stake_account = &mut ctx.accounts.stake_account;
        let reward_mint = &ctx.accounts.reward_mint;
        let reward_autority = &ctx.accounts.reward_authority;
        let reward_token_account = &ctx.accounts.reward_account;

        let token_program = &ctx.accounts.token_program;
        let clock = &ctx.accounts.clock;

        let to_reward = calculate_reward(
            rewarder.reward_rate,
            stake_account.num_staked,
            stake_account.last_claimed,
            clock.unix_timestamp,
        );

        transfer_reward(
            to_reward,
            rewarder,
            reward_mint,
            reward_token_account,
            reward_autority,
            token_program,
        )?;
        stake_account.last_claimed = clock.unix_timestamp;

        Ok(())
    }
}

pub fn calculate_reward(
    reward_rate: u64,
    num_staked: u16,
    last_claimed: i64,
    current_time: i64,
) -> u64 {
    if num_staked == 0 {
        return 0;
    }

    let elapsed_time = current_time - last_claimed;

    if elapsed_time <= 0 {
        return 0;
    }

    reward_rate * elapsed_time as u64 * num_staked as u64
}

pub fn transfer_reward<'info>(
    earned_reward: u64,
    rewarder: &Account<'info, NftStakeRewarder>,
    reward_mint: &Account<'info, Mint>,
    reward_account: &Account<'info, TokenAccount>,
    mint_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> ProgramResult {
    let mint_authority_seeds = &[
        rewarder.collection.as_bytes(),
        &id().to_bytes(),
        REWARDER_PREFIX,
        &rewarder.key().to_bytes(),
        &[rewarder.reward_authority_bump],
    ];
    let mint_authority_signer = &[&mint_authority_seeds[..]];
    let mint_accounts = MintTo {
        mint: reward_mint.to_account_info(),
        to: reward_account.to_account_info(),
        authority: mint_authority.to_account_info(),
    };
    let mint_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        mint_accounts,
        mint_authority_signer,
    );
    token::mint_to(mint_ctx, earned_reward)
}

#[derive(Accounts)]
#[instruction(_rewarder_bump: u8, reward_authority_bump: u8, reward_rate: u64, collection: String, creators: Vec<CreatorStruct>)]
pub struct InitializeRewarder<'info> {
    /// The new rewarder account to create
    #[account(
        init,
        space = NftStakeRewarder::calculate_len(creators.len(), &collection),
        payer = authority,
        seeds = [collection.as_bytes(), &id().to_bytes(), REWARDER_PREFIX],
        bump = _rewarder_bump,
    )]
    pub rewarder: Account<'info, NftStakeRewarder>,

    /// The owner of the rewarder account
    #[account(mut, signer)]
    pub authority: AccountInfo<'info>,

    /// PDA used for minting rewards
    #[account(
        seeds = [collection.as_bytes(), &id().to_bytes(), REWARDER_PREFIX, &rewarder.key().to_bytes()],
        bump = reward_authority_bump,
    )]
    pub reward_authority: AccountInfo<'info>,

    /// The SPL Mint of the reward token. Must have the reward authority mint authority
    #[account(
        constraint = reward_mint.mint_authority.contains(&reward_authority.key()) @ StakingError::RewarderNotMintAuthority
    )]
    pub reward_mint: Account<'info, Mint>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UpdateRewardRate<'info> {
    /// The new rewarder account to updtae
    #[account(
        mut,
        has_one = authority @ StakingError::InvalidRewarderAuthority,
    )]
    pub rewarder: Account<'info, NftStakeRewarder>,

    /// The owner of the rewarder account
    #[account(signer)]
    pub authority: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct InitializeStakeAccount<'info> {
    /// The owner of the stake account
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    /// The new stake account to initialize
    #[account(
        init,
        payer = owner,
        space = NftStakeAccount::LEN,
        seeds = [rewarder.collection.as_bytes(), &id().to_bytes(), ACCOUNT_PREFIX, &rewarder.key().to_bytes(), &owner.key().to_bytes()],
        bump = bump,
    )]
    pub stake_account: Account<'info, NftStakeAccount>,

    /// The rewarder associated with this stake account
    pub rewarder: Account<'info, NftStakeRewarder>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
// #[instruction(_vault_bump: u8)]
pub struct StakeNft<'info> {
    /// The owner of the stake account
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    /// The rewarder account for the collection
    #[account(mut)]
    pub rewarder: Box<Account<'info, NftStakeRewarder>>,

    /// PDA that has the authority to mint reward tokens
    #[account(
        seeds = [rewarder.collection.as_bytes(), &id().to_bytes(), REWARDER_PREFIX, &rewarder.key().to_bytes()],
        bump = rewarder.reward_authority_bump,
    )]
    pub reward_authority: AccountInfo<'info>,

    /// The stake account for the owner
    #[account(
        mut,
        has_one = rewarder @ StakingError::InvalidRewarder,
        has_one = owner @ StakingError::InvalidOwnerForStakeAccount,
        seeds = [rewarder.collection.as_bytes(), &id().to_bytes(), ACCOUNT_PREFIX, &rewarder.key().to_bytes(), &owner.key().to_bytes()],
        bump = stake_account.bump,
    )]
    pub stake_account: Account<'info, NftStakeAccount>,

    /// The Mint of the rewarded token
    #[account(
        mut,
        address = rewarder.reward_mint @ StakingError::InvalidRewardMint,
    )]
    pub reward_mint: Box<Account<'info, Mint>>,

    /// The token account from the owner
    #[account(
        mut,
        has_one = owner @ StakingError::InvalidOwnerForRewardToken,
        constraint = reward_token_account.mint == rewarder.reward_mint @ StakingError::InvalidRewardTokenAccount,
    )]
    pub reward_token_account: Account<'info, TokenAccount>,

    /// The Mint of the NFT
    #[account(
        constraint = nft_mint.supply == 1 @ StakingError::InvalidNFTMintSupply,
    )]
    pub nft_mint: Box<Account<'info, Mint>>,

    /// The token account from the owner
    #[account(
        mut,
        has_one = owner @ StakingError::InvalidNFTOwner,
        constraint = nft_token_account.mint == nft_mint.key() @ StakingError::InvalidNFTAccountMint,
        constraint = nft_token_account.amount == 1 @ StakingError::NFTAccountEmpty,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct UnstakeNft<'info> {
    /// The owner of the stake account
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    /// The rewarder account for the collection
    #[account(mut)]
    pub rewarder: Account<'info, NftStakeRewarder>,

    /// PDA that has the authority to mint reward tokens
    #[account(
        seeds = [rewarder.collection.as_bytes(), &id().to_bytes(), REWARDER_PREFIX, &rewarder.key().to_bytes()],
        bump = rewarder.reward_authority_bump,
    )]
    pub reward_authority: AccountInfo<'info>,

    /// The stake account for the owner
    #[account(
        mut,
        has_one = rewarder @ StakingError::InvalidRewarder,
        has_one = owner @ StakingError::InvalidOwnerForStakeAccount,
        seeds = [rewarder.collection.as_bytes(), &id().to_bytes(), ACCOUNT_PREFIX, &rewarder.key().to_bytes(), &owner.key().to_bytes()],
        bump = stake_account.bump,
    )]
    pub stake_account: Account<'info, NftStakeAccount>,

    /// The Mint of the rewarded token
    #[account(
        mut,
        address = rewarder.reward_mint @ StakingError::InvalidRewardMint,
    )]
    pub reward_mint: Box<Account<'info, Mint>>,

    /// The token account from the owner
    #[account(
        mut,
        has_one = owner @ StakingError::InvalidOwnerForRewardToken,
        constraint = reward_token_account.mint == rewarder.reward_mint @ StakingError::InvalidRewardTokenAccount,
    )]
    pub reward_token_account: Account<'info, TokenAccount>,

    /// The Mint of the NFT
    #[account(
        constraint = nft_mint.supply == 1 @ StakingError::InvalidNFTMintSupply,
    )]
    pub nft_mint: Box<Account<'info, Mint>>,

    /// The token account from the owner
    #[account(
        mut,
        constraint = nft_token_account.owner == stake_account.key() @ StakingError::InvalidStakedNFTOwner,
        constraint = nft_token_account.mint == nft_mint.key() @ StakingError::InvalidNFTAccountMint,
        address = get_associated_token_address(&owner.key(), &nft_mint.key()),
    )]
    pub nft_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    /// The owner of the stake account
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    /// The rewarder account for the collection
    #[account()]
    pub rewarder: Account<'info, NftStakeRewarder>,

    /// The stake account for the owner
    #[account(
        mut,
        has_one = rewarder @ StakingError::InvalidRewarder,
        has_one = owner @ StakingError::InvalidOwnerForStakeAccount,
        seeds = [rewarder.collection.as_bytes(), &id().to_bytes(), ACCOUNT_PREFIX, &rewarder.key().to_bytes(), &owner.key().to_bytes()],
        bump = stake_account.bump,
    )]
    pub stake_account: Account<'info, NftStakeAccount>,

    /// The Mint of the rewarded token
    #[account(
        mut,
        address = rewarder.reward_mint @ StakingError::InvalidRewardMint,
    )]
    pub reward_mint: Account<'info, Mint>,

    /// The token account for the reward mint for the owner
    #[account(
        mut,
        has_one = owner @ StakingError::InvalidOwnerForRewardToken,
        constraint = reward_account.mint == rewarder.reward_mint @ StakingError::InvalidRewardTokenAccount,
    )]
    pub reward_account: Account<'info, TokenAccount>,

    /// PDA that has the authority to mint reward tokens
    #[account(
        seeds = [rewarder.collection.as_bytes(), &id().to_bytes(), REWARDER_PREFIX, &rewarder.key().to_bytes()],
        bump = rewarder.reward_authority_bump,
    )]
    pub reward_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn check_metadata<'a, 'b, 'c, 'info>(
    metadata: &'a Account<'info, MetadataAccount>,
    nft_mint_key: &'b Pubkey,
    rewarder: &'c NftStakeRewarder,
) -> std::result::Result<(), ProgramError> {
    let (expected_address, _) = Pubkey::find_program_address(
        &[
            anchor_metaplex::PDAPrefix.as_bytes(),
            &anchor_metaplex::ID.to_bytes(),
            &nft_mint_key.to_bytes(),
        ],
        &anchor_metaplex::ID,
    );

    if metadata.key() != expected_address {
        return Err(StakingError::InvalidMetadataAccountAddress.into());
    }

    if metadata.update_authority != rewarder.allowed_update_authority {
        return Err(StakingError::InvalidMetadataUpdateAuthority.into());
    }

    if !metadata.data.name.starts_with(&rewarder.collection) {
        return Err(StakingError::InvalidMetadataCollectionPrefix.into());
    }

    if let Some(creators) = &metadata.data.creators {
        if creators.len() != rewarder.creators.len() {
            return Err(StakingError::InvalidMetadataCreators.into());
        }

        for creator in creators.iter() {
            let found_match = rewarder
                .creators
                .iter()
                .find(|known_creator| known_creator == creator);
            if found_match.is_none() {
                return Err(StakingError::InvalidMetadataCreators.into());
            }
        }
    } else {
        return Err(StakingError::InvalidMetadataCreators.into());
    }

    Ok(())
}

pub fn get_metadata_account<'a, 'b>(
    accounts: &'a [AccountInfo<'b>],
) -> std::result::Result<Account<'b, MetadataAccount>, StakingError> {
    let accounts_iter = &mut accounts.iter();
    let metadata_info =
        next_account_info(accounts_iter).or(Err(StakingError::MetadataAccountNotFound))?;

    if *metadata_info.owner != anchor_metaplex::ID {
        return Err(StakingError::MetadataAccountNotOwnedByCorrectProgram);
    }

    Ok(Account::try_from_unchecked(&metadata_info)
        .or(Err(StakingError::InvalidMetadataAccountData))?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_reward_calculation() {
        let current_time = 100_i64;
        let reward_rate = 10_u64;
        let last_claimed = 0_i64;
        let mut num_staked = 0;

        //if num staked is 0 always return 0 rewards
        let earned_rewared = calculate_reward(reward_rate, num_staked, last_claimed, current_time);
        assert_eq!(earned_rewared, 0);

        num_staked += 1;
        let earned_rewared = calculate_reward(reward_rate, num_staked, last_claimed, current_time);
        assert_eq!(earned_rewared, 1000);

        //twice the number staked recieves twice the reward
        num_staked += 1;
        let earned_rewared = calculate_reward(reward_rate, num_staked, last_claimed, current_time);
        assert_eq!(earned_rewared, 2000);
    }
}
