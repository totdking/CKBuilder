1. Balance should accept address maybe pubkey hash and rpc destination and should be able to queary any account available on the rpc, and there should be an option to include displaying the utxo for that account.
2. tx send shoudl accept either a ck address or pubkey hash.
3. send --amount math should be cleanly handled
4. for tx build, reduce the complexities and add the basic things needed to build a tx of any set
5. address can be used to viw any account on the ckb network (if that's possible). it can be used as advanced scripting, not basic top-level concern
6. include correct flag to indicate what it actually does, there shougl be a tx (sign i think to be able to sign offchain genearated tx )
7. Sign should accept keypair path or secret key
8. Put a flag to help in the devnet/offckb shorthand to help
9. I want a layer (i think this is already implemented) that interacts with offckb. node running like a scaffold that can allow the dev experiment on offckb to his/her heart's content
10. Account new shoudl by default store the acc to the default folder in the config/ckb location, but can be changed with configurable destination.
11. Accoht show shoould be able to shwo the defautl account there but as well should be able to show account details of any ckb account somehting similar to `solana account info <publickey>`
