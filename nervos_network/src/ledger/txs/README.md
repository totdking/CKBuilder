# ledger/txs

Temporary transaction files produced by `tx build` and `tx sign`.

Each file is the JSON representation of a single transaction at a point in time. Once the transaction has been applied to the ledger (`ledger kill` + `ledger birth`), the file is stale and can be deleted.

These files are excluded from git.
