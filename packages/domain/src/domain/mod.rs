pub mod crypto;
pub mod event_source;
pub mod project;

pub trait Contract {}

/// Structure representing an ERC-721 smart contract
pub struct Erc721;
impl Contract for Erc721 {}

/// Structure representing an ERC-3525 smart contract
pub struct Erc3525;
impl Contract for Erc3525 {}
