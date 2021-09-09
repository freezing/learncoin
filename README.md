# coolcoin

## Generate graphviz dot file

```
cargo run -- client --server "127.0.0.1:8334" getfullblockchain
cat blockchain.dot | dot -Tsvg > blockchain.svg
```

## Start a fullnode

```
cargo run -- daemon --enable_logging --coinbase_address "nikola's pocket" --server 127.0.0.1:8334
cargo run -- daemon --enable_logging --coinbase_address "coffee shop" --server 127.0.0.1:8333 --peers "127.0.0.1:8334"
```

## Send raw transaction

```
cargo run -- client --server "127.0.0.1:8334" \
    sendrawtransaction --inputs 9961b01dcd9b5263716e858b7a059570037642c469a5e097fe11c0a2763805e2:0 \
    --outputs mxh3H416KCRoBDiweSESew5YJyAk1nxLrN:35,mkrzDhhZtzQm8zgckSs4fMNrvtNJ66zaFe:15
```