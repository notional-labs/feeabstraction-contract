use thiserror::Error;

use cosmwasm_std::StdError;
use cw_utils::{ParseReplyError, PaymentError};

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    ParseReply(#[from] ParseReplyError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("Cannot register over an existing channel")]
    ChannelAlreadyRegistered,

    #[error("Invalid reply id")]
    InvalidReplyId,
}
