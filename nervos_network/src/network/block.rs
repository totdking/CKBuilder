use crate::network::transaction::CKBTransaction;
pub struct Block{
    /// Header contains the block’s metadata
    header: Header,
    uncles: Vec<UncleBlock>,
    transactions: Vec<CKBTransaction>,
    /// A list of hex-encoded short transaction IDs of the proposed transactions
    proposals: Vec<Proposal>
}

/// contains the uncle’s header and proposals, but no transactions
pub struct UncleBlock{
    /// The block header of the uncle block.
    pub header: Header,
    /// A list of short transaction IDs proposed by this uncle block
    pub proposal: Vec<Proposal>
}

// Two protocol parameters, close and far, specify the closest and farthest on-chain distances between a transaction's proposal and commitment. A non-cellbase transaction commit in block c must have been proposed in block p, where
// close <= c - p <= far
// In CKB's Mainnet, close is 2 and far is 10. Thus: (2-10) is block proposal window
// 2 <= c - p <= 10
/// A transaction proposal ID is the first 10 bytes of the transaction hash. In CKB, the transaction proposal ID must be proposed before a transaction can be committed to the blockchain.
pub struct Proposal([u8;10]);

/// The header field is part of the Block and UncleBlock structures. It contains metadata that summarizes and secures the block's contents, and it plays a critical role in consensus and chain validation.
pub struct Header {
    raw: RawHeader,
    nonce: u128,
}
/// The payload of the block header.
pub struct RawHeader{
    version: u32,
    compact_target: u32, 
    timestamp: u64,
    number: u64,
    epoch: u64,
    parent_hash: [u8;32],
    root_hash: [u8;32],
    proposal_hash: [u8;32],
    uncles_hash: [u8;32],
    dao: [u8;32],
}

//This following snippet describes the process to validate the PoW for a block header in the Nervos CKB blockchain:

// Serializing and hashing the block's raw data.
// Concatenating the hash with the nonce.
// Running the concatenated result through the Eaglesong algorithm.
// (Optional) Re-hashing for the Testnet.
// Converting the final output to an integer and ensuring it meets the required difficulty target.