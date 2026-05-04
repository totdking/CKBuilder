# ledger

This folder is the persistent storage location for the mock CKB ledger state.

When the CLI writes or reads ledger state, it targets a JSON file in this directory. The default file is `src/ledger/ledger.json`.

Each JSON file represents a snapshot of the live cell set at a point in time — a flat list of outpoint/cell pairs:

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

To point the CLI at this directory, pass `--ledger src/ledger/ledger.json` on any command, or run all commands from the project root with the default overridden.
