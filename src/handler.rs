use cosmwasm_std::{
    from_binary, to_binary, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Response, Uint128, WasmMsg,
};

pub mod execute {
    use super::*;
    use crate::error::ContractError;
    use crate::msg::Auction as AuctionMsg;
    use crate::msg::TokenMsg;
    use crate::state::*;
    use cosmwasm_std::Binary;
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

        if now < auction.start_timestmap
            || now
                > auction
                    .start_timestmap
                    .checked_add(auction.auction_duration)
                    .unwrap_or(u64::MAX)
        {
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

        let min_price = if let Some((_, amt)) = auction.curr_winner {
            u128::max(amt, auction.min_price.unwrap_or(0u128))
        } else {
            auction.min_price.unwrap_or(0u128)
        };

        if fund.amount.u128() < min_price {
            return Err(ContractError::AuctionPriceTooLow {
                min_price,
                current: fund.amount.u128(),
            });
        }

        let bidder = bidder.unwrap_or_else(|| info.sender.to_string());

        // Update auction status
        auction.bid_num += 1;
        auction.bidders.push((bidder.clone(), fund.amount.u128()));
        auction.curr_winner = Some((bidder, fund.amount.u128()));

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

        let now = env.block.time.seconds();

        if now
            < auction
                .start_timestmap
                .checked_add(auction.auction_duration)
                .unwrap_or(u64::MAX)
        {
            return Err(ContractError::BadRequest {
                msg: "Auction is not ended".to_string(),
            });
        }

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
        if now
            >= auction
                .start_timestmap
                .checked_add(auction.auction_duration)
                .unwrap_or(u64::MAX)
        {
            return Err(ContractError::BadRequest {
                msg: "Auction was ended!".to_string(),
            });
        }

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

        let now = env.block.time.seconds();
        if now < auction.start_timestmap
            || now
                > auction
                    .start_timestmap
                    .checked_add(auction.auction_duration)
                    .unwrap_or(u64::MAX)
        {
            return Err(ContractError::BadRequest {
                msg: "Auction was ended!".to_string(),
            });
        }

        let min_price = if let Some((_, amt)) = auction.curr_winner {
            u128::max(amt, auction.min_price.unwrap_or(0u128))
        } else {
            auction.min_price.unwrap_or(0u128)
        };

        // check recv amount gt min price
        if amount.u128() < min_price {
            return Err(ContractError::AuctionPriceTooLow {
                min_price,
                current: amount.u128(),
            });
        }

        let mut msgs = vec![];
        // CW20: refund to previous round winner
        if let Some((addr, amt)) = auction.curr_winner {
            let refund_msg = Cw20ExecuteMsg::Transfer {
                recipient: addr,
                amount: Uint128::new(amt),
            };

            let msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: auction.pay_token.clone().unwrap(),
                msg: to_binary(&refund_msg)?,
                funds: vec![],
            });
            msgs.push(msg);
        }

        let bidder = auction_msg.bidder.unwrap_or(sender);

        auction.curr_winner = Some((bidder.clone(), amount.u128()));
        auction.bid_num += 1;
        auction.bidders.push((bidder, amount.u128()));

        AUCTIONS.save(deps.storage, auction_msg.id, &auction)?;

        Ok(Response::new()
            .add_attribute("method", "handle_cw20_bid")
            .add_messages(msgs))
    }

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
}
