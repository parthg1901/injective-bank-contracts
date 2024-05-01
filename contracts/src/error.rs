use cosmwasm_std::{Timestamp, Uint128, StdError, Addr};
use cw721_base::ContractError as Cw721ContractError;
use thiserror::Error;
use cw_utils::PaymentError;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error(transparent)]
    Std(#[from] StdError),

    #[error(transparent)]
    Payment(#[from] PaymentError),

    #[error(transparent)]
    Cw721(#[from] Cw721ContractError),

    #[error("Not authorized")]
    NotOwner {},

    #[error("Only the borrower can repay {borrower}")]
    NotBorrower {
        borrower: Addr
    },

    #[error("Only the lender can claim")]
    NotLender {},

    #[error("Lender cannot borrow")]
    IsLender {},

    #[error("No offer found")]
    NoOffer {},

    #[error("Collection {collection_id} doesn't exist")]
    InvalidCollectionId {
        collection_id: Uint128
    },

    #[error("The loan is not taken")]
    NotTaken {},

    #[error("The loan is not open")]
    NotOpen {},

    #[error("You do not have enough tokens")]
    NotEnoughTokens {},

    #[error("INJ Amount must be greater than {amount}")]
    NotEnoughINJ {
        amount: Uint128
    },

    #[error("The offer is not paid")]
    NotPaid {},

    #[error("The loan has already expired at {active_till}")]
    LoanExpired {
        active_till: Timestamp,
    },

    #[error("The loan will expire at {active_till}")]
    LoanActive {
        active_till: Timestamp,
    },

    #[error("Active till missing")]
    ActiveTillMissing {}
}