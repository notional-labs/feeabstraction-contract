use cosmwasm_std::{
    entry_point, from_binary, from_slice, to_binary, to_vec, Binary, ContractResult, Deps, DepsMut,
    Empty, Env, Event, Ibc3ChannelOpenResponse, IbcBasicResponse, IbcChannelCloseMsg,
    IbcChannelConnectMsg, IbcChannelOpenMsg, IbcChannelOpenResponse, IbcPacketAckMsg,
    IbcPacketReceiveMsg, IbcPacketTimeoutMsg, IbcReceiveResponse, QueryRequest, StdError,
    StdResult, SystemResult, WasmMsg,
};
use cw_ibc_query::{
    check_order, check_version, IbcQueryResponse, PacketMsg, ReceiveIbcResponseMsg,
    ReceiverExecuteMsg, StdAck, IBC_APP_VERSION,
};
use cw_osmo_proto::osmosis::gamm::v1beta1::QuerySpotPriceRequest;
use prost::Message;

use crate::error::ContractError;
use crate::msg::{AcknowledgementMsg, IbcQueryRequestTwap, IbcQueryRequestTwapResponse};
use crate::state::PENDING;

#[entry_point]
/// enforces ordering and versioing constraints
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg,
) -> Result<IbcChannelOpenResponse, ContractError> {
    let channel = msg.channel();

    check_order(&channel.order)?;
    // In ibcv3 we don't check the version string passed in the message
    // and only check the counterparty version.
    if let Some(counter_version) = msg.counterparty_version() {
        check_version(counter_version)?;
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
        let ibc_msg: IbcQueryRequestTwap;
        let decoded_data: StdResult<IbcQueryRequestTwap> = from_binary(&msg.packet.data);
        match decoded_data {
            Ok(ibc_query_req) => ibc_msg = ibc_query_req,
            Err(error) => {
                return Err(StdError::generic_err(format!(
                    "Serilize error: {}",
                    error
                )))
            }
        }

        let pool_id: u64;
        match ibc_msg.pool_id.as_str().parse::<u64>() {
            Ok(id) => pool_id = id,
            Err(error) => {
                return Err(StdError::generic_err(format!(
                    "Parse error: {}",
                    error
                )))
            }
        }
        let query_request: QuerySpotPriceRequest = QuerySpotPriceRequest {
            pool_id: pool_id,
            token_in_denom: ibc_msg.base_asset_denom,
            token_out_denom: ibc_msg.quote_asset_denom,
            with_swap_fee: false,
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
            SystemResult::Ok(ContractResult::Err(contract_err)) => Err(StdError::generic_err(
                format!("Querier contract error: {}", contract_err),
            )),
            SystemResult::Ok(ContractResult::Ok(value)) => {
                let response = IbcQueryRequestTwapResponse { value };
                let acknowledgement = to_binary(&AcknowledgementMsg::Ok(response))?;
                Ok(IbcReceiveResponse::<Empty>::new()
                    .set_ack(acknowledgement)
                    .add_attribute("action", "receive_ibc_query"))
            }
        };
        return res;
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

// this encode an error or error message into a proper acknowledgement to the recevier
fn encode_ibc_error(msg: impl Into<String>) -> Binary {
    // this cannot error, unwrap to keep the interface simple
    to_binary(&AcknowledgementMsg::<()>::Err(msg.into())).unwrap()
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
