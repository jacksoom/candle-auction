use crate::msg::RandQueryMsg;
use cosmwasm_std::SystemError;
use cosmwasm_std::{from_binary, to_binary, ContractResult, SystemResult};
use cosmwasm_std::{QuerierResult, WasmQuery};

pub fn mock_query_handle(req: &WasmQuery) -> QuerierResult {
    match req {
        WasmQuery::Smart { contract_addr, msg } => {
            if contract_addr.eq("oracle_contract") {
                let _msg: RandQueryMsg = from_binary(msg).unwrap();
                let a = "gio1qdgzfy4vta5p43l4urdtmawka3qv2ldh4h0jat".to_string();

                SystemResult::Ok(ContractResult::Ok(
                    to_binary(&crate::msg::GetResponse {
                        randomness: to_binary(&a).unwrap(),
                    })
                    .unwrap(),
                ))
            } else {
                SystemResult::Err(SystemError::Unknown {})
            }
        }
        _ => SystemResult::Err(SystemError::Unknown {}),
    }
}
