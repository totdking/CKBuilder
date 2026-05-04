use anyhow::{Ok, Result, anyhow};
use bech32::{self, Bech32m, Hrp};
use molecule::prelude::{Builder, Entity};
use serde::{Serialize, Deserialize};
use crate::network::consensus::MockLedger;
use crate::network::transaction::{OutPoint, CellOutput};
use crate::data::account::Account;
// for transaction of generaton of ckb schema serialized tx using moleculec
use crate::schemas::{Byte32, Bytes, Script};


/// Ckb Cell is consumed and the data in it is replaced by another cell
#[derive(Debug)]
pub struct CkbCell{
    /// Size of the cell in shannons
    pub capacity: u64,
    /// Data stored in the Cell/ used for storing states
    pub data: Vec<u8>,
    /// CkbScript that defines the ownership of the Cell
    pub lock_script: CkbScript,
    /// Enforces the rules that must be followed in a tx for a cell to be consumed as an input
    /// Or for a cell to be created as an output
    pub type_script: Option<CkbScript>
}

/// CkbScript that defines the ownership of the Cell
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CkbScript {
    /// Hash of the ELF formatted RISC-V binary that contains a CKB CkbScript.
    /// logic of the address: Multi-sig, Anyway-Can-Pay, Passkey (R1) lock
    #[serde(with = "crate::cli::hex_serde::array32")]
    pub code_hash: [u8;32],
    /// - 0	"Data"	code_hash is the Blake2b hash of the binary. Uses VM v0.
    /// - 1	"Type"	code_hash is the Type CkbScript hash of a cell. Uses latest VM / v2.
    /// - 2	"Data1"	code_hash is the Blake2b hash of the binary. Uses VM v1.
    /// - 4	"Data2"	code_hash is the Blake2b hash of the binary. Uses VM v2.
    pub hash_type: u8,
    /// Arguments as the CkbScript input: **pubkey_hash of Account**
    #[serde(with = "crate::cli::hex_serde::array20")]
    pub args: [u8;20],
}

#[derive(Debug)]
pub enum HashType{
    Data,
    Type,
    Data1,
    Data2
}

impl CkbScript{
    #[cfg(test)]
    pub fn create_rand_lock_script() -> Result<Self> {
        use rand::{ Rng, rng};

        let mut rng = rng();
        let mut code_hash = [0u8; 32];
        let mut args = [0u8; 20];
        rng.fill_bytes(&mut code_hash);
        rng.fill_bytes(&mut args);
        // Way to generate random numbers 
        let hash_type = (rng.next_u32() % 5) as u8;
        let lockscript = CkbScript { code_hash, hash_type, args };
        if lockscript.is_valid_hash_type() {
            return Ok(lockscript) ;
        } else {
            Err(anyhow!("Not valid hashtype"))
        }
    }
    
    /// if the hashtype is a valid one 
    pub fn is_valid_hash_type(&self) -> bool {
        match self.hash_type {
            0 => true,
            1 => true,
            2 => true,
            4 => true,
            _ => false
        }
    }

    /// returns hashtype of lockscript
    pub fn hash_type(&self) -> HashType{
        match self.hash_type{
            0 => HashType::Data,
            1 => HashType::Type,
            2 => HashType::Data1,
            4 => HashType::Data2,
            _ => panic!()
        }
    }

    /// CkB helper function in cell.rs to use in molecule tx serialization
    pub fn pack (&self) -> Script {
        Script::new_builder()
        .code_hash(Byte32::from_slice(&self.code_hash).unwrap())
        .hash_type::<u8>(self.hash_type.into())
        .args(Bytes::from(self.args.to_vec()))
        .build()
    }
}

impl CkbCell{
    pub fn load_cell(&self){}
   
    pub fn consume_cell(mut mockledger: MockLedger, outpoint: &OutPoint) -> Result<()> {
        mockledger.kill_cell(outpoint)
    }
    pub fn create_cell(mut mockledger: MockLedger, outpoint: &OutPoint, cell: CellOutput) -> Result<()> {
        mockledger.birth_cell(outpoint, cell)
    }

    /// Before any tx on any cell can occur, it has to first unlock the lockscript
    /// 
    /// account to unlock a lock script 
    /// Lock Script args(pubkey hash) and if it is a valid hash-type
    pub fn can_unlock_script(&self, account: &Account) -> bool {
        // lockscript args
        if account.pubkey_hash != self.lock_script.args{
            return false
        }
        // Transaction witness
        if !self.lock_script.is_valid_hash_type(){
            return false
        }
        true
    }

    /// true is live, false is dead cell
    pub fn is_live(mockledger: &MockLedger, outpoint: &OutPoint) -> bool {
        MockLedger::is_live(&mockledger, outpoint)
    }

    /// One account can have several addresses due to different lockscripts
    pub fn create_address(lock_script: CkbScript) -> Result<String> {
        let CkbScript { code_hash,  hash_type, args } = lock_script;

        // Check to see if it is a valid hashtype used for lockscript
        if !lock_script.is_valid_hash_type(){
            return Err(anyhow!("Not valid hash type"));
        }

        // create payload
        let mut payload = Vec::with_capacity(32+1+20);
        payload.push(0x00);
        payload.extend_from_slice(&code_hash);
        payload.push(hash_type);
        payload.extend_from_slice(&args);

        // ckt for testnet and ckb for mainnet
        let hrp = Hrp::parse("ckb")?;
        // pass in address parameters
        let address = bech32::encode::<Bech32m>(hrp, &payload)?;
        Ok(address)
    }
}

impl CkbCell {
    pub fn new(capacity: u64, lock_script: CkbScript) -> Self {
        CkbCell { capacity, data: vec![], lock_script, type_script: None }
    }

    pub fn lock_args(&self) -> [u8; 20] {
        self.lock_script.args
    }
}

#[cfg(test)]
impl CkbCell {
    pub fn new_for_test(capacity: u64, lock_script: CkbScript) -> Self {
        Self::new(capacity, lock_script)
    }
}

#[cfg(test)]
mod tests{
    /// Courtesy of this test example : [link](https://docs.nervos.org/docs/ckb-fundamentals/ckb-address#example-generating-a-full-address)
    #[test]
    fn test_create_address(){
        use super::*;
        // let lock_script = CkbScript::create_rand_lock_script().unwrap();
        let lock_script2 = CkbScript{
            code_hash: hex::decode("9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8").unwrap().as_array().unwrap().to_owned(),
            hash_type: 01 ,
            args: hex::decode("b39bbc0b3673c7d36450bc14cfcdad2d559c6c64").unwrap().as_array().unwrap().to_owned()
        };
        let address = CkbCell::create_address(lock_script2).unwrap();
        println!("{:?}", address)
    }
}