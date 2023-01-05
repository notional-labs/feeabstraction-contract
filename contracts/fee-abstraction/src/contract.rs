use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Empty, Env, IbcMsg, MessageInfo, QueryRequest,
    Response, StdResult, to_vec, StdError, SystemResult, from_binary, ContractResult,
};

use cw_ibc_query::PacketMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::PACKET_LIFETIME;
use cw_osmo_proto::osmosis::gamm::v1beta1::{QuerySpotPriceRequest, QuerySpotPriceResponse};
use prost::Message;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    PACKET_LIFETIME.save(deps.storage, &msg.packet_lifetime)?;
    Ok(Response::new())
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // stop any coins sent
    cw_utils::nonpayable(&info)?;
    match msg {
        ExecuteMsg::IbcQuery {
            channel_id,
            msgs,
            callback,
        } => execute_ibc_query(deps, env, info, channel_id, msgs, callback),
        ExecuteMsg::OsmoTwapIbcQuery {
            channel_id,
            callback,
            pool_id,
            token_in_denom,
            token_out_denom,
            with_swap_fee,
        } => execute_osmo_twap_ibc_query(
            deps,
            env,
            info,
            channel_id,
            pool_id,
            token_in_denom,
            token_out_denom,
            with_swap_fee,
            callback,
        ),
    }
}

pub fn execute_ibc_query(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    channel_id: String,
    msgs: Vec<QueryRequest<Empty>>,
    callback: String,
) -> Result<Response, ContractError> {
    // validate callback address
    deps.api.addr_validate(&callback)?;

    // construct a packet to send
    let packet = PacketMsg::IbcQuery { msgs, callback };
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
        timeout: env
            .block
            .time
            .plus_seconds(PACKET_LIFETIME.load(deps.storage)?)
            .into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "execute_ibc_query");
    Ok(res)
}

pub fn execute_osmo_twap_ibc_query(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    channel_id: String,
    pool_id: u64,
    token_in_denom: String,
    token_out_denom: String,
    with_swap_fee: bool,
    callback: String,
) -> Result<Response, ContractError> {
    // validate callback address
    deps.api.addr_validate(&callback)?;

    let query_request: QuerySpotPriceRequest = QuerySpotPriceRequest {
        pool_id,
        token_in_denom,
        token_out_denom,
        with_swap_fee,
    };
    let vecu8_query_request = query_request.encode_to_vec();
    let data = Binary::from(vecu8_query_request);

    let query_request: QueryRequest<Empty> = QueryRequest::Stargate {
        path: "/osmosis.gamm.v2.Query/SpotPrice".to_string(),
        data: data,
    };
    let msgs = vec![query_request];

    // construct a packet to send
    let packet = PacketMsg::IbcQuery { msgs, callback };
    let msg = IbcMsg::SendPacket {
        channel_id,
        data: to_binary(&packet)?,
        timeout: env
            .block
            .time
            .plus_seconds(PACKET_LIFETIME.load(deps.storage)?)
            .into(),
    };

    let res = Response::new()
        .add_message(msg)
        .add_attribute("action", "execute_ibc_query");
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::QueryStargateTwap {
            pool_id,
            token_in_denom,
            token_out_denom,
            with_swap_fee,
        } => to_binary(&query_stargate_twap(
            deps,
            pool_id,
            token_in_denom,
            token_out_denom,
            with_swap_fee,
        )?),
    }
}

pub fn query_stargate_twap(
    deps: Deps,
    pool_id: u64,
    token_in_denom: String,
    token_out_denom: String,
    with_swap_fee: bool,
) -> StdResult<Binary> {
    let query_request: QuerySpotPriceRequest = QuerySpotPriceRequest {
        pool_id,
        token_in_denom,
        token_out_denom,
        with_swap_fee,
    };

    let vecu8_query_request = query_request.encode_to_vec();
    let data = Binary::from(vecu8_query_request);

    let query_request: QueryRequest<Empty> = QueryRequest::Stargate {
        path: "/osmosis.gamm.v2.Query/SpotPrice".to_string(),
        data: data,
    };

    let raw = to_vec(&query_request)
        .map_err(|serialize_err| {
            StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
        })
        .unwrap();

    let res = match deps.querier.raw_query(&raw) {
        SystemResult::Err(system_err) => Err(StdError::generic_err(format!(
            "Querier contract error: {}",
            system_err
        ))),
        SystemResult::Ok(ContractResult::Err(contract_err)) => Err(StdError::generic_err(format!(
            "Querier contract error: {}",
            contract_err
        ))),
        SystemResult::Ok(ContractResult::Ok(value)) => Ok(value),
    };
    res
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_ibc_channel_connect_ack, mock_ibc_channel_open_init,
        mock_ibc_channel_open_try, mock_info, MockApi, MockQuerier, MockStorage,
    };
    use cosmwasm_std::OwnedDeps;

    use cw_ibc_query::{APP_ORDER, BAD_APP_ORDER, IBC_APP_VERSION};

    use crate::ibc::{ibc_channel_connect, ibc_channel_open};

    use super::*;

    const CREATOR: &str = "creator";

    fn setup() -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            packet_lifetime: 60u64,
        };
        let info = mock_info(CREATOR, &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn instantiate_works() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            packet_lifetime: 60u64,
        };
        let info = mock_info("creator", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len())
    }

    #[test]
    fn enforce_version_in_handshake() {
        let mut deps = setup();

        let wrong_order = mock_ibc_channel_open_try("channel-12", BAD_APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_order).unwrap_err();

        let wrong_version = mock_ibc_channel_open_try("channel-12", APP_ORDER, "reflect");
        ibc_channel_open(deps.as_mut(), mock_env(), wrong_version).unwrap_err();

        let valid_handshake = mock_ibc_channel_open_try("channel-12", APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), valid_handshake).unwrap();
    }

    #[test]
    fn proper_handshake_flow() {
        let mut deps = setup();
        let channel_id = "channel-1234";

        // first we try to open with a valid handshake
        let handshake_open = mock_ibc_channel_open_init(channel_id, APP_ORDER, IBC_APP_VERSION);
        ibc_channel_open(deps.as_mut(), mock_env(), handshake_open).unwrap();

        // then we connect (with counter-party version set)
        let handshake_connect =
            mock_ibc_channel_connect_ack(channel_id, APP_ORDER, IBC_APP_VERSION);
        let res = ibc_channel_connect(deps.as_mut(), mock_env(), handshake_connect).unwrap();
        assert_eq!(0, res.messages.len());
    }
}
