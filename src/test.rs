mod tests {
    use crate::contract::execute;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Attribute;

    use crate::msg::*;

    use crate::contract::instantiate;
    use cosmwasm_std::{coins, to_binary, Addr, CosmosMsg, Timestamp, Uint128, WasmMsg};
    use cw20::Cw20ExecuteMsg;
    use cw721::Cw721ExecuteMsg;

    const TEST_DENOM: &str = "ugtb";

    #[test]
    fn test_init() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            min_auction_duration: 0,
            max_auction_duration: 2 * 30 * 24 * 3600,
            enable_auction: true,
            fee_rate: 2,
            default_denom: TEST_DENOM.to_string(),
            support_contract: vec!["cw20_contract_addr1".to_string()],
        };

        let info = mock_info("admin", &coins(0, TEST_DENOM.to_string()));
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(2, res.attributes.len());
    }

    #[test]
    fn test_post_auction() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            min_auction_duration: 0,
            max_auction_duration: 2 * 30 * 3600,
            enable_auction: true,
            fee_rate: 2,
            default_denom: TEST_DENOM.to_string(),
            support_contract: vec!["cw20_contract_addr1".to_string()],
        };

        let info = mock_info("admin", &coins(0, TEST_DENOM.to_string()));
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let auction_msg = ExecuteMsg::Auction {
            name: "test_auction_1".to_string(),
            start_timestmap: 1664805457,
            duration: 2 * 30 * 24 * 3600,
            tokens: vec![],
            denom: Some("ugtb".to_string()),
            pay_token: None,
            min_price: Some(123),
        };

        let res = execute(deps.as_mut(), mock_env(), info.clone(), auction_msg).unwrap();
        let attris = vec![
            Attribute {
                key: "method".to_string(),
                value: "auction".to_string(),
            },
            Attribute {
                key: "name".to_string(),
                value: "test_auction_1".to_string(),
            },
            Attribute {
                key: "start_timestmap".to_string(),
                value: "1664805457".to_string(),
            },
            Attribute {
                key: "auction_duration".to_string(),
                value: (2 * 30 * 24 * 3600).to_string(),
            },
            Attribute {
                key: "seller".to_string(),
                value: "admin".to_string(),
            },
            Attribute {
                key: "denom".to_string(),
                value: "ugtb".to_string(),
            },
            Attribute {
                key: "pay_token".to_string(),
                value: "".to_string(),
            },
            Attribute {
                key: "min_price".to_string(),
                value: "123".to_string(),
            },
        ];

        assert_eq!(res.attributes, attris, "failed");
        // TODO:add query check
    }

    #[test]
    fn test_recv_cw20() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            min_auction_duration: 0,
            max_auction_duration: 2 * 24 * 30 * 3600,
            enable_auction: true,
            fee_rate: 2,
            default_denom: TEST_DENOM.to_string(),
            support_contract: vec!["cw20_contract_addr1".to_string()],
        };

        let mut info = mock_info("admin", &coins(0, TEST_DENOM.to_string()));
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let post_auction_msg = ExecuteMsg::Auction {
            name: "test_auction_1".to_string(),
            start_timestmap: 1571797400,
            duration: 2 * 30 * 24 * 3600,
            tokens: vec![],
            denom: None,
            pay_token: Some("cw20_contract_addr1".to_string()),
            min_price: Some(123),
        };

        execute(deps.as_mut(), mock_env(), info.clone(), post_auction_msg).unwrap();

        // First auction bid success
        let auction_msg = Auction {
            id: 1,
            bidder: None,
        };

        let token_msg = TokenMsg::Cw20ReceiveMsg {
            sender: "admin1".to_string(),
            amount: Uint128::new(200u128),
            msg: to_binary(&auction_msg).unwrap(),
        };

        info.sender = Addr::unchecked("cw20_contract_addr1");

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(token_msg),
        )
        .unwrap();
        assert_eq!(res.attributes.len(), 1, "res.attributes is not expect");

        // Second auction success and the first auction bid will be refund
        let auction_msg = Auction {
            id: 1,
            bidder: None,
        };

        let token_msg = TokenMsg::Cw20ReceiveMsg {
            sender: "admin2".to_string(),
            amount: Uint128::new(300u128),
            msg: to_binary(&auction_msg).unwrap(),
        };

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(token_msg),
        )
        .unwrap();

        let refund_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cw20_contract_addr1".to_string(),
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: "admin1".to_string(),
                amount: Uint128::new(200u128),
            })
            .unwrap(),
            funds: vec![],
        });

        assert_eq!(res.attributes.len(), 1, "res.attributes is not expect");
        assert_eq!(res.messages[0].msg, refund_msg, "message error");
    }

    #[test]
    fn test_auction_and_recv_cw721() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            min_auction_duration: 0,
            max_auction_duration: 2 * 24 * 30 * 3600,
            enable_auction: true,
            fee_rate: 2,
            default_denom: TEST_DENOM.to_string(),
            support_contract: vec![
                "cw20_contract_addr".to_string(),
                "cw721_contract_addr".to_string(),
            ],
        };

        let mut info = mock_info("alice", &coins(0, TEST_DENOM.to_string()));
        instantiate(deps.as_mut(), env, info.clone(), msg).unwrap();

        let post_auction_msg = ExecuteMsg::Auction {
            name: "test_auction_1".to_string(),
            start_timestmap: 1571797400,
            duration: 2 * 30 * 24 * 3600,
            tokens: vec![],
            denom: None,
            pay_token: Some("cw20_contract_addr1".to_string()),
            min_price: Some(123),
        };

        execute(deps.as_mut(), mock_env(), info.clone(), post_auction_msg).unwrap();

        let auction_msg = Auction {
            id: 1,
            bidder: None,
        };

        let token_msg = TokenMsg::Cw721ReceiveMsg {
            sender: "alice".to_string(),
            token_id: "test_token".to_string(),
            msg: to_binary(&auction_msg).unwrap(),
        };

        info.sender = Addr::unchecked("cw721_contract_addr");

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(token_msg),
        )
        .unwrap();
        assert_eq!(res.attributes.len(), 1, "res.attributes is not expect");

        // Second auction success and the first auction bid will be refund
        let auction_msg = Auction {
            id: 1,
            bidder: None,
        };

        info.sender = Addr::unchecked("cw20_contract_addr1");

        let token_msg1 = TokenMsg::Cw20ReceiveMsg {
            sender: "bob".to_string(),
            amount: Uint128::new(300u128),
            msg: to_binary(&auction_msg).unwrap(),
        };

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(token_msg1),
        )
        .unwrap();

        assert!(res.attributes.len() == 1, "attri");

        let token_msg2 = TokenMsg::Cw20ReceiveMsg {
            sender: "keven".to_string(),
            amount: Uint128::new(400u128),
            msg: to_binary(&auction_msg).unwrap(),
        };

        let res = execute(
            deps.as_mut(),
            mock_env(),
            info.clone(),
            ExecuteMsg::Receive(token_msg2),
        )
        .unwrap();
        assert!(res.attributes.len() == 1, "attri");
        assert_eq!(res.messages.len(), 1, "yes");

        // Keven reclaim winner tokens
        let claim_msg = ExecuteMsg::WinnerClaim {
            auction_id: 1,
            winner: Some("keven".to_string()),
        };

        let mut end_env = mock_env();
        end_env.block.time = Timestamp::from_seconds(1571797400 + 100000000);

        let res = execute(deps.as_mut(), end_env, info.clone(), claim_msg).unwrap();
        assert_eq!(res.attributes.len(), 1, "attri error");

        let transfer_msg: CosmosMsg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "cw721_contract_addr".to_string(),
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: "keven".to_string(),
                token_id: "test_token".to_string(),
            })
            .unwrap(),
            funds: vec![],
        });

        assert_eq!(res.messages[0].msg, transfer_msg, "made transfer");
    }
}
