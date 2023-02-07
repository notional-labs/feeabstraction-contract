use cosmwasm_std::{
    entry_point, from_binary, from_slice, to_binary, to_vec, Binary, ContractResult, Deps, DepsMut,
    Empty, Env, Event, Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcOrder, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, QuerierResult, QueryRequest,
    StdError, StdResult, SystemResult, WasmMsg,
};
use cw_osmo_proto::osmosis::gamm::v1beta1::QuerySpotPriceRequest;
use prost::Message;

use crate::error::{ContractError, QueryError, QueryResult};
use crate::msg::{CallResult, IbcQueryRequestTwapResponse, IbcStargate};
use crate::state::PENDING;
use crate::{APP_ORDER, IBC_APP_VERSION};

#[entry_point]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> StdResult<IbcChannelOpenResponse> {
    let channel = msg.channel();

    if channel.order != APP_ORDER {
        return Err(StdError::generic_err("Only supports unordered channels"));
    }

    // In ibcv3 we don't check the version string passed in the message
    // and only check the counterparty version.
    if let Some(counter_version) = msg.counterparty_version() {
        if counter_version != IBC_APP_VERSION {
            return Err(StdError::generic_err(format!(
                "Counterparty version must be `{}`",
                IBC_APP_VERSION
            )));
        }
    }

    // We return the version we need (which could be different than the counterparty version)
    Ok(Some(Ibc3ChannelOpenResponse {
        version: IBC_APP_VERSION.to_string(),
    }))
}

#[entry_point]
/// once it's established, we create the reflect contract
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    let chan_id = &channel.endpoint.channel_id;

    // store the channel id for the reply handler
    PENDING.save(deps.storage, chan_id)?;

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_connect")
        .add_attribute("channel_id", chan_id)
        .add_event(Event::new("ibc").add_attribute("channel", "connect")))
}

#[entry_point]
/// On closed channel, we take all tokens from reflect contract to this contract.
/// We also delete the channel entry from accounts.
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    let channel = msg.channel();
    // get contract address and remove lookup
    let channel_id = channel.endpoint.channel_id.as_str();

    Ok(IbcBasicResponse::new()
        .add_attribute("action", "ibc_close")
        .add_attribute("channel_id", channel_id))
}

#[entry_point]
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    // put this in a closure so we can convert all error responses into acknowledgements
    (|| {
        let ibc_msgs: Vec<IbcStargate>;
        let decoded_data: StdResult<Vec<IbcStargate>> = from_binary(&msg.packet.data);
        match decoded_data {
            Ok(ibc_query_req) => ibc_msgs = ibc_query_req,
            Err(error) => return Err(StdError::generic_err(format!("Serilize error: {}", error))),
        }

        let mut query_requests: Vec<QueryRequest<Empty>> = vec![];
        for ibc_msg in ibc_msgs {
            query_requests.push(QueryRequest::Stargate {
                path: ibc_msg.path,
                data: ibc_msg.data,
            })
        }

        let mut result: Vec<CallResult> = vec![CallResult::default(); query_requests.len()];

        for i in 0..query_requests.len() {
            let query_request = query_requests[i].clone();
            let raw = to_vec(&query_request)
                .map_err(|serialize_err| {
                    StdError::generic_err(format!("Serializing QueryRequest: {}", serialize_err))
                })
                .unwrap();
            let res = deps.querier.raw_query(&raw);
            result[i] = match process_query_result(res) {
                Ok(res) => CallResult {
                    success: true,
                    data: res,
                },
                Err(err) => return Err(err.std_at_index(i)),
            }
        }
        let value: Binary;
        match to_binary(&result) {
            Ok(data) => value = data,
            Err(err) => return Err(StdError::generic_err(format!("To binary error: {}", err))),
        }
        let response = IbcQueryRequestTwapResponse::Result(value);
        let acknowledgement = to_binary(&response)?;
        Ok(IbcReceiveResponse::<Empty>::new()
            .set_ack(acknowledgement)
            .add_attribute("action", "receive_ibc_query"))
    })()
    .or_else(|e| {
        // we try to capture all app-level errors and convert them into
        // acknowledgement packets that contain an error code.
        let acknowledgement = encode_ibc_error(format!("invalid packet: {}", e));
        Ok(IbcReceiveResponse::new()
            .set_ack(acknowledgement)
            .add_event(Event::new("ibc").add_attribute("packet", "receive")))
    })
}

fn process_query_result(result: QuerierResult) -> QueryResult {
    match result {
        SystemResult::Err(system_err) => Err(QueryError::System(system_err.to_string())),
        SystemResult::Ok(ContractResult::Err(contract_err)) => {
            Err(QueryError::Contract(contract_err))
        }
        SystemResult::Ok(ContractResult::Ok(value)) => Ok(value),
    }
}

// this encode an error or error message into a proper acknowledgement to the recevier
fn encode_ibc_error(msg: impl Into<String>) -> Binary {
    // this cannot error, unwrap to keep the interface simple
    to_binary(&IbcQueryRequestTwapResponse::Error(msg.into())).unwrap()
}

#[entry_point]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse, ContractError> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_ack"))
}

#[entry_point]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    Ok(IbcBasicResponse::new().add_attribute("action", "ibc_packet_timeout"))
}
