[![Build Status](https://travis-ci.com/gnosis/subgraph-rs.svg?branch=main)](https://travis-ci.com/github/gnosis/subgraph-rs)

# `subgraph-rs`

Write subgraph mappings in Rust 🦀

## TODO

- [ ] Add better logging and `-v|--verbose` flag
- [ ] Improve diplaying errors to users
- [ ] Wasm module post-processing:
  - `start` function export
  - `wasm-opt`
- [ ] Finish porting all APIs to `subgraph` crate
  - [ ] bigDecimal
  - [-] bigInt
  - [ ] crypto
  - [ ] dataSource
  - [ ] ens
  - [ ] ethereum
  - [ ] ipfs
  - [ ] json
  - [x] log
  - [ ] store
  - [-] typeConversion
- [ ] ABI Code generation
- [ ] `subgraph` insert custom section to ammend manifest
- [ ] Upgrade to `apiVersion: 0.0.5` (requires changes to AssemblyScript types)
