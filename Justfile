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
	{{ if env == "mainnet" { mainnet_config } else { testnet_config } }} RUST_LOG=debug RUST_BACKTRACE=1 DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/ SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 APIBARA_TOKEN={{apibara_default_token}} cargo run -p carbonable-api

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

# start application database
start_db: 
    docker compose -p carbonable-indexer up -d

# installs project stack
install: start_db && reset run_seeding
    cargo build
    
# deploy application to desired environment
deploy env=default_env:
    fly deploy -c fly.{{env}}.toml

# connect to psql locally
db_connect:
    docker exec -ti carbonable-indexer-db-1 psql -W carbonable_indexer carbonable

# proxy database to port to localhost
proxy env=default_env:
    fly proxy 5432 -a carbonable-{{env}}-indexer-db

# ssh to app
ssh env=default_env:
    fly ssh console -c fly.{{env}}.toml

# restart indexer-service
restart_indexer env=default_env:
    fly m ls -c fly.{{env}}.toml -j | jq '.[] | select(.state == "started" and .config.metadata.fly_process_group == "indexer") | .id' | xargs -n 1 fly m restart -c fly.{{env}}.toml
