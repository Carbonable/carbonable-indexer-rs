alias c := clippy
default_env := "testnet"
default_starting_block := "0"
default_force := ""

apibara_default_token := "change_me"

testnet_config := "APIBARA_URI=https://goerli.starknet.a5a.ch NETWORK=goerli"
mainnet_config := "APIBARA_URI=https://mainnet.starknet.a5a.ch NETWORK=mainnet"

default:
    just --list

# run indexer against against blockchain as data source
run_indexer env=default_env starting_block=default_starting_block force=default_force:
	{{ if env == "mainnet" { mainnet_config } else { testnet_config } }} DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/  SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 APIBARA_TOKEN={{apibara_default_token}} RUST_LOG=debug RUST_BACKTRACE=1 cargo run -p carbonable-indexer -- --starting-block {{starting_block}} --batch-size 10 --only-index {{force}}

# seed base data from data directory
run_seeding env=default_env:
	{{ if env == "mainnet" { mainnet_config } else { testnet_config } }} DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/ SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 APIBARA_TOKEN={{apibara_default_token}} RUST_LOG=info RUST_BACKTRACE=1 cargo run -p carbonable-indexer -- --only-seed

# run api package to expose carbonable indexer at `http://localhost:8000`
run_api env=default_env:
	{{ if env == "mainnet" { mainnet_config } else { testnet_config } }} RUST_LOG=debug DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/ SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 APIBARA_TOKEN={{apibara_default_token}} cargo run -p carbonable-api

# migrate database to newest schema version
migrate:
	DATABASE_URL=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer cargo run -p carbonable-migration 

# migrate database down to version 0
migrate_down:
	DATABASE_URL=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer cargo run -p carbonable-migration down

# refresh database
reset:
	DATABASE_URL=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer cargo run -p carbonable-migration refresh


# runs {{target}} crate's tests
test target:
    cargo test -p {{target}} -- --nocapture

# runs cargo clippy project wide
clippy:
    cargo clippy