use anchor_lang::prelude::*;

pub mod anchor_metaplex;
pub mod state;

use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use state::*;

const GMOOT_PREFIX: &[u8] = b"gmoot";
const REWARDER_PREFIX: &[u8] = b"rewarder";
const ACCOUNT_PREFIX: &[u8] = b"stake_account";

// const GMOOT_UPDATE_AUTHORITY: &str = "2MUpR2xj5FjzL13NiZa852nzwtNTb1FKVf1ERKSvZKd8";
// const GMOOT_CREATORS: &[&str] = &[
//     "8mxiQyfXpWdohutWgq652XQ5LT4AaX4Lf5c4gZsdNLfd",
//     "2MUpR2xj5FjzL13NiZa852nzwtNTb1FKVf1ERKSvZKd8",
// ];

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod gmoot_staking {
    use anchor_spl::token::{self, Transfer};

    use super::*;
    pub fn initialize_rewarder(
        ctx: Context<InitializeRewarder>,
        reward_authority_bump: u8,
        reward_rate: u64,
    ) -> ProgramResult {
        let rewarder = &mut ctx.accounts.rewarder;

        rewarder.authority = ctx.accounts.authority.key();
        rewarder.reward_mint = ctx.accounts.reward_mint.key();
        rewarder.reward_authority_bump = reward_authority_bump;
        rewarder.reward_rate = reward_rate;

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

    pub fn stake_gmoot(ctx: Context<StakeGmoot>) -> ProgramResult {
        let owner = &ctx.accounts.owner;
        let rewarder = &ctx.accounts.rewarder;
        let stake_account = &mut ctx.accounts.stake_account;
        let reward_mint = &ctx.accounts.reward_mint;
        let reward_autority = &ctx.accounts.reward_authority;
        let reward_token_account = &ctx.accounts.reward_token_account;
        let nft_token_account = &ctx.accounts.nft_token_account;
        let nft_vault = &ctx.accounts.nft_vault;

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

        //increase the number of staked nfts by 1
        stake_account.num_staked += 1;

        //transfer nft to vault
        let tx_accounts = Transfer {
            from: nft_token_account.to_account_info(),
            to: nft_vault.to_account_info(),
            authority: owner.to_account_info(),
        };
        let tx_ctx = CpiContext::new(token_program.to_account_info(), tx_accounts);
        token::transfer(tx_ctx, 1)?;

        Ok(())
    }

    pub fn unstake_gmoot(ctx: Context<UnstakeGmoot>) -> ProgramResult {
        let owner = &ctx.accounts.owner;
        let rewarder = &ctx.accounts.rewarder;
        let stake_account = &mut ctx.accounts.stake_account;
        let reward_mint = &ctx.accounts.reward_mint;
        let reward_autority = &ctx.accounts.reward_authority;
        let reward_token_account = &ctx.accounts.reward_token_account;
        let nft_token_account = &ctx.accounts.nft_token_account;
        let nft_vault = &ctx.accounts.nft_vault;

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

        let stake_account_seeds = &[
            GMOOT_PREFIX,
            ACCOUNT_PREFIX,
            &owner.key.to_bytes(),
            &[stake_account.bump],
        ];

        let stake_account_signer = &[&stake_account_seeds[..]];

        //transfer nft to vault
        let tx_accounts = Transfer {
            from: nft_vault.to_account_info(),
            to: nft_token_account.to_account_info(),
            authority: stake_account.to_account_info(),
        };
        let tx_ctx = CpiContext::new_with_signer(
            token_program.to_account_info(),
            tx_accounts,
            stake_account_signer,
        );
        token::transfer(tx_ctx, 1)?;

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
    rewarder: &Account<'info, GmootStakeRewarder>,
    reward_mint: &Account<'info, Mint>,
    reward_account: &Account<'info, TokenAccount>,
    mint_authority: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
) -> ProgramResult {
    let mint_authority_seeds = &[
        GMOOT_PREFIX,
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
#[instruction(reward_authority_bump: u8)]
pub struct InitializeRewarder<'info> {
    /// The new rewarder account to create
    #[account(
        init,
        space = GmootStakeRewarder::LEN,
        payer = authority,
    )]
    pub rewarder: Account<'info, GmootStakeRewarder>,

    /// The owner of the rewarder account
    #[account(mut, signer)]
    pub authority: AccountInfo<'info>,

    /// PDA used for minting rewards
    #[account(
        seeds = [GMOOT_PREFIX, REWARDER_PREFIX, &rewarder.key().to_bytes()],
        bump = reward_authority_bump,
    )]
    pub reward_authority: AccountInfo<'info>,

    /// The SPL Mint of the reward token. Must have the reward authority mint authority
    #[account(
        constraint = reward_mint.mint_authority.contains(&reward_authority.key())
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
        has_one = authority,
    )]
    pub rewarder: Account<'info, GmootStakeRewarder>,

    /// The owner of the rewarder account
    #[account(mut, signer)]
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
        space = GmootStakeAccount::LEN,
        seeds = [GMOOT_PREFIX, ACCOUNT_PREFIX, &owner.key.to_bytes()],
        bump = bump,
    )]
    pub stake_account: Account<'info, GmootStakeAccount>,

    /// The rewarder associated with this stake account
    pub rewarder: Account<'info, GmootStakeRewarder>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct StakeGmoot<'info> {
    /// The owner of the stake account
    #[account(mut, signer)]
    pub owner: AccountInfo<'info>,

    /// The rewarder account for the collection
    #[account()]
    pub rewarder: Account<'info, GmootStakeRewarder>,

    /// PDA that has the authority to mint reward tokens
    #[account(
        seeds = [GMOOT_PREFIX, REWARDER_PREFIX, &rewarder.key().to_bytes()],
        bump = rewarder.reward_authority_bump,
    )]
    pub reward_authority: AccountInfo<'info>,

    /// The stake account for the owner
    #[account(
        mut,
        has_one = rewarder,
        seeds = [GMOOT_PREFIX, ACCOUNT_PREFIX, &owner.key.to_bytes()],
        bump = stake_account.bump,
    )]
    pub stake_account: Account<'info, GmootStakeAccount>,

    /// The Mint of the rewarded token
    #[account(
        address = rewarder.reward_mint,
    )]
    pub reward_mint: Account<'info, Mint>,

    /// The token account from the owner
    #[account(
        mut,
        has_one = owner,
        constraint = reward_token_account.mint == rewarder.reward_mint,
    )]
    pub reward_token_account: Account<'info, TokenAccount>,

    /// The Mint of the NFT
    pub nft_mint: Account<'info, Mint>,

    /// The token account from the owner
    #[account(
        mut,
        has_one = owner,
        constraint = nft_token_account.mint == nft_mint.key(),
        constraint = nft_token_account.amount == 1,
    )]
    pub nft_token_account: Account<'info, TokenAccount>,

    // /// The metaplex metadata for the NFT, used to determine that the NFT is a GMoot
    // #[account(
    //     seeds = [&anchor_metaplex::PDAPrefix.as_bytes(), &anchor_metaplex::ID.to_bytes()[..], &nft_mint.key().to_bytes()],
    //     bump = metadata_bump,
    //     constraint = check_metadata(&*nft_metadata),
    //     constraint = nft_metadata.mint == nft_mint.key(),
    // )]
    // pub nft_metadata: Account<'info, MetadataAccount>,
    //
    //
    /// The account to hold the NFT while staked
    #[account(
        init,
        token::mint = nft_mint,
        token::authority = stake_account,
        payer = owner,
        address = get_associated_token_address(&stake_account.key(), &nft_mint.key()),
    )]
    pub nft_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct UnstakeGmoot<'info> {
    /// The owner of the stake account
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    /// The rewarder account for the collection
    #[account()]
    pub rewarder: Account<'info, GmootStakeRewarder>,

    /// PDA that has the authority to mint reward tokens
    #[account(
        seeds = [GMOOT_PREFIX, REWARDER_PREFIX, &rewarder.key().to_bytes()],
        bump = rewarder.reward_authority_bump,
    )]
    pub reward_authority: AccountInfo<'info>,

    /// The stake account for the owner
    #[account(
        mut,
        has_one = rewarder,
        seeds = [GMOOT_PREFIX, ACCOUNT_PREFIX, &owner.key.to_bytes()],
        bump = stake_account.bump,
    )]
    pub stake_account: Account<'info, GmootStakeAccount>,

    /// The Mint of the rewarded token
    #[account(
        address = rewarder.reward_mint,
    )]
    pub reward_mint: Account<'info, Mint>,

    /// The token account from the owner
    #[account(
        mut,
        has_one = owner,
        constraint = reward_token_account.mint == rewarder.reward_mint,
    )]
    pub reward_token_account: Account<'info, TokenAccount>,

    /// The Mint of the NFT
    pub nft_mint: Account<'info, Mint>,

    /// The token account from the owner
    #[account(
        mut,
        has_one = owner,
        constraint = nft_token_account.mint == nft_mint.key(),
        address = get_associated_token_address(&owner.key(), &nft_mint.key()),
    )]
    pub nft_token_account: Account<'info, TokenAccount>,

    /// The account to holding the NFT while staked
    #[account(
        mut,
        address = get_associated_token_address(&stake_account.key(), &nft_mint.key()),
    )]
    pub nft_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct Claim<'info> {
    /// The owner of the stake account
    #[account(signer)]
    pub owner: AccountInfo<'info>,

    /// The rewarder account for the collection
    #[account()]
    pub rewarder: Account<'info, GmootStakeRewarder>,

    /// The stake account for the owner
    #[account(
        mut,
        has_one = rewarder,
        seeds = [GMOOT_PREFIX, ACCOUNT_PREFIX, &owner.key.to_bytes()],
        bump = stake_account.bump,
    )]
    pub stake_account: Account<'info, GmootStakeAccount>,

    /// The Mint of the rewarded token
    #[account(
        address = rewarder.reward_mint,
    )]
    pub reward_mint: Account<'info, Mint>,

    /// The token account for the reward mint for the owner
    #[account(
        mut,
        has_one = owner,
        constraint = reward_account.mint == rewarder.reward_mint,
    )]
    pub reward_account: Account<'info, TokenAccount>,

    /// PDA that has the authority to mint reward tokens
    #[account(
        seeds = [GMOOT_PREFIX, REWARDER_PREFIX, &rewarder.key().to_bytes()],
        bump = rewarder.reward_authority_bump,
    )]
    pub reward_authority: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub clock: Sysvar<'info, Clock>,
}

// pub fn check_metadata(metadata: &MetadataAccount) -> bool {
//     if metadata.update_authority.to_string() != String::from(GMOOT_UPDATE_AUTHORITY) {
//         return false;
//     }

//     if !metadata.data.name.starts_with("gmoot bag") {
//         return false;
//     }

//     if metadata.data.seller_fee_basis_points != 100 {
//         return false;
//     }

//     if let Some(creators) = &metadata.data.creators {
//         if creators.len() != 2 {
//             return false;
//         }

//         for creator in creators.iter() {
//             if !GMOOT_CREATORS.contains(&creator.address.to_string().as_str()) {
//                 return false;
//             }
//         }
//     } else {
//         return false;
//     }

//     true
// }

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
