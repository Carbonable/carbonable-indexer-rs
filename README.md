# carbonable-indexer-rs
Indexer Carbonable in Rust

Pre-requisite
---

Install [just](https://github.com/casey/just#installation). This is a more convenient task runner.
Install [docker](https://docs.docker.com/engine/install)
Install [rust](https://www.rust-lang.org/tools/install)

To install project and its dependencies.
```shell
$ just install
```

**Everytime `${env}` is asked, default is testnet so you wille have to tell env only for mainnet**


### Deployment

To testnet
```shell
$ just deploy 
```

Or mainnet
```shell
$ just deploy mainnet
```

### Description

Indexer has 3 main commands :
- migration (just migrate // cd /srv/www && ./carbonable-migration)
- seeding (just run_seeding // cd /srv/www && ./carbonable-indexer --only-seed)
- indexing (just run_indexer // cd /srv/www && ./carbonable-indexer --only-index)


Base onchain contract address can be found under `data/{env}.data.json`


---

If at some time you need to **reindex data** or **re-seed** application (e.g contract address have changed)
```shell
$ just ssh ${env}
$ cd srv/www
$ ./carbonable-migration refresh
$ ./carbonable-indexer --only-seed
$ exit
$ just restart_indexer ${env}
```
 

### Utils

List all available commands with documentation
```shell
$ just --list
```


Write migration:
```shell
$ cargo install sea-orm-cli
$ cd packages
$ sea-orm-cli migrate generate ${name_of_the_migration}
```


Full reset database 
```shell
$ just reset && just run_seeding && just run_indexer
```
