use cosmwasm_std::CanonicalAddr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CONFIG: Item<Config> = Item::new("CONFIG");
pub const AUCTIONS: Map<u64, Auction> = Map::new("AUCTIONS"); // AUCTIONS record

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Config {
    pub auction_num: u64,
    pub min_auction_duration: u64,
    pub max_auction_duration: u64,
    pub enable_auction: bool,
    pub fee_rate: u64,
    pub default_denom: String,
    pub support_contract: Vec<String>,
    pub version: ContractVersion,
    pub owner: CanonicalAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct ContractVersion {
    /// contract is the crate name of the implementing contract, eg. `crate:cw20-base`
    /// we will use other prefixes for other languages, and their standard global namespacing
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.7.0", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to it's implementation somehow)
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub struct Auction {
    /// The name of the auction item
    pub name: String,
    /// Start Second-level timestamp to bid
    pub start_timestmap: u64,
    /// End second-level timestamp to bid, end_timestamp = start_timestmap + auction_duration
    pub auction_duration: u64,
    /// Bidder infomation
    pub bidders: Vec<(String, u128)>,
    /// Current winner (with bid) who finally won Candle auction
    pub curr_winner: Option<(String, u128)>,
    /// ERC721 contract
    /// rewarding contract address (NFT or DNS), token id
    pub tokens: Vec<(String, String)>,
    /// Seller
    pub seller: CanonicalAddr,
    /// bid denom
    pub denom: Option<String>,
    /// CW20 token bid
    pub pay_token: Option<String>,
    /// Bid min price
    pub min_price: Option<u128>,
    /// bid num
    pub bid_num: u32,
}

/// Auction statuses
/// logic inspired by
/// [Parachain Auction](https://github.com/paritytech/polkadot/blob/master/runtime/common/src/traits.rs#L160)
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub enum AuctionStatus {
    /// An auction has not started yet.
    NotStarted,
    /// We are in the starting period of the auction, collecting initial bids.
    OpeningPeriod,
    /// We are in the ending period of the auction, where we are taking snapshots of the winning
    /// bids. Snapshots are taken currently on per-block basis, but this logic could be later evolve
    /// to take snapshots of on arbitrary length (in blocks)
    EndingPeriod,
    /// Candle was blown
    Ended,
}
