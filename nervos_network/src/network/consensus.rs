// Educational mock only — not used in production.
// In production, cell liveness is determined by the CKB indexer RPC (get_cells),
// and cells are consumed/created by broadcasting transactions via send_transaction.
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use serde::{Serialize, Deserialize};
#[cfg(test)]
use crate::network::transaction::{OutPoint, CellOutput};
#[cfg(test)]
use anyhow::{Result, anyhow};

#[cfg(test)]
pub struct MockLedger{
    pub live_cell: HashMap<OutPoint, CellOutput>
}

#[cfg(test)]
impl MockLedger{
    pub fn is_live(&self, outpoint: &OutPoint) -> bool {
        // self.live_cell.contains_key(outpoint)
        self.live_cell.keys().any(|x| x == outpoint)
    }

    pub fn kill_cell(&mut self, outpoint: &OutPoint) -> Result<()> {
        if self.live_cell.remove(outpoint).is_some(){
            return Ok(())
        }
        return Err(anyhow!("Cannot kill non-existing cell"));
    }
    
    pub fn birth_cell(&mut self, outpoint: &OutPoint, cell: CellOutput) -> Result<()> {
        if self.is_live(outpoint) {
            return Err(anyhow!("Cannot rebirth cell for this OutPoint"));
        }
        self.live_cell.insert(outpoint.clone(), cell);
        Ok(())
    }

        pub fn new() -> Self {
        MockLedger { live_cell: std::collections::HashMap::new() }
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = std::fs::read_to_string(path)?;
        let ld: LedgerData = serde_json::from_str(&data)?;
        let live_cell = ld.cells.into_iter().map(|e| (e.outpoint, e.output)).collect();
        Ok(MockLedger { live_cell })
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let cells = self.live_cell.iter()
            .map(|(k, v)| CellEntry { outpoint: *k, output: *v })
            .collect();
        let json = serde_json::to_string_pretty(&LedgerData { cells })?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
#[derive(Serialize, Deserialize)]
struct LedgerData {
    cells: Vec<CellEntry>,
}

#[cfg(test)]
#[derive(Serialize, Deserialize)]
struct CellEntry {
    outpoint: OutPoint,
    output: CellOutput,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::CkbScript;
    use std::collections::HashMap;

    fn make_dummy_script() -> CkbScript {
        CkbScript {
            code_hash: [0u8; 32],
            hash_type: 0,
            args: [0u8; 20],
        }
    }

    fn make_cell(capacity: u64) -> CellOutput {
        CellOutput { capacity, lock_script: make_dummy_script(), type_script: None }
    }

    fn make_outpoint(seed: u8, index: u32) -> OutPoint {
        OutPoint { tx_hash: [seed; 32], index }
    }

    #[test]
    fn test_is_live_initially_false() {
        let ledger = MockLedger { live_cell: HashMap::new() };
        let outpoint = make_outpoint(1, 0);
        assert!(!ledger.is_live(&outpoint));
    }

    #[test]
    fn test_birth_and_is_live() {
        let mut ledger = MockLedger { live_cell: HashMap::new() };
        let outpoint = make_outpoint(2, 0);
        let cell = make_cell(1000);

        // birth the cell
        assert!(ledger.birth_cell(&outpoint, cell).is_ok());
        assert!(ledger.is_live(&outpoint));
        assert_eq!(ledger.live_cell.len(), 1);
    }

    #[test]
    fn test_kill_cell() {
        let mut ledger = MockLedger { live_cell: HashMap::new() };
        let outpoint = make_outpoint(3, 0);
        let cell = make_cell(2000);

        assert!(ledger.birth_cell(&outpoint, cell).is_ok());
        assert!(ledger.is_live(&outpoint));

        // kill it
        ledger.kill_cell(&outpoint);
        assert!(!ledger.is_live(&outpoint));
        assert_eq!(ledger.live_cell.len(), 0);
    }

    #[test]
    fn test_birth_rejects_rebirth() {
        let mut ledger = MockLedger { live_cell: HashMap::new() };
        let outpoint = make_outpoint(4, 0);
        let cell1 = make_cell(3000);
        let cell2 = make_cell(4000);

        assert!(ledger.birth_cell(&outpoint, cell1).is_ok());
        assert!(ledger.is_live(&outpoint));
        assert_eq!(ledger.live_cell.get(&outpoint).unwrap().capacity, 3000);

        // birth again should be rejected
        assert!(ledger.birth_cell(&outpoint, cell2).is_err());
        // original cell remains
        assert_eq!(ledger.live_cell.get(&outpoint).unwrap().capacity, 3000);
        assert_eq!(ledger.live_cell.len(), 1);
    }
}
