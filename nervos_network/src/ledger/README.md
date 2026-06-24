# ledger

Persistent storage for the **mock** CKB ledger state used in unit and integration tests.

> **Note:** This folder is not used in production. In production, cell liveness is managed by a real CKB node and queried via the `get_cells` indexer RPC in `src/network/rpc.rs`. The mock ledger exists only to support offline testing without a running node.

---

## What is the ledger?

In CKB, the global state is a set of **live cells** — cells that have been created by a transaction and not yet consumed. When a transaction is broadcast, it:

1. Kills its input cells (removes them from the live set)
2. Births its output cells (adds them to the live set)

The mock ledger (`MockLedger` in `src/network/consensus.rs`) replicates this logic in memory using a `HashMap<OutPoint, CellOutput>`.

---

## Storage format

The mock ledger persists to a JSON file in this directory. Default path: `src/ledger/ledger.json`.

```json
{
  "cells": [
    {
      "outpoint": {
        "tx_hash": "aaaaaa...aa",
        "index": 0
      },
      "output": {
        "capacity": 10000000000,
        "lock_script": {
          "code_hash": "9bd7e0...e8",
          "hash_type": 1,
          "args": "b39bbc...64"
        },
        "type_script": null
      }
    }
  ]
}
```

Each entry maps an `OutPoint` (tx_hash + index) to a `CellOutput` (capacity + lock + type). An outpoint present in this file is "live" — one that has been removed is "dead" (spent).

---

## Usage in tests

The `MockLedger` is used in `consensus.rs` and the e2e test suite in `transaction.rs` to verify that:

- Cells cannot be double-spent
- Cells cannot be born twice at the same outpoint
- Spending a dead cell is rejected
- The full sign → validate → kill → birth cycle works correctly offline
