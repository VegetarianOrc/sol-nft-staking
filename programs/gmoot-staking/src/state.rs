use std::mem::size_of;

use anchor_lang::prelude::*;

pub trait Len {
    const LEN: usize;
}

impl<T> Len for T
where
    T: AnchorDeserialize + AnchorSerialize,
{
    const LEN: usize = 8 + size_of::<T>();
}

#[account]
pub struct GmootStakeRewarder {
    pub authority: Pubkey,
    pub reward_mint: Pubkey,
    pub reward_authority_bump: u8,
    /// tokens rewarded per staked NFT per second
    pub reward_rate: u64,
}

#[account]
pub struct GmootStakeAccount {
    pub owner: Pubkey,
    pub rewarder: Pubkey,
    pub num_staked: u16,
    pub bump: u8,
    pub last_claimed: i64,
}
