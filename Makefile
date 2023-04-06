run_indexer:
	DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/ NETWORK=goerli SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 RUST_LOG=info RUST_BACKTRACE=1 cargo run -p carbonable-indexer

run_api:
	RUST_LOG=debug DATABASE_URI=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer GATEWAY=https://carbonable.infura-ipfs.io/ipfs/ SEQUENCER_DOMAIN=https://DOMAIN.infura.io/v3/f46a67c22ae24d98a6dde83028e735c0 NETWORK=goerli cargo run -p carbonable-api

migrate:
	DATABASE_URL=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer cargo run -p carbonable-migration 

migrate_down:
	DATABASE_URL=postgres://carbonable:carbonable@localhost:5432/carbonable_indexer cargo run -p carbonable-migration down

start_db:
	docker compose -p carbonable-indexer up -d 

stop_db:
	docker compose -p carbonable-indexer down
