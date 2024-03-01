use zksync_types::{
    commitment::{pre_boojum_serialize_commitments, serialize_commitments, L1BatchWithMetadata},
    ethabi::Token,
    web3::{contract::Error as Web3ContractError, error::Error as Web3ApiError},
    U256,
};

use crate::Tokenizable;

/// Encoding for `CommitBatchInfo` from `IExecutor.sol`
#[derive(Debug)]
pub struct CommitBatchInfoRollup<'a> {
    pub l1_batch_with_metadata: &'a L1BatchWithMetadata,
}

impl<'a> CommitBatchInfoRollup<'a> {
    pub fn new(l1_batch_with_metadata: &'a L1BatchWithMetadata) -> Self {
        Self {
            l1_batch_with_metadata,
        }
    }
}

impl<'a> Tokenizable for CommitBatchInfoRollup<'a> {
    fn from_token(_token: Token) -> Result<Self, Web3ContractError>
    where
        Self: Sized,
    {
        // Currently there is no need to decode this struct.
        // We still want to implement `Tokenizable` trait for it, so that *once* it's needed
        // the implementation is provided here and not in some other inconsistent way.
        Err(Web3ContractError::Api(Web3ApiError::Decoder(
            "Not implemented".to_string(),
        )))
    }

    fn into_token(self) -> Token {
        if self
            .l1_batch_with_metadata
            .header
            .protocol_version
            .unwrap()
            .is_pre_boojum()
        {
            pre_boojum_into_token(self.l1_batch_with_metadata)
        } else {
            Token::Tuple(rollup_mode_l1_commit_data(self.l1_batch_with_metadata))
        }
    }
}

/// Encoding for `CommitBatchInfo` from `IExecutor.sol`
#[derive(Debug)]
pub struct CommitBatchInfoValidium<'a> {
    pub l1_batch_with_metadata: &'a L1BatchWithMetadata,
}

impl<'a> CommitBatchInfoValidium<'a> {
    pub fn new(l1_batch_with_metadata: &'a L1BatchWithMetadata) -> Self {
        Self {
            l1_batch_with_metadata,
        }
    }
}

impl<'a> Tokenizable for CommitBatchInfoValidium<'a> {
    fn from_token(_token: Token) -> Result<Self, Web3ContractError>
    where
        Self: Sized,
    {
        // Currently there is no need to decode this struct.
        // We still want to implement `Tokenizable` trait for it, so that *once* it's needed
        // the implementation is provided here and not in some other inconsistent way.
        Err(Web3ContractError::Api(Web3ApiError::Decoder(
            "Not implemented".to_string(),
        )))
    }

    fn into_token(self) -> Token {
        if self
            .l1_batch_with_metadata
            .header
            .protocol_version
            .unwrap()
            .is_pre_boojum()
        {
            pre_boojum_into_token(self.l1_batch_with_metadata)
        } else {
            Token::Tuple(validium_mode_l1_commit_data(self.l1_batch_with_metadata))
        }
    }
}

fn pre_boojum_into_token(l1_batch_commit_with_metadata: &L1BatchWithMetadata) -> Token {
    let header = &l1_batch_commit_with_metadata.header;
    let metadata = &l1_batch_commit_with_metadata.metadata;
    Token::Tuple(vec![
        Token::Uint(U256::from(header.number.0)),
        Token::Uint(U256::from(header.timestamp)),
        Token::Uint(U256::from(metadata.rollup_last_leaf_index)),
        Token::FixedBytes(metadata.merkle_root_hash.as_bytes().to_vec()),
        Token::Uint(U256::from(header.l1_tx_count)),
        Token::FixedBytes(metadata.l2_l1_merkle_root.as_bytes().to_vec()),
        Token::FixedBytes(header.priority_ops_onchain_data_hash().as_bytes().to_vec()),
        Token::Bytes(metadata.initial_writes_compressed.clone().unwrap()),
        Token::Bytes(metadata.repeated_writes_compressed.clone().unwrap()),
        Token::Bytes(pre_boojum_serialize_commitments(&header.l2_to_l1_logs)),
        Token::Array(
            header
                .l2_to_l1_messages
                .iter()
                .map(|message| Token::Bytes(message.to_vec()))
                .collect(),
        ),
        Token::Array(
            l1_batch_commit_with_metadata
                .raw_published_factory_deps
                .iter()
                .map(|bytecode| Token::Bytes(bytecode.to_vec()))
                .collect(),
        ),
    ])
}

fn encode_l1_commit(l1_batch_with_metadata: &L1BatchWithMetadata, pubdata: Token) -> Vec<Token> {
    let header = &l1_batch_with_metadata.header;
    let metadata = &l1_batch_with_metadata.metadata;
    let commit_data = vec![
        // `batchNumber`
        Token::Uint(U256::from(header.number.0)),
        // `timestamp`
        Token::Uint(U256::from(header.timestamp)),
        // `indexRepeatedStorageChanges`
        Token::Uint(U256::from(metadata.rollup_last_leaf_index)),
        // `newStateRoot`
        Token::FixedBytes(metadata.merkle_root_hash.as_bytes().to_vec()),
        // `numberOfLayer1Txs`
        Token::Uint(U256::from(header.l1_tx_count)),
        // `priorityOperationsHash`
        Token::FixedBytes(header.priority_ops_onchain_data_hash().as_bytes().to_vec()),
        // `bootloaderHeapInitialContentsHash`
        Token::FixedBytes(
            metadata
                .bootloader_initial_content_commitment
                .unwrap()
                .as_bytes()
                .to_vec(),
        ),
        // `eventsQueueStateHash`
        Token::FixedBytes(
            metadata
                .events_queue_commitment
                .unwrap()
                .as_bytes()
                .to_vec(),
        ),
        // `systemLogs`
        Token::Bytes(serialize_commitments(&header.system_logs)),
        pubdata,
    ];
    commit_data
}

fn validium_mode_l1_commit_data(l1_batch_with_metadata: &L1BatchWithMetadata) -> Vec<Token> {
    encode_l1_commit(l1_batch_with_metadata, Token::Bytes(vec![]))
}

fn rollup_mode_l1_commit_data(l1_batch_with_metadata: &L1BatchWithMetadata) -> Vec<Token> {
    encode_l1_commit(
        l1_batch_with_metadata,
        Token::Bytes(L1BatchWithMetadata::construct_pubdata(
            l1_batch_with_metadata,
        )),
    )
}
