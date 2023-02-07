use thiserror::Error;

use cosmwasm_std::{StdError, Binary};
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

#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Querier system error: {0}")]
    System(String),

    #[error("Querier contract error: {0}")]
    Contract(String),
}

pub type QueryResult = core::result::Result<Binary, QueryError>;

impl QueryError {
    pub fn std_at_index(self, i: usize) -> StdError {
        StdError::generic_err(format!("Error at index {}, {}", i, self))
    }

    pub fn std(self) -> StdError {
        StdError::generic_err(self)
    }
}

impl From<QueryError> for String {
    fn from(q: QueryError) -> Self {
        q.to_string()
    }
}

impl From<QueryError> for StdError {
    fn from(source: QueryError) -> Self {
        source.std()
    }
}