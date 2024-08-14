# carbonable-indexer-rs

Api package is the open door of main application features.

It is composed of 4 main components :

- Launchpad
- Portfolio
- Farming
- Project

## Launchpad

Launchpad is in charge of displaying all projects launched on carbonable and some information about how to buy them.

## Portfolio

Portfolio is in charge of keeping track of user's token ids and value associated to them.

## Farming

Farming is in charge of making interraction from dapp to smart contract a bit easier.

## Project

Project is the base data layer containing all informations stored in smart contracts

## Route hierarchy

Is defined like so :

- `/latest/block` : get latest indexed block number
- `/config` : get configuration of current network
- `/portfolio/{wallet}` : get all token ids and value associated to a given wallet
  - for historical reasons, this endpoints can deal with both erc3525 and erc721 tokens
  - for next version, only erc3525 tokens will be supported more precisely, 1155 will be the next standard
  - it is also able to deal with different versions and implementations of the metadata
- `/projects/{slug}` : get all informations about a given project
- `/farming/claim-all/{wallet}` : claim all unclaimed tokens for a given wallet
  - based on user's wallet, returns the list of yielders on which claiming is possible.
- `/farming/list` : get all projects launched on carbonable
  - projects where farming is enabled
- `/farming/list/global/{wallet}` : get all projects launched on carbonable for a given wallet
  - global are values that are not associated to a specific project but are the sum of tokens accross projects for a given wallet (total value, total resell claimable, total offset claimable)
- `/farming/list/unconnected/{slug}` : get all projects launched on carbonable for a given project
  - this endpoint is used to get values based on projects and that is not dependent on user wallet to find values
- `/farming/list/{wallet}/{slug}` : get all projects launched on carbonable for a given wallet and project
  - this endpoint is dependant on user wallet to find values (resell claimable, offset claimable, total_deposited and so on)
- `/farming/details/{wallet}/{slug}` : get all informations about a given project for a given wallet
  - deeper project informations based on slug to display detail page about a project.
- `/launchpad/list` : get all projects launched on carbonable
- `/launchpad/details/{slug}` : get all informations about a given project
