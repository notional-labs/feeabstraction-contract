use std::time::SystemTime;

use cosmwasm_std::{Binary, ContractResult, Empty, QueryRequest, Timestamp};
use prost::Message;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Just needs to know the code_id of a reflect contract to spawn sub-accounts
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub packet_lifetime: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {}

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
    pub base_denom: String,
    pub pool_id: String,
    pub quote_denom: String,
    pub twap_period: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, ::prost::Message)]
#[serde(rename_all = "snake_case")]
pub struct QueryTwapRequest {
    #[prost(uint64, tag = "1")]
    pub pool_id: u64,
    #[prost(string, tag = "2")]
    pub base_asset: String,
    #[prost(string, tag = "3")]
    pub quote_asset: String,
    #[prost(Timestamp, tag = "4")]
    pub start_time: Timestamp
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

/// All acknowledgements are wrapped in `ContractResult`.
/// The success value depends on the PacketMsg variant.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct Result {
    pub value: Binary,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IbcQueryRequestTwapResponse {
    Result(Binary),
    Error(String),
}
