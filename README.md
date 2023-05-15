# carbonable-indexer-rs
Indexer Carbonable in Rust

Pre-requisite
---

Install [just](https://github.com/casey/just#installation). This is a more convenient task runner.
Install [docker](https://docs.docker.com/engine/install)

List all available commands with documentation
```shell
$ just --list
```


To install project and its dependencies.
```shell
$ just install
```

Full reset database 
```shell
$ just reset && just run_seeding && just run_indexer
```

Deploy application
---
Deploy to testnet
```shell
$ just deploy testnet
```

Or mainnet
```shell
$ just deploy mainnet
```
