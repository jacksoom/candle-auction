use crate::error::ContractError;
use crate::handler::*;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, ContractVersion, CONFIG};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::{get_contract_version, set_contract_version};

const CONTRACT_NAME: &str = "crates.io:candle_auction";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let owner = deps.api.addr_canonicalize(info.sender.as_str())?;
    let oracle_contract = deps.api.addr_canonicalize(&msg.oracle_contract)?;

    CONFIG.save(
        deps.storage,
        &Config {
            auction_num: 0,
            min_auction_duration: msg.min_auction_duration,
            max_auction_duration: msg.max_auction_duration,
            enable_auction: msg.enable_auction,
            fee_rate: msg.fee_rate,
            default_denom: msg.default_denom,
            support_contract: msg.support_contract,
            version: ContractVersion {
                contract: CONTRACT_NAME.to_string(),
                version: CONTRACT_VERSION.to_string(),
            },
            owner,
            oracle_contract,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Auction {
            name,
            start_timestmap,
            duration,
            tokens,
            payment_type,
            payment,
            min_price,
        } => execute::auction(
            deps,
            env,
            info,
            name,
            start_timestmap,
            duration,
            tokens,
            payment_type,
            payment,
            min_price,
        ),
        ExecuteMsg::Receive(msg) => execute::receive(deps, env, info, msg),
        ExecuteMsg::WinnerClaim { auction_id, winner } => {
            execute::winner_claim(deps, env, info, auction_id, winner)
        }
        ExecuteMsg::BlowCandle { auction_id } => execute::blow_candle(deps, env, auction_id),
        ExecuteMsg::FlowRefund { auction_id } => execute::auction_flow(deps, env, auction_id),
        ExecuteMsg::BidForDenom { bidder, auction_id } => {
            execute::bid_for_denom(deps, env, info, bidder, auction_id)
        }
        _ => Err(ContractError::InvalidName {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query::config(deps)?),
        QueryMsg::Auction { id } => to_binary(&query::auction(deps, id)?),
        QueryMsg::AuctionList {
            status,
            page,
            limit,
        } => to_binary(&query::auction_list(deps, env, status, page, limit)?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, _env: Env, _: MigrateMsg) -> Result<Response, ContractError> {
    let ver = get_contract_version(deps.storage)?;
    // ensure we are migrating from an allowed contract
    if ver.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }

    if ver.version.gt(&CONTRACT_VERSION.to_string()) {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

    // set the new version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}
