use crate::state::{AuctionStatus, PaymentType};
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Binary, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Auction init msg
#[cw_serde]
pub struct InstantiateMsg {
    pub min_auction_duration: u64,
    pub max_auction_duration: u64,
    pub enable_auction: bool,
    pub fee_rate: u64,
    pub default_denom: String,
    pub support_contract: Vec<String>,
    pub oracle_contract: String,
}

/// Auction warrper message

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Auction {
    pub id: u64,
    pub bidder: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    // /// Post a new auction
    Auction {
        name: String,
        start_timestmap: u64,
        duration: u64,
        tokens: Vec<(String, String)>,
        payment_type: PaymentType,
        payment: String,
        min_price: Option<u128>,
    },
    /// Winner claim the reward
    WinnerClaim {
        auction_id: u64,
        winner: Option<String>,
    },
    /// Update Config
    UpdateConfig {
        min_auction_duration: Option<u64>,
        max_auction_duration: Option<u64>,
        enable_auction: Option<bool>,
        fee_rate: Option<u64>,
        default_denom: Option<String>,
        support_contract: Option<Vec<String>>,
    },
    /// Candle blow
    BlowCandle { auction_id: u64 },
    /// Receive interface
    Receive(ReceiveMsg),
    /// auction flow refund
    FlowRefund { auction_id: u64 },
    /// Bid for denom payment
    BidForDenom {
        bidder: Option<String>,
        auction_id: u64,
    },
}

#[cw_serde]
pub enum QueryMsg {
    /// Get auction static config
    Config {},
    /// Get Auction list
    AuctionList {
        status: Option<AuctionStatus>,
        page: u32,
        limit: u32,
    },
    /// Get auction by auction id
    Auction { id: u64 },
}

#[cw_serde]
pub enum RandQueryMsg {
    Get { round: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct GetResponse {
    /// The randomness if available. When the beacon does not exist, this is an empty value.
    pub randomness: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct ReceiveMsg {
    pub sender: String,
    pub amount: Option<Uint128>,
    pub token_id: Option<String>,
    pub msg: Binary,
}

pub mod response {
    use super::*;
    #[cw_serde]
    pub struct Config {
        pub auction_num: u64,
        pub min_auction_duration: u64,
        pub max_auction_duration: u64,
        pub enable_auction: bool,
        pub fee_rate: u64,
        pub default_denom: String,
        pub support_contract: Vec<String>,
    }

    #[cw_serde]
    pub struct Auction {
        pub name: String,
        pub start_timestmap: u64,
        pub auction_duration: u64,
        pub bidders: Vec<(String, u64, u128)>,
        pub curr_winner: Option<(String, u64, u128)>,
        pub tokens: Vec<(String, String)>,
        pub seller: Addr,
        pub payment_type: PaymentType,
        pub payment: String,
        pub min_price: Option<u128>,
        pub bid_num: u32,
    }
}

#[cw_serde]
pub struct MigrateMsg {}
