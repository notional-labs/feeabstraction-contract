use cosmwasm_std::IbcOrder;

pub mod contract;
pub mod error;
pub mod ibc;
pub mod msg;
pub mod state;

pub const IBC_APP_VERSION: &str = "outpost-forwarder-v1";
pub const APP_ORDER: IbcOrder = IbcOrder::Unordered;
// we use this for tests to ensure it is rejected
pub const BAD_APP_ORDER: IbcOrder = IbcOrder::Ordered;
