# carbonable-indexer-rs - domain

This is the core domain layer of the application.

It is composed of 2 main components :

- Domain
- Infrastructure

## Domain

It contains base data structures and traits that represent required application features.

## Infrastructure

Contains all concrete implementations of the domain layer.

All the seeding strategies are present in the infrastructure layer.

- `postgres` - seeding smart contracts strategies into database.
- `seed` - logic to pull data onchain from smart contracts to postgres database
- `starknet` - taking input onchain from starknet.
- `view_model` - mapping between domain and view models. From database to whats needing in api routes.
