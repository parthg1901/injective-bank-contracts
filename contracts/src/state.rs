use cosmwasm_std::{Addr, Uint128, Timestamp};
use cw_storage_plus::{Map, Item};
use cosmwasm_schema::cw_serde;

#[cw_serde]
pub enum Status {
    Open,
    Taken,
    Paid,
    Failed,
    Done,
    Cancelled
}

#[cw_serde]
pub struct Collection {
    pub addr: Addr,
    pub apy: Uint128,
}

#[cw_serde]
pub struct Offer {
    pub offer_id: Uint128,
    pub amount: Uint128,
    pub active_till: Timestamp,
    pub interest: Uint128,
    pub lender: Addr,
    pub borrower: Option<Addr>,
    pub token_id: Option<Uint128>,
    pub status: Status,
    pub duration: Uint128,
}

#[cw_serde]
pub struct CollectionData {
    pub collection_id: Uint128,
    pub total_offers: Uint128,
    pub offers_taken: Uint128,
    pub best_offer: Option<Offer>,
    pub total_pool: Uint128
}

#[cw_serde]
pub struct OfferData {
    pub lent: Vec<Offer>,
    pub borrowed: Vec<Offer>
}

pub const OWNER_KEY: &str = "owner";
pub const OWNER: Item<Addr> = Item::new(OWNER_KEY);

pub const COLLECTIONS_KEY: &str = "collections";
pub const COLLECTIONS: Item<Vec<Collection>> = Item::new(COLLECTIONS_KEY);

pub const OFFERS_KEY: &str = "offers";
pub const OFFERS: Map<Addr, Vec<Offer>> = Map::new(OFFERS_KEY);

pub const FOUNDERS_KEY: &str = "founders";
pub const FOUNDERS: Item<Vec<Addr>> = Item::new(FOUNDERS_KEY);
