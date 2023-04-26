alias c := clippy

default:
    just --list

# run indexer against against blockchain as data source
run_indexer:
	DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/ NETWORK=goerli SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 RUST_LOG=info RUST_BACKTRACE=1 cargo run -p carbonable-indexer -- --starting-block 0 --batch-size 10 --only-index

# seed base data from data directory
run_seeding:
	DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/ NETWORK=goerli SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 RUST_LOG=info RUST_BACKTRACE=1 cargo run -p carbonable-indexer -- --only-seed

# run api package to expose carbonable indexer at `http://localhost:8000`
run_api:
	RUST_LOG=debug DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/ SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 NETWORK=goerli cargo run -p carbonable-api

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
