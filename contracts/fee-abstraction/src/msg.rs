use prost::bytes::Bytes;

use cosmwasm_std::{Empty, QueryRequest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Just needs to know the code_id of a reflect contract to spawn sub-accounts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub packet_lifetime: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    IbcQuery {
        channel_id: String,
        // Queries to be executed
        msgs: Vec<QueryRequest<Empty>>,
        // Callback contract address that implements ReceiveIbcResponseMsg
        callback: String,
    },
    OsmoTwapIbcQuery {
        channel_id: String,
        // Callback contract address that implements ReceiveIbcResponseMsg
        callback: String,
        pool_id: u64,
        token_in_denom: String,
        token_out_denom: String,
        with_swap_fee: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    QueryStargateTwap {
        pool_id: u64,
        token_in_denom: String,
        token_out_denom: String,
        with_swap_fee: bool,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct IbcQueryRequestTwap {
    pool_id: u64,
    base_asset_denom: String,
    quote_asset_denom: String,
}

impl From<&[u8]> for IbcQueryRequestTwap {
    fn from(item: &[u8]) -> Self {
        let (head, body, _tail) = unsafe { item.align_to::<IbcQueryRequestTwap>() };
        assert!(head.is_empty(), "Data was not aligned");
        let my_struct = &body[0];
        return my_struct.clone()
    }
}
