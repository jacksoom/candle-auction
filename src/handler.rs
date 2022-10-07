use cosmwasm_std::{
    from_binary, to_binary, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};

const DRAND_NEXT_ROUND_SECURITY: u64 = 10;

pub mod execute {
    //{{{
    use super::*;
    use crate::error::ContractError;
    use crate::msg::{Auction as AuctionMsg, TokenMsg};
    use crate::state::*;
    use cosmwasm_std::{BankMsg, Binary, Coin};
    use cw20::Cw20ExecuteMsg;

    #[allow(clippy::too_many_arguments)]
    pub fn auction(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        name: String,
        start_timestmap: u64,
        auction_duration: u64,
        tokens: Vec<(String, String)>,
        denom: Option<String>,
        pay_token: Option<String>,
        min_price: Option<u128>,
    ) -> Result<Response, ContractError> {
        let mut config = CONFIG.load(deps.storage)?;
        let now = env.block.time.seconds();
        if now > start_timestmap + auction_duration {
            return Err(ContractError::BadRequest {
                msg: "Bad timestamp setting".to_string(),
            });
        }

        if (denom.is_some() && pay_token.is_some()) || (denom.is_none() && pay_token.is_none()) {
            return Err(ContractError::BadRequest {
                msg: "Bad payment setting".to_string(),
            });
        }

        // TODO
        // auction params precheck
        // check contract has been received those tokens
        let auction = Auction {
            name,
            start_timestmap,
            auction_duration,
            bidders: vec![],
            curr_winner: None,
            tokens,
            seller: deps.api.addr_canonicalize(info.sender.as_str())?,
            denom,
            pay_token,
            min_price,
            bid_num: 0,
            is_candle_blow: false,
        };

        let auction_id = config.auction_num + 1;

        AUCTIONS.save(deps.storage, auction_id, &auction)?;

        config.auction_num += 1;
        CONFIG.save(deps.storage, &config)?;

        Ok(Response::new()
            .add_attribute("method", "auction")
            .add_attribute("name", auction.name)
            .add_attribute("start_timestmap", auction.start_timestmap.to_string())
            .add_attribute("auction_duration", auction.auction_duration.to_string())
            .add_attribute("seller", info.sender.to_string())
            .add_attribute("denom", auction.denom.unwrap_or_default())
            .add_attribute("pay_token", auction.pay_token.unwrap_or_default())
            .add_attribute(
                "min_price",
                auction.min_price.unwrap_or_default().to_string(),
            ))
    }

    pub fn bid_for_denom(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        bidder: Option<String>,
        auction_id: u64,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        assert!(config.enable_auction, "Auction disabled");

        let mut auction = AUCTIONS.may_load(deps.storage, auction_id)?.unwrap();

        let now = env.block.time.seconds();

        if !auction.status(now).eq(&AuctionStatus::OpeningPeriod) {
            return Err(ContractError::NotOpeningPeriod {
                start: auction.start_timestmap,
                end: auction
                    .start_timestmap
                    .checked_add(auction.auction_duration)
                    .unwrap_or(u64::MAX),
            });
        }

        // check bid price
        let default_fund = &Coin {
            denom: auction.denom.clone().unwrap(),
            amount: Uint128::from(0u128),
        };

        let fund = info
            .funds
            .iter()
            .find(|fund| fund.denom.eq(&auction.denom.clone().unwrap()))
            .unwrap_or(default_fund);

        let min_price = auction.bid_min_price();

        if fund.amount.u128() < min_price {
            return Err(ContractError::AuctionPriceTooLow {
                min_price,
                current: fund.amount.u128(),
            });
        }

        let bidder = bidder.unwrap_or_else(|| info.sender.to_string());

        // Update auction status
        auction.bid_num += 1;
        auction
            .bidders
            .push((bidder.clone(), now, fund.amount.u128()));
        auction.curr_winner = Some((bidder, now, fund.amount.u128()));

        AUCTIONS.save(deps.storage, auction_id, &auction)?;

        Ok(Response::new())
    }

    use cw721::Cw721ExecuteMsg;

    pub fn winner_claim(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        auction_id: u64,
        winner: Option<String>,
    ) -> Result<Response, ContractError> {
        let auction = AUCTIONS.may_load(deps.storage, auction_id)?.unwrap();

        let winner = winner.unwrap_or_else(|| info.sender.to_string());

        assert_eq!(
            auction.status(env.block.time.seconds()),
            AuctionStatus::Ended,
            "Auction is not ended"
        );

        if auction.curr_winner.is_none() || !auction.curr_winner.as_ref().unwrap().0.eq(&winner) {
            return Err(ContractError::BadRequest {
                msg: "Not Winner".to_string(),
            });
        }

        let mut messages: Vec<CosmosMsg> = vec![];
        // Transfer all nft
        for token in auction.tokens.clone() {
            let transfer_nft_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: token.0,
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: winner.clone(),
                    token_id: token.1,
                })?,
                funds: vec![],
            });

            messages.push(transfer_nft_msg);
        }

        AUCTIONS.save(deps.storage, auction_id, &auction)?;

        // TODO:transfer the denom to seller

        Ok(Response::new()
            .add_attribute("method", "winner_claim")
            .add_messages(messages))
    }

    pub fn handle_cw721(
        deps: DepsMut,
        info: MessageInfo,
        env: Env,
        sender: String,
        token_id: String,
        msg: Binary,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;

        if !config.support_contract.contains(&info.sender.to_string()) {
            return Err(ContractError::BadRequest {
                msg: "Unsupport contract!".to_string(),
            });
        }

        let auction_msg: AuctionMsg = from_binary(&msg)?;

        let mut auction = AUCTIONS.load(deps.storage, auction_msg.id)?;
        let now = env.block.time.seconds();

        assert_eq!(
            auction.status(now),
            AuctionStatus::NotStarted,
            "Auction status is now NotStarted"
        );

        assert_eq!(
            deps.api.addr_humanize(&auction.seller).unwrap().to_string(),
            sender,
            "now owner"
        );

        auction.tokens.push((info.sender.to_string(), token_id));

        AUCTIONS.save(deps.storage, auction_msg.id, &auction)?;

        Ok(Response::new().add_attribute("method", "handle_cw721"))
    }

    /// handle receive cw20 token bid request
    /// Bid rules
    /// 1: Receive token is correct
    /// 2: Whether the time can be bid
    /// 3: Highest bid price
    /// If eligible all bid rules. the bidder be the current winner. and refund to previous winner
    pub fn handle_cw20_bid(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        sender: String,
        amount: Uint128,
        msg: Binary,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        assert!(config.enable_auction, "Auction has been disabled!");

        let auction_msg: AuctionMsg = from_binary(&msg)?;

        let mut auction = AUCTIONS.load(deps.storage, auction_msg.id)?;

        let now = env.block.time.seconds();

        assert_eq!(
            auction.status(now),
            AuctionStatus::OpeningPeriod,
            "Cannot bid right now"
        );

        if !auction
            .pay_token
            .clone()
            .unwrap_or_default()
            .eq(&info.sender.to_string())
        {
            return Err(ContractError::BadRequest {
                msg: "Unsupport contract!".to_string(),
            });
        }

        let min_price = auction.bid_min_price();

        // check recv amount gt min price
        if amount.u128() < min_price {
            return Err(ContractError::AuctionPriceTooLow {
                min_price,
                current: amount.u128(),
            });
        }

        let bidder = auction_msg.bidder.unwrap_or(sender);
        auction.curr_winner = Some((bidder.clone(), now, amount.u128()));
        auction.bid_num += 1;
        auction.bidders.push((bidder, now, amount.u128()));

        AUCTIONS.save(deps.storage, auction_msg.id, &auction)?;

        Ok(Response::new().add_attribute("method", "handle_cw20_bid"))
    }

    use super::DRAND_NEXT_ROUND_SECURITY;
    use crate::msg::RandQueryMsg;
    use cosmwasm_std::WasmQuery;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    pub fn blow_candle(
        deps: DepsMut,
        env: Env,
        auction_id: u64,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        let mut auction = AUCTIONS.load(deps.storage, auction_id)?;
        let now = env.block.time.seconds();
        assert_eq!(
            auction.status(now),
            AuctionStatus::Ended,
            "Auction status is now ended"
        );

        assert!(auction.curr_winner.is_some(), "Auction flow");

        let rand_key = auction_id + DRAND_NEXT_ROUND_SECURITY;

        let msg = RandQueryMsg::Get { round: rand_key };
        let wasm = WasmQuery::Smart {
            contract_addr: deps.api.addr_humanize(&config.oracle_contract)?.to_string(),
            msg: to_binary(&msg)?,
        };

        let res: crate::msg::GetResponse = deps.querier.query(&wasm.into())?;
        let mut hasher = DefaultHasher::new();
        res.randomness.hash(&mut hasher);

        let offset = hasher.finish() % auction.auction_duration;

        let mut bank_msgs = vec![];
        let mut cw20_refund_msg = vec![];

        let end_time = offset
            .checked_add(auction.start_timestmap)
            .unwrap_or(u64::MAX);

        auction.curr_winner = None;

        for (bidder, bid_time, amount) in auction.bidders.iter().rev() {
            if *bid_time <= end_time && auction.curr_winner.is_none() {
                auction.curr_winner = Some((bidder.clone(), *bid_time, *amount));
                continue;
            }

            // Refund Non winner
            if auction.denom.is_none() {
                let refund_msg = Cw20ExecuteMsg::Transfer {
                    recipient: bidder.clone(),
                    amount: Uint128::new(*amount),
                };

                let msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: auction.pay_token.clone().unwrap(),
                    msg: to_binary(&refund_msg)?,
                    funds: vec![],
                });
                cw20_refund_msg.push(msg);
            } else {
                bank_msgs.push(BankMsg::Send {
                    to_address: bidder.clone(),
                    amount: vec![Coin {
                        denom: auction.denom.clone().unwrap(),
                        amount: Uint128::new(*amount),
                    }],
                });
            }
        }

        auction.is_candle_blow = true;

        AUCTIONS.save(deps.storage, auction_id, &auction)?;

        Ok(Response::new()
            .add_messages(bank_msgs)
            .add_messages(cw20_refund_msg))
    }

    use cw721::Cw721ExecuteMsg::TransferNft;
    pub fn receive(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        cw20_msg: TokenMsg,
    ) -> Result<Response, ContractError> {
        match cw20_msg {
            TokenMsg::Cw20ReceiveMsg {
                sender,
                amount,
                msg,
            } => handle_cw20_bid(deps, env, info, sender, amount, msg),
            TokenMsg::Cw721ReceiveMsg {
                sender,
                token_id,
                msg,
            } => handle_cw721(deps, info, env, sender, token_id, msg),
        }
    }

    /// If the auction was flow. return the token of the seller
    pub fn auction_flow(
        deps: DepsMut,
        env: Env,
        auction_id: u64,
    ) -> Result<Response, ContractError> {
        let mut auction = AUCTIONS.load(deps.storage, auction_id)?;
        let now = env.block.time.seconds();
        assert_eq!(
            auction.status(now),
            AuctionStatus::Ended,
            "auction status is not done"
        );

        assert!(auction.curr_winner.is_none(), "Auction not flow");
        assert!(!auction.is_candle_blow, "Auction is not blow right now");

        let mut msgs = vec![];
        for (addr, token_id) in auction.tokens.clone() {
            let refund_token_msg = TransferNft {
                recipient: deps.api.addr_humanize(&auction.seller)?.to_string(),
                token_id,
            };

            let msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: addr,
                msg: to_binary(&refund_token_msg)?,
                funds: vec![],
            });

            msgs.push(msg);
        }

        auction.is_candle_blow = true;
        AUCTIONS.save(deps.storage, auction_id, &auction)?;

        Ok(Response::new().add_messages(msgs))
    }
} //}}}

pub mod query {
    //{{{
    use crate::msg::response;
    use crate::state::{AuctionStatus, AUCTIONS, CONFIG};
    use cosmwasm_std::Env;
    use cosmwasm_std::{Deps, StdResult};

    pub fn config(deps: Deps) -> StdResult<response::Config> {
        let config = CONFIG.load(deps.storage)?;
        Ok(response::Config {
            auction_num: config.auction_num,
            min_auction_duration: config.min_auction_duration,
            max_auction_duration: config.max_auction_duration,
            enable_auction: config.enable_auction,
            fee_rate: config.fee_rate,
            default_denom: config.default_denom,
            support_contract: config.support_contract,
        })
    }

    pub fn auction_list(
        deps: Deps,
        env: Env,
        status: Option<AuctionStatus>,
        page: u32,
        limit: u32,
    ) -> StdResult<Option<Vec<response::Auction>>> {
        let config = CONFIG.load(deps.storage)?;
        let start_amount = page * limit;
        let mut count = 0;
        let mut res = vec![];

        for i in (0..(config.auction_num as usize)).rev() {
            let auction = AUCTIONS.load(deps.storage, i as u64)?;
            if let Some(status) = status.to_owned() {
                if auction.status(env.block.time.seconds()) != status {
                    continue;
                }
            }

            if count >= start_amount {
                res.push(response::Auction {
                    name: auction.name,
                    start_timestmap: auction.start_timestmap,
                    auction_duration: auction.auction_duration,
                    bidders: auction.bidders,
                    curr_winner: auction.curr_winner,
                    tokens: auction.tokens,
                    seller: deps.api.addr_humanize(&auction.seller)?,
                    denom: auction.denom,
                    pay_token: auction.pay_token,
                    min_price: auction.min_price,
                    bid_num: auction.bid_num,
                });
            }

            if res.len() >= limit as usize {
                break;
            }

            count += 1;
        }

        Ok(Some(res))
    }

    pub fn auction(deps: Deps, auction_id: u64) -> StdResult<Option<response::Auction>> {
        let auction_res = AUCTIONS.load(deps.storage, auction_id);
        match auction_res {
            Ok(auction) => Ok(Some(response::Auction {
                name: auction.name,
                start_timestmap: auction.start_timestmap,
                auction_duration: auction.auction_duration,
                bidders: auction.bidders,
                curr_winner: auction.curr_winner,
                tokens: auction.tokens,
                seller: deps.api.addr_humanize(&auction.seller)?,
                denom: auction.denom,
                pay_token: auction.pay_token,
                min_price: auction.min_price,
                bid_num: auction.bid_num,
            })),
            Err(_) => Ok(None),
        }
    }
} //}}}
