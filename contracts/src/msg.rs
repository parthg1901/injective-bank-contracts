use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use crate::state::{CollectionData, Collection, OfferData};

#[cw_serde]
pub struct InstantiateMsg {
    pub founder_one: Addr,
    pub founder_two: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    AddCollection {
        collection_address: Addr,
        apy: Uint128,
    },

    Lend {
        collection_id: Uint128, 
        duration: Uint128, 
    },

    Borrow {
        collection_id: Uint128, 
        token_id: Uint128,
    },

    Repay {
        collection_id: Uint128, 
        offer_id: Uint128, 
    },

    Claim {
        collection_id: Uint128, 
        offer_id: Uint128, 
    },

    Withdraw {
        collection_id: Uint128, 
        offer_id: Uint128, 
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Vec<Collection>)]
    CollectionsList {},

    #[returns(Uint128)]
    GetBestOffer {
        collection_id: Uint128,
        from: Addr
    },

    #[returns(OfferData)]
    GetOffers {
        collection_id: Uint128,
        from: Addr
    },

    #[returns(CollectionData)]
    GetCollectionData {
        collection_id: Uint128,
        from: Addr
    }
}

