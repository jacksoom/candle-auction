use cosmwasm_schema::cw_serde;
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
    pub oracle_contract: CanonicalAddr,
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
#[cw_serde]
pub enum PaymentType {
    Coin,
    Cw20,
}

#[cw_serde]
pub struct Auction {
    /// The name of the auction item
    pub name: String,
    /// Start Second-level timestamp to bid
    pub start_timestmap: u64,
    /// End second-level timestamp to bid, end_timestamp = start_timestmap + auction_duration
    pub auction_duration: u64,
    /// Bidder infomation
    pub bidders: Vec<(String, u64, u128)>,
    /// Current winner (with bid) who finally won Candle auction.
    /// (bidder_address, bid_timestamp, bid_price)
    pub curr_winner: Option<(String, u64, u128)>,
    /// ERC721 contract
    /// rewarding contract address (NFT or DNS), token id
    pub tokens: Vec<(String, String)>,
    /// Seller
    pub seller: CanonicalAddr,
    /// bid payment type.
    pub payment_type: u8,
    /// bid payment value. denom/cw20 token address
    pub payment: String,
    /// Bid min price
    pub min_price: Option<u128>,
    /// bid num
    pub bid_num: u32,
    /// Auction candle has been blowed
    pub is_candle_blow: bool,
}

impl Auction {
    /// Calc auction status  by the given current time
    /// 1. Not started: start_timestmap < current_timestamp
    /// 2. Ended: current_timestamp > auction_end_time
    /// 3. OpeningPeriod
    pub fn status(&self, curr_timestamp: u64) -> AuctionStatus {
        if self.start_timestmap > curr_timestamp {
            return AuctionStatus::NotStarted;
        }

        if curr_timestamp
            > self
                .start_timestmap
                .checked_add(self.auction_duration)
                .unwrap_or(u64::MAX)
        {
            return AuctionStatus::Ended;
        }

        AuctionStatus::OpeningPeriod
    }

    pub fn bid_min_price(&self) -> u128 {
        if let Some((_, _, amt)) = self.curr_winner {
            u128::max(amt, self.min_price.unwrap_or(0u128))
        } else {
            self.min_price.unwrap_or(0u128)
        }
    }
}

/// Auction statuses
/// logic inspired by
/// [Parachain Auction](https://github.com/paritytech/polkadot/blob/master/runtime/common/src/traits.rs#L160)
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, JsonSchema, Debug)]
pub enum AuctionStatus {
    /// An auction has not started yet.
    NotStarted = 0,
    /// We are in the starting period of the auction, collecting initial bids.
    OpeningPeriod = 1,
    /// Candle was blown
    Ended = 2,
}
