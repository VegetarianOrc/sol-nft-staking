use std::ops::{Deref, DerefMut};

use anchor_lang::prelude::{ProgramError, Pubkey};
use anchor_lang::solana_program::borsh::try_from_slice_unchecked;
use metaplex_token_metadata::state::{Key as MetaplexKey, Metadata, MAX_METADATA_LEN};
use metaplex_token_metadata::utils::try_from_slice_checked;

pub use metaplex_token_metadata::state::PREFIX as PDAPrefix;
pub use metaplex_token_metadata::ID;

#[derive(Clone)]
pub struct MetaplexTokenMetadata;

impl anchor_lang::AccountDeserialize for MetaplexTokenMetadata {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        MetaplexTokenMetadata::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(_buf: &mut &[u8]) -> Result<Self, ProgramError> {
        Ok(MetaplexTokenMetadata)
    }
}

impl anchor_lang::Id for MetaplexTokenMetadata {
    fn id() -> Pubkey {
        ID
    }
}

#[derive(Clone)]
pub struct MetadataAccount(Metadata);

impl MetadataAccount {
    pub const LEN: usize = MAX_METADATA_LEN;
}

impl anchor_lang::AccountDeserialize for MetadataAccount {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        try_from_slice_checked(buf, MetaplexKey::MetadataV1, MAX_METADATA_LEN).map(MetadataAccount)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self, ProgramError> {
        let metadata: Metadata = try_from_slice_unchecked(buf)
            .map_err(|err| ProgramError::BorshIoError(err.to_string()))?;
        Ok(MetadataAccount(metadata))
    }
}

impl anchor_lang::AccountSerialize for MetadataAccount {
    fn try_serialize<W: std::io::Write>(&self, _writer: &mut W) -> Result<(), ProgramError> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::Owner for MetadataAccount {
    fn owner() -> Pubkey {
        ID
    }
}

impl Deref for MetadataAccount {
    type Target = Metadata;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MetadataAccount {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
