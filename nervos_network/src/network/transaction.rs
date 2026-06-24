use anyhow::{Ok, Result, anyhow};
use blake2b_rs::Blake2bBuilder;
use molecule::prelude::*;
use secp256k1::{
    self, Message, Secp256k1, SecretKey,
    ecdsa::{RecoverableSignature, RecoveryId},
};
use serde_json;

use crate::data::{Account, CkbCell, CkbScript};
use crate::schemas::{
    Byte32, Bytes, BytesOpt, BytesVec, CellDep as MolCellDep, CellDepVec,
    CellInput as MolCellInput, CellInputVec, CellOutput as MolCellOutput, CellOutputVec,
    OutPoint as MolOutpoint, RawTransactionBuilder, ScriptOpt, TransactionBuilder, Uint32,
    WitnessArgs as MolWitnessArgs, *,
};
use serde::{Deserialize, Serialize};

/// Transaction structure of ckb
#[derive(Debug, Serialize, Deserialize)]
pub struct CKBTransaction {
    pub version: u32,
    pub cell_deps: Vec<CellDep>,
    #[serde(with = "crate::cli::hex_serde::array32")]
    pub header_deps: [u8; 32],
    pub inputs: Vec<CellInput>,
    pub witnesses: Vec<WitnessArgs>,
    pub outputs: Vec<CellOutput>,
    #[serde(with = "crate::cli::hex_serde::vec_bytes")]
    pub output_data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessArgs {
    #[serde(with = "crate::cli::hex_serde::opt_vec_bytes")]
    pub lock: Option<Vec<u8>>,
    #[serde(with = "crate::cli::hex_serde::opt_vec_bytes")]
    pub input_type: Option<Vec<u8>>,
    #[serde(with = "crate::cli::hex_serde::opt_vec_bytes")]
    pub output_type: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CellDep {
    pub outpoint: OutPoint,
    /// 0(code) or 1(dep_group)
    pub dep_type: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CellInput {
    pub previous_outpoint: OutPoint,
    pub since: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CellOutput {
    pub capacity: u64,
    pub lock_script: CkbScript,
    pub type_script: Option<CkbScript>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct OutPoint {
    #[serde(with = "crate::cli::hex_serde::array32")]
    pub tx_hash: [u8; 32],
    pub index: u32,
}

#[allow(dead_code)]
pub enum TxState {
    Pending,
    Confirmed,
    Confirming,
    Conflicting,
    Conflictive,
    Reverted,
    Abandoned,
}

#[allow(dead_code)]
impl CKBTransaction {
    /// Helper function to return transaction builder
    pub fn transaction_builder(&self) -> TransactionBuilder {
        TransactionBuilder::default()
            .raw(
                RawTransactionBuilder::default()
                    .version(Uint32::from_slice(&self.version.to_le_bytes()).unwrap())
                    .cell_deps(
                        CellDepVec::new_builder()
                            .extend(self.cell_deps.iter().map(|i| i.pack()))
                            .build(),
                    )
                    .header_deps(Byte32::from_slice(&self.header_deps).unwrap())
                    .inputs(
                        CellInputVec::new_builder()
                            .extend(self.inputs.iter().map(|i| i.pack()))
                            .build(),
                    )
                    .outputs(
                        CellOutputVec::new_builder()
                            .extend(self.outputs.iter().map(|i| i.pack()))
                            .build(),
                    )
                    .outputs_data(
                        BytesVec::new_builder()
                            .push(Bytes::from(self.output_data.clone()))
                            .build(),
                    )
                    .build(),
            )
            .witnesses(
                BytesVec::new_builder()
                    .extend(
                        self.witnesses
                            .iter()
                            .map(|w| Bytes::from(w.pack().as_slice().to_vec())),
                    )
                    .build(),
            )
    }

    /// Personalized blake2b hash of serialized tx proof excluding witness
    ///
    /// Creates raw tx_hash excluding witnesses
    pub fn hash(&self) -> [u8; 32] {
        // serialize tx body excluding witness as it will be used in the sighash creation
        // Build a mol_tx using the raw generated .mol file from the schema
        let raw_tx = RawTransactionBuilder::default()
            .version(Uint32::from_slice(&self.version.to_le_bytes()).unwrap())
            .cell_deps(
                CellDepVec::new_builder()
                    .extend(self.cell_deps.iter().map(|i| i.pack()))
                    .build(),
            )
            .header_deps(Byte32::from_slice(&self.header_deps).unwrap())
            .inputs(
                CellInputVec::new_builder()
                    .extend(self.inputs.iter().map(|i| i.pack()))
                    .build(),
            )
            .outputs(
                CellOutputVec::new_builder()
                    .extend(self.outputs.iter().map(|i| i.pack()))
                    .build(),
            )
            .outputs_data(
                BytesVec::new_builder()
                    .push(Bytes::from(self.output_data.clone()))
                    .build(),
            )
            .build();

        // Blake2b personal hashing
        let mut hasher = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        let mut hash = [0u8; 32];
        hasher.update(raw_tx.as_slice());
        hasher.finalize(&mut hash);
        return hash;
    }

    /// creates the message from hashing the raw_tx_hash || witnesses to pass to create-transaction
    pub fn create_sighash(&self) -> [u8; 32] {
        let witnesses = &self.witnesses;
        // display the raw tx hash from self.hash
        let raw_tx_hash = self.hash();

        // start the hasher with the raw_tx_hash
        let mut hasher = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        hasher.update(&raw_tx_hash);

        // iterate through the witnesses
        for (i, witness) in witnesses.iter().enumerate() {
            let witness_bytes = if i == 0 {
                let mut signed_witness = witness.clone();
                signed_witness.lock = Some(vec![0u8; 65]);
                signed_witness.pack().as_bytes()
            } else {
                witness.pack().as_bytes()
            };
            let len = (witness_bytes.len() as u64).to_le_bytes();
            hasher.update(&len);
            hasher.update(&witness_bytes);
        }

        // finalize the hasher to the sig-hash
        let mut sig_hash = [0u8; 32];
        hasher.finalize(&mut sig_hash);
        return sig_hash;
    }

    /// private_key.sign_recoverable.(sig_hash) this signs the message created by the sig_hash()
    pub fn create_signature(&self, private_key: SecretKey) -> [u8; 65] {
        let sig_hash = self.create_sighash();

        // Create a secp256k1 context
        let secp = Secp256k1::new();

        // wrap the sig-hash in a message wrapper
        let msg = Message::from_digest(sig_hash);

        // The 65 bytes break down as:
        //   [  r (32 bytes)  |  s (32 bytes)  |  v (1 byte)  ]
        //        64 bytes of secp256k1 signature      recovery id
        //   - r and s are the two 32-byte components of a standard secp256k1 ECDSA signature
        //   - v (1 byte) is the recovery id — it lets a verifier reconstruct your public key from the signature alone, without you having
        //   to include it explicitly. This is what makes it a recoverable signature.
        let sig = secp.sign_ecdsa_recoverable(msg, &private_key);
        let (recovery_id, sig_bytes) = sig.serialize_compact();
        let mut signature = [0u8; 65];
        signature[0..64].copy_from_slice(&sig_bytes);
        signature[64] = recovery_id as u8;
        return signature;
    }

    pub fn sign(&mut self, account: &Account, input_cells: &[CkbCell]) -> Result<Transaction> {
        for cell in input_cells {
            if !cell.can_unlock_script(account) {
                return Err(anyhow!("account does not own any one of the input cells"));
            }
        }
        let private_key = SecretKey::from_byte_array(account.private_key)?;
        let signature = self.create_signature(private_key);

        // embed the real signature into the witness[0].lock
        self.witnesses[0].lock = Some(signature.to_vec());

        // Return the Transaction
        Ok(self.transaction_builder().build())
    }

    /// Validates a spend: structural checks + cryptographic signature recovery.
    ///
    /// Checks in order:
    /// 1. since == 0 (no time lock)
    /// 2. witness slot exists for input_index
    /// 3. witnesses[input_index].lock is Some (tx is signed)
    /// 4. signature is 65 bytes
    /// 5. recovered pubkey hash matches cell.lock_script.args (correct signer)
    pub fn validate_spend(&self, input_index: usize, cells: &[CkbCell]) -> Result<()> {
        let since = self.inputs[input_index].since;
        if since != 0 {
            return Err(anyhow!(
                "input {} has a since lock ({})",
                input_index,
                since
            ));
        }

        if input_index >= self.witnesses.len() {
            return Err(anyhow!("no witness slot for input {}", input_index));
        }

        let sig_bytes = self.witnesses[input_index]
            .lock
            .as_ref()
            .ok_or_else(|| anyhow!("witnesses[{}].lock is empty — tx is unsigned", input_index))?;

        if sig_bytes.len() != 65 {
            return Err(anyhow!(
                "witnesses[{}].lock is {} bytes, expected 65",
                input_index,
                sig_bytes.len()
            ));
        }

        if input_index >= cells.len() {
            return Err(anyhow!("no cell provided for input {}", input_index));
        }

        let recovery_id = RecoveryId::from_u8_masked(sig_bytes[64]);
        let rec_sig = RecoverableSignature::from_compact(&sig_bytes[..64], recovery_id)
            .map_err(|e| anyhow!("malformed signature: {}", e))?;
        let msg = Message::from_digest(self.create_sighash());
        let secp = Secp256k1::new();
        let recovered_pubkey = secp
            .recover_ecdsa(msg, &rec_sig)
            .map_err(|e| anyhow!("signature recovery failed: {}", e))?;

        let ser_pubkey = recovered_pubkey.serialize();
        let mut hasher = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        hasher.update(&ser_pubkey);
        let mut full_hash = [0u8; 32];
        hasher.finalize(&mut full_hash);
        let mut recovered_hash = [0u8; 20];
        recovered_hash.copy_from_slice(&full_hash[..20]);

        let expected = cells[input_index].lock_args();
        if recovered_hash != expected {
            return Err(anyhow!(
                "wrong signer: recovered pubkey hash {} does not match lock args {}",
                hex::encode(recovered_hash),
                hex::encode(expected)
            ));
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn broadcast(&self, _transaction: Transaction) {
        todo!()
    }

    /// Computes the raw transaction hash compatible with the real CKB node.
    ///
    /// The auto-generated Molecule schema in this project stores `header_deps` as a single
    /// `Byte32` (32 bytes), but the real CKB spec uses `Byte32Vec` (empty = 4 bytes: `[0,0,0,0]`).
    /// This method assembles the correct Molecule Table layout manually so the hash matches
    /// what the CKB node computes.
    pub fn rpc_raw_tx_hash(&self) -> [u8; 32] {
        let version_mol = Uint32::from_slice(&self.version.to_le_bytes()).unwrap();
        let cell_deps_mol = CellDepVec::new_builder()
            .extend(self.cell_deps.iter().map(|d| d.pack()))
            .build();
        // Correct empty Byte32Vec: item_count=0 → [00 00 00 00]
        let header_deps_mol: [u8; 4] = [0u8, 0, 0, 0];
        let inputs_mol = CellInputVec::new_builder()
            .extend(self.inputs.iter().map(|i| i.pack()))
            .build();
        let outputs_mol = CellOutputVec::new_builder()
            .extend(self.outputs.iter().map(|o| o.pack()))
            .build();
        // One empty Bytes per output (no cell data for simple transfers)
        let outputs_data_mol = BytesVec::new_builder()
            .extend(self.outputs.iter().map(|_| Bytes::new_builder().build()))
            .build();

        let fields: &[&[u8]] = &[
            version_mol.as_slice(),
            cell_deps_mol.as_slice(),
            &header_deps_mol,
            inputs_mol.as_slice(),
            outputs_mol.as_slice(),
            outputs_data_mol.as_slice(),
        ];

        // Molecule Table layout: [total_size u32] [n×offset u32] [field_data...]
        let header_size = (fields.len() + 1) * 4;
        let data_size: usize = fields.iter().map(|f| f.len()).sum();
        let total_size = header_size + data_size;

        let mut raw_tx = Vec::with_capacity(total_size);
        raw_tx.extend_from_slice(&(total_size as u32).to_le_bytes());
        let mut offset = header_size;
        for f in fields {
            raw_tx.extend_from_slice(&(offset as u32).to_le_bytes());
            offset += f.len();
        }
        for f in fields {
            raw_tx.extend_from_slice(f);
        }

        let mut hasher = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        let mut hash = [0u8; 32];
        hasher.update(&raw_tx);
        hasher.finalize(&mut hash);
        hash
    }

    /// CKB sighash_all using the real-network-compatible raw tx hash.
    pub fn create_rpc_sighash(&self) -> [u8; 32] {
        let raw_tx_hash = self.rpc_raw_tx_hash();
        let mut hasher = Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        hasher.update(&raw_tx_hash);

        for (i, witness) in self.witnesses.iter().enumerate() {
            let witness_bytes = if i == 0 {
                let mut signed_witness = witness.clone();
                signed_witness.lock = Some(vec![0u8; 65]);
                signed_witness.pack().as_bytes()
            } else {
                witness.pack().as_bytes()
            };
            let len = (witness_bytes.len() as u64).to_le_bytes();
            hasher.update(&len);
            hasher.update(&witness_bytes);
        }

        let mut sig_hash = [0u8; 32];
        hasher.finalize(&mut sig_hash);
        sig_hash
    }

    /// Produces a 65-byte recoverable signature valid for the real CKB network.
    pub fn create_rpc_signature(&self, private_key: SecretKey) -> [u8; 65] {
        let sig_hash = self.create_rpc_sighash();
        let secp = Secp256k1::new();
        let msg = Message::from_digest(sig_hash);
        let sig = secp.sign_ecdsa_recoverable(msg, &private_key);
        let (recovery_id, sig_bytes) = sig.serialize_compact();
        let mut signature = [0u8; 65];
        signature[0..64].copy_from_slice(&sig_bytes);
        signature[64] = recovery_id as u8;
        signature
    }

    /// Serializes the transaction into the JSON format expected by the CKB node's RPC.
    pub fn to_rpc_value(&self) -> serde_json::Value {
        let hash_type_str = |ht: u8| match ht {
            0 => "data",
            1 => "type",
            2 => "data1",
            4 => "data2",
            _ => "type",
        };

        let script_json = |s: &CkbScript| {
            serde_json::json!({
                "code_hash": format!("0x{}", hex::encode(s.code_hash)),
                "hash_type": hash_type_str(s.hash_type),
                "args": format!("0x{}", hex::encode(s.args)),
            })
        };

        let cell_deps: Vec<serde_json::Value> = self
            .cell_deps
            .iter()
            .map(|dep| {
                serde_json::json!({
                    "out_point": {
                        "tx_hash": format!("0x{}", hex::encode(dep.outpoint.tx_hash)),
                        "index": format!("0x{:x}", dep.outpoint.index),
                    },
                    "dep_type": if dep.dep_type == 1 { "dep_group" } else { "code" },
                })
            })
            .collect();

        let inputs: Vec<serde_json::Value> = self
            .inputs
            .iter()
            .map(|inp| {
                serde_json::json!({
                    "previous_output": {
                        "tx_hash": format!("0x{}", hex::encode(inp.previous_outpoint.tx_hash)),
                        "index": format!("0x{:x}", inp.previous_outpoint.index),
                    },
                    "since": format!("0x{:x}", inp.since),
                })
            })
            .collect();

        let outputs: Vec<serde_json::Value> = self
            .outputs
            .iter()
            .map(|out| {
                let type_val = out.type_script.as_ref().map(|s| script_json(s));
                serde_json::json!({
                    "capacity": format!("0x{:x}", out.capacity),
                    "lock": script_json(&out.lock_script),
                    "type": type_val,
                })
            })
            .collect();

        // One empty data entry per output
        let outputs_data: Vec<&str> = self.outputs.iter().map(|_| "0x").collect();

        // Witnesses as raw packed bytes in hex
        let witnesses: Vec<String> = self
            .witnesses
            .iter()
            .map(|w| format!("0x{}", hex::encode(w.pack().as_slice())))
            .collect();

        serde_json::json!({
            "version": format!("0x{:x}", self.version),
            "cell_deps": cell_deps,
            "header_deps": [],
            "inputs": inputs,
            "outputs": outputs,
            "outputs_data": outputs_data,
            "witnesses": witnesses,
        })
    }
}

// Impl pack for nested structures (Outpoint, CellInput, CellOuput, CellDep, WitnessArgs) and CKB script for CellOutput
impl OutPoint {
    pub fn pack(&self) -> MolOutpoint {
        MolOutpoint::new_builder()
            .tx_hash(Byte32::from_slice(&self.tx_hash).unwrap())
            .index(Uint32::from_slice(&self.index.to_le_bytes()).unwrap())
            .build()
    }
}

impl CellInput {
    pub fn pack(&self) -> MolCellInput {
        MolCellInput::new_builder()
            .previous_outpoint(self.previous_outpoint.pack())
            .since(Uint64::from_slice(&self.since.to_le_bytes()).unwrap())
            .build()
    }
}

impl CellOutput {
    pub fn pack(&self) -> MolCellOutput {
        let type_script = match self.type_script {
            Some(s) => ScriptOpt::new_builder().set(Some(s.pack())).build(),
            None => ScriptOpt::default(),
        };
        MolCellOutput::new_builder()
            .capacity(Uint64::from_slice(&self.capacity.to_le_bytes()).unwrap())
            .lock(self.lock_script.pack())
            .type_(type_script)
            .build()
    }
}

impl CellDep {
    pub fn pack(&self) -> MolCellDep {
        MolCellDep::new_builder()
            .out_point(self.outpoint.pack())
            .dep_type(Byte::new(self.dep_type))
            .build()
    }
}

impl WitnessArgs {
    /// Conversion of self to Molecule compatible data type
    pub fn pack(&self) -> MolWitnessArgs {
        // transform the original Vec<u8> to Bytes
        let lock = self.lock.as_ref().map(|l| Bytes::from(l.clone()));
        let input = self.input_type.as_ref().map(|i| Bytes::from(i.clone()));
        let output = self.output_type.as_ref().map(|o| Bytes::from(o.clone()));

        MolWitnessArgs::new_builder()
            .lock(BytesOpt::new_builder().set(lock).build())
            .input_type(BytesOpt::new_builder().set(input).build())
            .output_type(BytesOpt::new_builder().set(output).build())
            .build()
    }
}

#[cfg(test)]
mod e2e_tests {
    use super::*;
    use crate::data::{Account, CkbCell, CkbScript};
    use crate::network::consensus::MockLedger;
    use secp256k1::SecretKey;
    use std::collections::HashMap;

    // ---- shared fixtures ----

    /// Returns (Account, SecretKey, lock_script) all derived from the same secret bytes.
    fn make_actor(secret: [u8; 32]) -> (Account, SecretKey, CkbScript) {
        let account = Account::from_secret(secret);
        let sk = SecretKey::from_byte_array(secret).unwrap();
        let lock = CkbScript {
            code_hash: [0xABu8; 32],
            hash_type: 1,
            args: account.pubkey_hash,
        };
        (account, sk, lock)
    }

    fn make_ledger() -> MockLedger {
        MockLedger {
            live_cell: HashMap::new(),
        }
    }

    fn outpoint(seed: u8) -> OutPoint {
        OutPoint {
            tx_hash: [seed; 32],
            index: 0,
        }
    }

    fn cell_output(capacity: u64, lock: CkbScript) -> CellOutput {
        CellOutput {
            capacity,
            lock_script: lock,
            type_script: None,
        }
    }

    fn unsigned_transfer_tx(
        from_outpoint: OutPoint,
        to_lock: CkbScript,
        capacity: u64,
    ) -> CKBTransaction {
        CKBTransaction {
            version: 0,
            cell_deps: vec![],
            header_deps: [0u8; 32],
            inputs: vec![CellInput {
                previous_outpoint: from_outpoint,
                since: 0,
            }],
            witnesses: vec![WitnessArgs {
                lock: None,
                input_type: None,
                output_type: None,
            }],
            outputs: vec![cell_output(capacity, to_lock)],
            output_data: vec![],
        }
    }

    // ================================================================
    // Account / ownership
    // ================================================================

    #[test]
    fn account_unlocks_its_own_cell() {
        let (alice, _, alice_lock) = make_actor([0x01u8; 32]);
        let cell = CkbCell::new_for_test(100_0000_0000, alice_lock);
        assert!(cell.can_unlock_script(&alice));
    }

    #[test]
    fn account_cannot_unlock_foreign_cell() {
        let (_, _, alice_lock) = make_actor([0x01u8; 32]);
        let (bob, _, _) = make_actor([0x02u8; 32]);
        let cell = CkbCell::new_for_test(100_0000_0000, alice_lock);
        assert!(!cell.can_unlock_script(&bob));
    }

    #[test]
    fn invalid_hash_type_blocks_unlock() {
        let (alice, _, mut lock) = make_actor([0x01u8; 32]);
        lock.hash_type = 3; // 3 is not a valid hash type
        let cell = CkbCell::new_for_test(100_0000_0000, lock);
        assert!(!cell.can_unlock_script(&alice));
    }

    // ================================================================
    // validate_spend
    // ================================================================

    #[test]
    fn unsigned_tx_fails_validate_spend() {
        let (_, _, alice_lock) = make_actor([0x01u8; 32]);
        let (_, _, bob_lock) = make_actor([0x02u8; 32]);
        let input_cell = CkbCell::new_for_test(100_0000_0000, alice_lock);
        let tx = unsigned_transfer_tx(outpoint(0xAA), bob_lock, 99_0000_0000);
        assert!(tx.validate_spend(0, &[input_cell]).is_err());
    }

    #[test]
    fn signed_tx_passes_validate_spend() {
        let (alice, _, alice_lock) = make_actor([0x01u8; 32]);
        let (_, _, bob_lock) = make_actor([0x02u8; 32]);
        let cells = [CkbCell::new_for_test(100_0000_0000, alice_lock)];
        let mut tx = unsigned_transfer_tx(outpoint(0xAA), bob_lock, 99_0000_0000);
        tx.sign(&alice, &cells).unwrap();
        assert!(tx.validate_spend(0, &cells).is_ok());
    }

    #[test]
    fn since_lock_blocks_validate_spend() {
        let (alice, _, alice_lock) = make_actor([0x01u8; 32]);
        let (_, _, bob_lock) = make_actor([0x02u8; 32]);
        let cells = [CkbCell::new_for_test(100_0000_0000, alice_lock)];
        let mut tx = CKBTransaction {
            version: 0,
            cell_deps: vec![],
            header_deps: [0u8; 32],
            inputs: vec![CellInput {
                previous_outpoint: outpoint(0xAA),
                since: 100,
            }],
            witnesses: vec![WitnessArgs {
                lock: None,
                input_type: None,
                output_type: None,
            }],
            outputs: vec![cell_output(99_0000_0000, bob_lock)],
            output_data: vec![],
        };
        tx.sign(&alice, &cells).unwrap();
        assert!(tx.validate_spend(0, &cells).is_err());
    }

    #[test]
    fn missing_witness_slot_blocks_validate_spend() {
        let (_, _, alice_lock) = make_actor([0x01u8; 32]);
        let (_, _, bob_lock) = make_actor([0x02u8; 32]);
        let input_cell = CkbCell::new_for_test(100_0000_0000, alice_lock);
        let tx = CKBTransaction {
            version: 0,
            cell_deps: vec![],
            header_deps: [0u8; 32],
            inputs: vec![CellInput {
                previous_outpoint: outpoint(0xAA),
                since: 0,
            }],
            witnesses: vec![],
            outputs: vec![cell_output(99_0000_0000, bob_lock)],
            output_data: vec![],
        };
        assert!(tx.validate_spend(0, &[input_cell]).is_err());
    }

    #[test]
    fn second_input_without_witness_fails_validate_spend() {
        let (alice, _, alice_lock) = make_actor([0x01u8; 32]);
        let (_, _, bob_lock) = make_actor([0x02u8; 32]);
        let cells = [
            CkbCell::new_for_test(100_0000_0000, alice_lock),
            CkbCell::new_for_test(100_0000_0000, alice_lock),
        ];
        let mut tx = CKBTransaction {
            version: 0,
            cell_deps: vec![],
            header_deps: [0u8; 32],
            inputs: vec![
                CellInput {
                    previous_outpoint: outpoint(0xAA),
                    since: 0,
                },
                CellInput {
                    previous_outpoint: outpoint(0xBB),
                    since: 0,
                },
            ],
            witnesses: vec![
                WitnessArgs {
                    lock: None,
                    input_type: None,
                    output_type: None,
                },
                WitnessArgs {
                    lock: None,
                    input_type: None,
                    output_type: None,
                },
            ],
            outputs: vec![cell_output(99_0000_0000, bob_lock)],
            output_data: vec![],
        };
        tx.sign(&alice, &cells).unwrap();
        assert!(tx.validate_spend(0, &cells).is_ok());
        assert!(tx.validate_spend(1, &cells).is_err());
    }

    // ================================================================
    // Ledger state transitions
    // ================================================================

    #[test]
    fn birthed_cell_is_live() {
        let (_, _, lock) = make_actor([0x01u8; 32]);
        let mut ledger = make_ledger();
        let op = outpoint(0x01);
        ledger
            .birth_cell(&op, cell_output(100_0000_0000, lock))
            .unwrap();
        assert!(ledger.is_live(&op));
    }

    #[test]
    fn killed_cell_is_dead() {
        let (_, _, lock) = make_actor([0x01u8; 32]);
        let mut ledger = make_ledger();
        let op = outpoint(0x01);
        ledger
            .birth_cell(&op, cell_output(100_0000_0000, lock))
            .unwrap();
        ledger.kill_cell(&op).unwrap();
        assert!(!ledger.is_live(&op));
    }

    #[test]
    fn double_birth_same_outpoint_rejected() {
        let (_, _, lock) = make_actor([0x01u8; 32]);
        let mut ledger = make_ledger();
        let op = outpoint(0x01);
        assert!(
            ledger
                .birth_cell(&op, cell_output(100_0000_0000, lock))
                .is_ok()
        );
        assert!(
            ledger
                .birth_cell(&op, cell_output(200_0000_0000, lock))
                .is_err()
        );
        assert_eq!(ledger.live_cell[&op].capacity, 100_0000_0000);
    }

    #[test]
    fn double_spend_same_outpoint_rejected() {
        let (_, _, lock) = make_actor([0x01u8; 32]);
        let mut ledger = make_ledger();
        let op = outpoint(0x01);
        ledger
            .birth_cell(&op, cell_output(100_0000_0000, lock))
            .unwrap();
        assert!(ledger.kill_cell(&op).is_ok());
        assert!(ledger.kill_cell(&op).is_err());
    }

    #[test]
    fn kill_nonexistent_cell_returns_error() {
        let mut ledger = make_ledger();
        let op = outpoint(0x99);
        assert!(ledger.kill_cell(&op).is_err());
    }

    // ================================================================
    // Full e2e flows
    // ================================================================

    #[test]
    fn e2e_simple_transfer_alice_to_bob() {
        let (alice, _, alice_lock) = make_actor([0x01u8; 32]);
        let (bob, _, bob_lock) = make_actor([0x02u8; 32]);

        let mut ledger = make_ledger();
        let alice_op = outpoint(0xAA);
        ledger
            .birth_cell(&alice_op, cell_output(200_0000_0000, alice_lock))
            .unwrap();
        assert!(ledger.is_live(&alice_op));

        // ownership: Alice can unlock, Bob cannot
        assert!(CkbCell::new_for_test(200_0000_0000, alice_lock).can_unlock_script(&alice));
        assert!(!CkbCell::new_for_test(200_0000_0000, alice_lock).can_unlock_script(&bob));

        let cells = [CkbCell::new_for_test(200_0000_0000, alice_lock)];
        let mut tx = unsigned_transfer_tx(alice_op, bob_lock, 199_0000_0000);
        assert!(tx.validate_spend(0, &cells).is_err());

        tx.sign(&alice, &cells).unwrap();
        assert!(tx.validate_spend(0, &cells).is_ok());

        let bob_op = OutPoint {
            tx_hash: tx.hash(),
            index: 0,
        };
        ledger.kill_cell(&alice_op).unwrap();
        ledger
            .birth_cell(&bob_op, cell_output(199_0000_0000, bob_lock))
            .unwrap();

        assert!(!ledger.is_live(&alice_op));
        assert!(ledger.is_live(&bob_op));
        assert_eq!(ledger.live_cell.len(), 1);
    }

    #[test]
    fn e2e_signature_commits_to_recipient() {
        let (_, alice_sk, _) = make_actor([0x01u8; 32]);
        let (_, _, bob_lock) = make_actor([0x02u8; 32]);
        let (_, _, carol_lock) = make_actor([0x03u8; 32]);

        let tx_to_bob = unsigned_transfer_tx(outpoint(0xAA), bob_lock, 99_0000_0000);
        let tx_to_carol = unsigned_transfer_tx(outpoint(0xAA), carol_lock, 99_0000_0000);

        assert_ne!(tx_to_bob.hash(), tx_to_carol.hash());
        assert_ne!(tx_to_bob.create_sighash(), tx_to_carol.create_sighash());
        assert_ne!(
            tx_to_bob.create_signature(alice_sk),
            tx_to_carol.create_signature(alice_sk)
        );
    }

    #[test]
    fn e2e_validate_spend_rejects_wrong_signer() {
        let (_, _, alice_lock) = make_actor([0x01u8; 32]);
        let (_, bob_sk, _) = make_actor([0x02u8; 32]);
        let alice_cell = CkbCell::new_for_test(100_0000_0000, alice_lock);

        let mut tx = unsigned_transfer_tx(outpoint(0xAA), alice_lock, 99_0000_0000);
        let wrong_sig = tx.create_signature(bob_sk);
        tx.witnesses[0].lock = Some(wrong_sig.to_vec());

        let result = tx.validate_spend(0, &[alice_cell]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("wrong signer"));
    }

    /// sign() enforces ownership — it rejects if the account does not own the input cell.
    #[test]
    fn e2e_sign_rejects_wrong_account() {
        let (_, _, alice_lock) = make_actor([0x01u8; 32]);
        let (bob, _, _) = make_actor([0x02u8; 32]);

        let cells = [CkbCell::new_for_test(100_0000_0000, alice_lock)];
        let mut tx = unsigned_transfer_tx(outpoint(0xAA), alice_lock, 99_0000_0000);

        assert!(tx.sign(&bob, &cells).is_err());
        // witness slot remains empty — tx is still unsigned
        assert!(tx.validate_spend(0, &cells).is_err());
    }

    #[test]
    fn e2e_merge_two_cells() {
        let (alice, _, alice_lock) = make_actor([0x01u8; 32]);

        let mut ledger = make_ledger();
        let op_a = outpoint(0xA1);
        let op_b = outpoint(0xA2);
        ledger
            .birth_cell(&op_a, cell_output(100_0000_0000, alice_lock))
            .unwrap();
        ledger
            .birth_cell(&op_b, cell_output(100_0000_0000, alice_lock))
            .unwrap();

        let mut tx = CKBTransaction {
            version: 0,
            cell_deps: vec![],
            header_deps: [0u8; 32],
            inputs: vec![
                CellInput {
                    previous_outpoint: op_a,
                    since: 0,
                },
                CellInput {
                    previous_outpoint: op_b,
                    since: 0,
                },
            ],
            witnesses: vec![
                WitnessArgs {
                    lock: None,
                    input_type: None,
                    output_type: None,
                },
                WitnessArgs {
                    lock: None,
                    input_type: None,
                    output_type: None,
                },
            ],
            outputs: vec![cell_output(199_0000_0000, alice_lock)],
            output_data: vec![],
        };

        let cells = [
            CkbCell::new_for_test(100_0000_0000, alice_lock),
            CkbCell::new_for_test(100_0000_0000, alice_lock),
        ];
        tx.sign(&alice, &cells).unwrap();
        assert!(tx.validate_spend(0, &cells).is_ok());
        // validate_spend(1) fails — witnesses[1].lock is None (sign() only fills witnesses[0])
        // Copying witnesses[0].lock into witnesses[1] would also fail: sighash_all hashes
        // witnesses[1] as-is, so mutating it changes the sighash and invalidates the signature.
        assert!(tx.validate_spend(1, &cells).is_err());

        let merged_op = OutPoint {
            tx_hash: tx.hash(),
            index: 0,
        };
        ledger.kill_cell(&op_a).unwrap();
        ledger.kill_cell(&op_b).unwrap();
        ledger
            .birth_cell(&merged_op, cell_output(199_0000_0000, alice_lock))
            .unwrap();

        assert!(!ledger.is_live(&op_a));
        assert!(!ledger.is_live(&op_b));
        assert!(ledger.is_live(&merged_op));
        assert_eq!(ledger.live_cell.len(), 1);
    }

    #[test]
    fn e2e_double_spend_rejected_at_ledger() {
        let (alice, _, alice_lock) = make_actor([0x01u8; 32]);
        let (_, _, bob_lock) = make_actor([0x02u8; 32]);

        let mut ledger = make_ledger();
        let alice_op = outpoint(0xAA);
        ledger
            .birth_cell(&alice_op, cell_output(100_0000_0000, alice_lock))
            .unwrap();

        let mut tx1 = unsigned_transfer_tx(alice_op, bob_lock, 99_0000_0000);
        tx1.sign(&alice, &[CkbCell::new_for_test(100_0000_0000, alice_lock)])
            .unwrap();
        assert!(
            tx1.validate_spend(0, &[CkbCell::new_for_test(100_0000_0000, alice_lock)])
                .is_ok()
        );
        ledger.kill_cell(&alice_op).unwrap();

        let mut tx2 = unsigned_transfer_tx(alice_op, bob_lock, 99_0000_0000);
        tx2.sign(&alice, &[CkbCell::new_for_test(100_0000_0000, alice_lock)])
            .unwrap();
        assert!(
            tx2.validate_spend(0, &[CkbCell::new_for_test(100_0000_0000, alice_lock)])
                .is_ok()
        ); // tx itself looks valid...
        assert!(ledger.kill_cell(&alice_op).is_err()); // ...but ledger rejects it
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::CkbScript;
    use secp256k1::ecdsa::{RecoverableSignature, RecoveryId};
    use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};

    // ---- test fixtures ----

    fn lock_script() -> CkbScript {
        CkbScript {
            code_hash: [0xABu8; 32],
            hash_type: 1,
            args: [0xCDu8; 20],
        }
    }

    fn outpoint(seed: u8) -> OutPoint {
        OutPoint {
            tx_hash: [seed; 32],
            index: seed as u32,
        }
    }

    fn witness(lock: Option<Vec<u8>>) -> WitnessArgs {
        WitnessArgs {
            lock,
            input_type: None,
            output_type: None,
        }
    }

    fn base_tx() -> CKBTransaction {
        CKBTransaction {
            version: 0,
            cell_deps: vec![CellDep {
                outpoint: outpoint(1),
                dep_type: 0,
            }],
            header_deps: [0u8; 32],
            inputs: vec![CellInput {
                previous_outpoint: outpoint(2),
                since: 0,
            }],
            witnesses: vec![witness(None)],
            outputs: vec![CellOutput {
                capacity: 100_0000_0000,
                lock_script: lock_script(),
                type_script: None,
            }],
            output_data: vec![0xDE, 0xAD],
        }
    }

    fn sk() -> SecretKey {
        SecretKey::from_byte_array([0x01u8; 32]).unwrap()
    }

    // ---- OutPoint::pack ----

    #[test]
    fn outpoint_pack_roundtrip() {
        let mol = OutPoint {
            tx_hash: [0xABu8; 32],
            index: 7,
        }
        .pack();
        assert_eq!(mol.tx_hash().as_slice(), &[0xABu8; 32]);
        assert_eq!(mol.index().as_slice(), &7u32.to_le_bytes());
    }

    #[test]
    fn outpoint_pack_zero() {
        let mol = OutPoint {
            tx_hash: [0u8; 32],
            index: 0,
        }
        .pack();
        assert_eq!(mol.tx_hash().as_slice(), &[0u8; 32]);
        assert_eq!(mol.index().as_slice(), &0u32.to_le_bytes());
    }

    #[test]
    fn outpoint_pack_max_index() {
        let mol = OutPoint {
            tx_hash: [0u8; 32],
            index: u32::MAX,
        }
        .pack();
        assert_eq!(mol.index().as_slice(), &u32::MAX.to_le_bytes());
    }

    // ---- CellInput::pack ----

    #[test]
    fn cellinput_pack_roundtrip() {
        let mol = CellInput {
            previous_outpoint: OutPoint {
                tx_hash: [0x11u8; 32],
                index: 3,
            },
            since: 1000,
        }
        .pack();
        assert_eq!(mol.since().as_slice(), &1000u64.to_le_bytes());
        assert_eq!(mol.previous_outpoint().tx_hash().as_slice(), &[0x11u8; 32]);
        assert_eq!(
            mol.previous_outpoint().index().as_slice(),
            &3u32.to_le_bytes()
        );
    }

    #[test]
    fn cellinput_pack_zero_since() {
        let mol = CellInput {
            previous_outpoint: outpoint(0),
            since: 0,
        }
        .pack();
        assert_eq!(mol.since().as_slice(), &0u64.to_le_bytes());
    }

    #[test]
    fn cellinput_pack_max_since() {
        let mol = CellInput {
            previous_outpoint: outpoint(0),
            since: u64::MAX,
        }
        .pack();
        assert_eq!(mol.since().as_slice(), &u64::MAX.to_le_bytes());
    }

    // ---- CellOutput::pack ----

    #[test]
    fn celloutput_pack_no_type_script() {
        let mol = CellOutput {
            capacity: 500,
            lock_script: lock_script(),
            type_script: None,
        }
        .pack();
        assert_eq!(mol.capacity().as_slice(), &500u64.to_le_bytes());
        assert!(mol.type_().to_opt().is_none());
    }

    #[test]
    fn celloutput_pack_with_type_script() {
        let ts = CkbScript {
            code_hash: [0x99u8; 32],
            hash_type: 0,
            args: [0x01u8; 20],
        };
        let mol = CellOutput {
            capacity: 200,
            lock_script: lock_script(),
            type_script: Some(ts),
        }
        .pack();
        assert!(mol.type_().to_opt().is_some());
    }

    // ---- CellDep::pack ----

    #[test]
    fn celldep_pack_code_type() {
        let mol = CellDep {
            outpoint: outpoint(1),
            dep_type: 0,
        }
        .pack();
        assert_eq!(mol.dep_type().as_slice(), &[0u8]);
        assert_eq!(mol.out_point().tx_hash().as_slice(), &[1u8; 32]);
    }

    #[test]
    fn celldep_pack_dep_group_type() {
        let mol = CellDep {
            outpoint: outpoint(1),
            dep_type: 1,
        }
        .pack();
        assert_eq!(mol.dep_type().as_slice(), &[1u8]);
    }

    // ---- WitnessArgs::pack ----

    #[test]
    fn witnessargs_pack_all_none() {
        let mol = WitnessArgs {
            lock: None,
            input_type: None,
            output_type: None,
        }
        .pack();
        assert!(mol.lock().to_opt().is_none());
        assert!(mol.input_type().to_opt().is_none());
        assert!(mol.output_type().to_opt().is_none());
    }

    #[test]
    fn witnessargs_pack_all_some() {
        let mol = WitnessArgs {
            lock: Some(vec![0x11u8; 10]),
            input_type: Some(vec![0x22u8; 5]),
            output_type: Some(vec![0x33u8; 3]),
        }
        .pack();
        assert!(mol.lock().to_opt().is_some());
        assert!(mol.input_type().to_opt().is_some());
        assert!(mol.output_type().to_opt().is_some());
        assert_eq!(
            mol.lock().to_opt().unwrap().raw_data().as_ref(),
            [0x11u8; 10]
        );
    }

    #[test]
    fn witnessargs_pack_mixed() {
        let mol = WitnessArgs {
            lock: Some(vec![0xAAu8; 65]),
            input_type: None,
            output_type: Some(vec![0xBBu8; 4]),
        }
        .pack();
        assert!(mol.lock().to_opt().is_some());
        assert!(mol.input_type().to_opt().is_none());
        assert!(mol.output_type().to_opt().is_some());
    }

    // ---- CKBTransaction::hash ----

    #[test]
    fn hash_is_deterministic() {
        let tx = base_tx();
        assert_eq!(tx.hash(), tx.hash());
    }

    #[test]
    fn hash_is_nonzero() {
        assert_ne!(base_tx().hash(), [0u8; 32]);
    }

    #[test]
    fn hash_changes_with_version() {
        let tx_a = CKBTransaction {
            version: 0,
            ..base_tx()
        };
        let tx_b = CKBTransaction {
            version: 1,
            ..base_tx()
        };
        assert_ne!(tx_a.hash(), tx_b.hash());
    }

    #[test]
    fn hash_changes_with_output_capacity() {
        let tx_a = base_tx();
        let tx_b = CKBTransaction {
            outputs: vec![CellOutput {
                capacity: 999,
                lock_script: lock_script(),
                type_script: None,
            }],
            ..base_tx()
        };
        assert_ne!(tx_a.hash(), tx_b.hash());
    }

    #[test]
    fn hash_excludes_witnesses() {
        // hash() must not incorporate witnesses — identical tx body, different witnesses → same hash
        let tx_a = CKBTransaction {
            witnesses: vec![],
            ..base_tx()
        };
        let tx_b = CKBTransaction {
            witnesses: vec![witness(Some(vec![0xFF; 32]))],
            ..base_tx()
        };
        assert_eq!(tx_a.hash(), tx_b.hash());
    }

    // ---- CKBTransaction::create_sighash ----

    #[test]
    fn sighash_is_deterministic() {
        let tx = base_tx();
        assert_eq!(tx.create_sighash(), tx.create_sighash());
    }

    #[test]
    fn sighash_differs_from_tx_hash() {
        // sighash folds in witnesses, so it must differ from the raw tx hash
        let tx = base_tx();
        assert_ne!(tx.hash(), tx.create_sighash());
    }

    #[test]
    fn sighash_first_witness_lock_is_zeroed() {
        // witness[0].lock is replaced with 65 zeros during hashing, so its actual
        // content must not affect the sighash — that is the placeholder invariant
        let tx_a = CKBTransaction {
            witnesses: vec![witness(Some(vec![0x01u8; 65]))],
            ..base_tx()
        };
        let tx_b = CKBTransaction {
            witnesses: vec![witness(Some(vec![0xFFu8; 65]))],
            ..base_tx()
        };
        assert_eq!(tx_a.create_sighash(), tx_b.create_sighash());
    }

    #[test]
    fn sighash_no_witnesses_equals_hash_of_raw_tx_hash() {
        // With zero witnesses the hasher only processes the raw_tx_hash, so we can
        // reproduce the expected value independently
        let tx = CKBTransaction {
            witnesses: vec![],
            ..base_tx()
        };
        let raw = tx.hash();
        let mut h = blake2b_rs::Blake2bBuilder::new(32)
            .personal(b"ckb-default-hash")
            .build();
        h.update(&raw);
        let mut expected = [0u8; 32];
        h.finalize(&mut expected);
        assert_eq!(tx.create_sighash(), expected);
    }

    #[test]
    fn sighash_second_witness_is_not_zeroed() {
        // Unlike witness[0], subsequent witnesses are hashed as-is
        let tx_a = CKBTransaction {
            witnesses: vec![witness(None), witness(None)],
            ..base_tx()
        };
        let tx_b = CKBTransaction {
            witnesses: vec![witness(None), witness(Some(vec![0xFFu8; 10]))],
            ..base_tx()
        };
        assert_ne!(tx_a.create_sighash(), tx_b.create_sighash());
    }

    #[test]
    fn sighash_changes_with_tx_body() {
        let tx_a = base_tx();
        let tx_b = CKBTransaction {
            outputs: vec![CellOutput {
                capacity: 999,
                lock_script: lock_script(),
                type_script: None,
            }],
            ..base_tx()
        };
        assert_ne!(tx_a.create_sighash(), tx_b.create_sighash());
    }

    // ---- CKBTransaction::create_signature ----

    #[test]
    fn signature_is_65_bytes() {
        assert_eq!(base_tx().create_signature(sk()).len(), 65);
    }

    #[test]
    fn signature_recovery_id_is_valid() {
        let sig = base_tx().create_signature(sk());
        assert!(
            sig[64] == 0 || sig[64] == 1,
            "recovery id must be 0 or 1, got {}",
            sig[64]
        );
    }

    #[test]
    fn signature_is_deterministic() {
        // secp256k1 uses RFC 6979 deterministic nonces
        let tx = base_tx();
        assert_eq!(tx.create_signature(sk()), tx.create_signature(sk()));
    }

    #[test]
    fn signature_differs_for_different_keys() {
        let tx = base_tx();
        let sk_a = SecretKey::from_byte_array([0x01u8; 32]).unwrap();
        let sk_b = SecretKey::from_byte_array([0x02u8; 32]).unwrap();
        assert_ne!(tx.create_signature(sk_a), tx.create_signature(sk_b));
    }

    #[test]
    fn signature_differs_for_different_transactions() {
        let tx_a = base_tx();
        let tx_b = CKBTransaction {
            outputs: vec![CellOutput {
                capacity: 999,
                lock_script: lock_script(),
                type_script: None,
            }],
            ..base_tx()
        };
        assert_ne!(tx_a.create_signature(sk()), tx_b.create_signature(sk()));
    }

    #[test]
    fn signature_recovers_to_correct_pubkey() {
        // End-to-end check: sign → recover → verify the recovered key matches the signer
        let secret = sk();
        let secp = Secp256k1::new();
        let expected_pubkey = PublicKey::from_secret_key(&secp, &secret);

        let tx = base_tx();
        let sig_bytes = tx.create_signature(secret);

        let recovery_id = RecoveryId::from_u8_masked(sig_bytes[64]);
        let rec_sig = RecoverableSignature::from_compact(&sig_bytes[..64], recovery_id).unwrap();
        let msg = Message::from_digest(tx.create_sighash());

        let recovered = secp.recover_ecdsa(msg, &rec_sig).unwrap();
        assert_eq!(recovered, expected_pubkey);
    }
}
