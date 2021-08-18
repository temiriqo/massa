// Copyright (c) 2021 MASSA LABS <info@massa.net>

use crate::{
    serialization::{
        array_from_slice, DeserializeCompact, DeserializeVarInt, SerializeCompact, SerializeVarInt,
    },
    BlockId, ModelsError, Slot, BLOCK_ID_SIZE_BYTES,
};
use crypto::{
    hash::{Hash, HASH_SIZE_BYTES},
    signature::{
        verify_signature, PublicKey, Signature, PUBLIC_KEY_SIZE_BYTES, SIGNATURE_SIZE_BYTES,
    },
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub const ENDORSEMENT_ID_SIZE_BYTES: usize = HASH_SIZE_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct EndorsementId(Hash);

impl std::fmt::Display for EndorsementId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0.to_bs58_check())
    }
}

impl FromStr for EndorsementId {
    type Err = ModelsError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(EndorsementId(Hash::from_str(s)?))
    }
}

impl EndorsementId {
    pub fn to_bytes(&self) -> [u8; HASH_SIZE_BYTES] {
        self.0.to_bytes()
    }

    pub fn into_bytes(self) -> [u8; HASH_SIZE_BYTES] {
        self.0.into_bytes()
    }

    pub fn from_bytes(data: &[u8; HASH_SIZE_BYTES]) -> Result<EndorsementId, ModelsError> {
        Ok(EndorsementId(
            Hash::from_bytes(data).map_err(|_| ModelsError::HashError)?,
        ))
    }
    pub fn from_bs58_check(data: &str) -> Result<EndorsementId, ModelsError> {
        Ok(EndorsementId(
            Hash::from_bs58_check(data).map_err(|_| ModelsError::HashError)?,
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Endorsement {
    pub content: EndorsementContent,
    pub signature: Signature,
}

impl Endorsement {
    /// Verify the signature and integrity of the endorsement and computes ID
    pub fn verify_integrity(&self) -> Result<EndorsementId, ModelsError> {
        let content_hash = Hash::hash(&self.content.to_bytes_compact()?);
        verify_signature(
            &content_hash,
            &self.signature,
            &self.content.sender_public_key,
        )?;
        self.get_endorsement_id()
    }

    pub fn get_endorsement_id(&self) -> Result<EndorsementId, ModelsError> {
        Ok(EndorsementId(Hash::hash(&self.to_bytes_compact()?)))
    }
}

impl SerializeCompact for Endorsement {
    fn to_bytes_compact(&self) -> Result<Vec<u8>, ModelsError> {
        let mut res: Vec<u8> = Vec::new();

        // content
        res.extend(self.content.to_bytes_compact()?);

        // signature
        res.extend(&self.signature.to_bytes());

        Ok(res)
    }
}

impl DeserializeCompact for Endorsement {
    fn from_bytes_compact(buffer: &[u8]) -> Result<(Self, usize), ModelsError> {
        let mut cursor = 0;

        // content
        let (content, delta) = EndorsementContent::from_bytes_compact(&buffer[cursor..])?;
        cursor += delta;

        // signature
        let signature = Signature::from_bytes(&array_from_slice(&buffer[cursor..])?)?;
        cursor += SIGNATURE_SIZE_BYTES;

        let res = Endorsement { content, signature };

        Ok((res, cursor))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndorsementContent {
    /// Public key of the endorser.
    pub sender_public_key: PublicKey,
    /// slot of endorsed block
    pub slot: Slot,
    /// endorsement index inside the block
    pub index: u32,
    /// hash of endorsed block
    pub endorsed_block: BlockId,
}

impl SerializeCompact for EndorsementContent {
    fn to_bytes_compact(&self) -> Result<Vec<u8>, ModelsError> {
        let mut res: Vec<u8> = Vec::new();

        // Sender public key
        res.extend(&self.sender_public_key.to_bytes());

        // Slot
        res.extend(self.slot.to_bytes_compact()?);

        // endorsement index inside the block
        res.extend(self.index.to_varint_bytes());

        // hash of endorsed block
        res.extend(&self.endorsed_block.to_bytes());

        Ok(res)
    }
}

impl DeserializeCompact for EndorsementContent {
    fn from_bytes_compact(buffer: &[u8]) -> Result<(Self, usize), ModelsError> {
        let mut cursor = 0usize;

        // sender public key
        let sender_public_key = PublicKey::from_bytes(&array_from_slice(&buffer[cursor..])?)?;
        cursor += PUBLIC_KEY_SIZE_BYTES;

        // slot
        let (slot, delta) = Slot::from_bytes_compact(&buffer[cursor..])?;
        cursor += delta;

        // endorsement index inside the block
        let (index, delta) = u32::from_varint_bytes(&buffer[cursor..])?;
        cursor += delta;

        // hash of endorsed block
        let endorsed_block = BlockId::from_bytes(&array_from_slice(&buffer[cursor..])?)?;
        cursor += BLOCK_ID_SIZE_BYTES;

        Ok((
            EndorsementContent {
                sender_public_key,
                slot,
                index,
                endorsed_block,
            },
            cursor,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_endorsement_serialization() {
        let ctx = crate::SerializationContext {
            max_block_size: 1024 * 1024,
            max_block_operations: 1024,
            parent_count: 3,
            max_peer_list_length: 128,
            max_message_size: 3 * 1024 * 1024,
            max_bootstrap_blocks: 100,
            max_bootstrap_cliques: 100,
            max_bootstrap_deps: 100,
            max_bootstrap_children: 100,
            max_bootstrap_pos_cycles: 1000,
            max_bootstrap_pos_entries: 1000,
            max_ask_blocks_per_message: 10,
            max_operations_per_message: 1024,
            max_endorsements_per_message: 1024,
            max_bootstrap_message_size: 100000000,
            max_block_endorsments: 8,
        };
        crate::init_serialization_context(ctx);

        let sender_priv = crypto::generate_random_private_key();
        let sender_public_key = crypto::derive_public_key(&sender_priv);

        let content = EndorsementContent {
            sender_public_key,
            slot: Slot::new(10, 1),
            index: 0,
            endorsed_block: BlockId(Hash::hash("blk".as_bytes())),
        };
        let hash = Hash::hash(&content.to_bytes_compact().unwrap());
        let signature = crypto::sign(&hash, &sender_priv).unwrap();
        let endorsement = Endorsement {
            content: content.clone(),
            signature,
        };

        let ser_content = content.to_bytes_compact().unwrap();
        let (res_content, _) = EndorsementContent::from_bytes_compact(&ser_content).unwrap();
        assert_eq!(format!("{:?}", res_content), format!("{:?}", content));

        let ser_endorsement = endorsement.to_bytes_compact().unwrap();
        let (res_endorsement, _) = Endorsement::from_bytes_compact(&ser_endorsement).unwrap();
        assert_eq!(
            format!("{:?}", res_endorsement),
            format!("{:?}", endorsement)
        );
    }
}