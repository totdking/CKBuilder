use secp256k1::{SecretKey, Secp256k1, PublicKey};
use blake2b_rs::Blake2bBuilder;
use crate::data::{CkbScript, cell};

// Secret : raw entropy/ random 32 bytes pulled from mnemonic phrase
// private key [u8;32] 32 bytes 
// public key [u8;33] 33 bytes
// pubkey hash [u8;20] 160 bits / 20 bytes / H160
// Address 95 chars

pub struct Account{
    // for signing (key)
    pub private_key : [u8;32],
    // for identification (lock)
    pub pubkey_hash: [u8;20]
}

impl Account {
    /// To generate an Account keypair from 32 byte entropy secret key
    /// 
    /// secret -> Public Key -> Hash -> Truncate
    pub fn from_secret(secret: [u8;32]) -> Self {
        // Convert raw bytes to a Secp Secretkey Object
        let sk = SecretKey::from_byte_array(secret).expect("Invalid secret key length or range");

        // Derive the 33 byte compressed pubkey
        let secp = Secp256k1::new();
        let pubkey = PublicKey::from_secret_key(&secp, &sk);
        let ser_pubkey = pubkey.serialize();

        // Hash the Public Key using personalized Blake2b Using ckb-hash
        let mut hasher = Blake2bBuilder::new(32).personal(b"ckb-default-hash").build();
        hasher.update(&ser_pubkey);
        let mut full_hash = [0u8;32];
        hasher.finalize(&mut full_hash);

        // Truncate to 20 bytes (The "Blake160" / pubkey_hash)
        let mut pubkey_hash = [0u8;20];
        pubkey_hash.copy_from_slice(&full_hash[0..20]);

        // return the finalized account gotten
        Self{
            private_key: secret,
            pubkey_hash
        }
    }
}

#[derive(Debug)]
pub struct Address(String);

impl Address {
    /// Generates a Bech32m CKB address from a lock script
    pub fn from_script(lock_script: CkbScript) -> Address{
        let add_str = cell::CkbCell::create_address(lock_script);
        Address(add_str.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::{SecretKey, Secp256k1, PublicKey};
    use blake2b_rs::Blake2bBuilder;

    #[test]
    fn test_from_secret_derives_pubkey_hash() {
        // choose a deterministic secret
        let secret: [u8;32] = [0x11u8; 32];

        // run the constructor
        let acct = Account::from_secret(secret);

        // independently derive expected pubkey_hash
        let sk = SecretKey::from_byte_array(secret).expect("secret -> SecretKey");
        let secp = Secp256k1::new();
        let pubkey = PublicKey::from_secret_key(&secp, &sk);
        let ser_pubkey = pubkey.serialize();

        let mut hasher = Blake2bBuilder::new(32).personal(b"ckb-default-hash").build();
        hasher.update(&ser_pubkey);
        let mut full_hash = [0u8;32];
        hasher.finalize(&mut full_hash);

        let mut expected = [0u8;20];
        expected.copy_from_slice(&full_hash[0..20]);

        assert_eq!(acct.pubkey_hash, expected);
        assert_eq!(acct.private_key, secret);
    }
}