use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Status, Offer, OFFERS, OWNER, COLLECTIONS, FOUNDERS, CollectionData, Collection, OfferData};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response,
    Uint128, Timestamp, BankMsg, coins, WasmMsg
};
use rust_decimal::Decimal;
use num_traits::{pow::Pow, ToPrimitive};
use std::str::FromStr;
use cw2::set_contract_version;
use cw721_base::{
    msg::ExecuteMsg as Cw721ExecuteMsg,
};
use cw_utils::must_pay;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:injective-bank";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    OWNER.save(deps.storage, &info.sender.clone())?;
    COLLECTIONS.save(deps.storage, &Vec::new().clone())?;
    let mut founders: Vec<Addr> = Vec::new();
    founders.push(msg.founder_one);
    founders.push(msg.founder_two);
    FOUNDERS.save(deps.storage, &founders.clone());

    Ok(Response::new()
        .add_attribute("instantiated", "true"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    use ExecuteMsg::*;
    match msg {
        AddCollection {
            collection_address,
            apy
        } => {
            Ok(add_collection(deps, env, info, collection_address, apy)?)
        },
        Lend {
            collection_id,
            duration,
        } => {
            Ok(lend(deps, env, info, collection_id, duration)?)
        },
        Borrow {
            collection_id,
            token_id,
        } => {
            Ok(borrow(deps, env, info, collection_id, token_id)?)
        },
        Repay {
            collection_id,
            offer_id,
        } => {
            Ok(repay(deps, env, info, collection_id, offer_id)?)
        },
        Claim {
            collection_id,
            offer_id,
        } => {
            Ok(claim(deps, env, info, collection_id, offer_id)?)
        },
        Withdraw {
            collection_id,
            offer_id
        } => {
            Ok(withdraw(deps, env, info, collection_id, offer_id)?)
        },
    }
}

pub fn add_collection(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    collection_address: Addr,
    apy: Uint128,
) -> Result<Response, ContractError> {
    let owner : Addr= OWNER.load(deps.storage)?;
    if owner != info.sender {
        return Err(ContractError::NotOwner {});
    }

    let mut collections = COLLECTIONS.load(deps.storage)?;
    let data = Collection {
        addr: collection_address.clone(),
        apy: apy.clone()
    };
    collections.push(data.clone());

    COLLECTIONS.save(deps.storage, &collections)?;
    OFFERS.save(deps.storage, collection_address.clone(), &Vec::new())?;
    Ok(Response::default())
}

pub fn lend(
    deps: DepsMut, 
    _env: Env, 
    info: MessageInfo, 
    collection_id: Uint128, 
    duration: Uint128, 
) -> Result<Response, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;

    if collections.get(collection_id.u128() as usize).is_none() {
        return Err(ContractError::InvalidCollectionId {collection_id});
    }

    let collection = &collections[collection_id.u128() as usize];

    let value = must_pay(&info, "inj")?.u128();
    let mut offers = OFFERS.load(deps.storage, collection.addr.clone())?;
    let founders = FOUNDERS.load(deps.storage)?;
    let base_amount = (value) as u128;
    let percentage: u128 = 5;
    let divisor: u128 = 1000;

    // Perform the multiplication and division
    let commission = (base_amount) * percentage / divisor;
    let transfer_msg_one = BankMsg::Send {
        to_address: founders[0].clone().into(),
        amount: coins(commission.into(), "inj"),
    };
    let transfer_msg_two = BankMsg::Send {
        to_address: founders[1].clone().into(),
        amount: coins(commission.into(), "inj"),
    };
    let new_offer = Offer {
        offer_id: Uint128::from(offers.len() as u128),
        amount: Uint128::from(value - ((2*commission) as u128)),
        active_till: Timestamp::from_nanos(0),
        lender: info.sender.clone(),
        status: Status::Open,
        borrower: None,
        interest: Uint128::from(get_interest(collection.apy.clone().u128(), duration.clone().u128(), value)),
        token_id: None,
        duration: Uint128::from(duration),
    };
    offers.push(new_offer.clone());
    OFFERS.save(deps.storage, collection.addr.clone(), &offers)?;

    Ok(Response::new().add_message(transfer_msg_one).add_message(transfer_msg_two))
}

fn get_interest(apy: u128, duration: u128, amount: u128) -> u128 {
    let apy_decimal = u128_to_decimal(apy);
    let duration_decimal = u128_to_decimal(duration);
    let amount_decimal = u128_to_decimal(amount);
    let t_int = ((apy_decimal / Decimal::from_str("100").unwrap() + Decimal::from(1))
        .pow(duration_decimal / Decimal::from_str("31536000").unwrap()) - Decimal::from(1))*(Decimal::from(31536000) / duration_decimal);

    let an_int = t_int * amount_decimal;

    (an_int * (duration_decimal / Decimal::from(31536000))).to_u128().unwrap()
}

fn u128_to_decimal(value: u128) -> Decimal {
    Decimal::from_str(&value.to_string()).unwrap()
}

pub fn borrow(
    deps: DepsMut, 
    env: Env, 
    info: MessageInfo, 
    collection_id: Uint128, 
    token_id: Uint128, 
) -> Result<Response, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;

    if collections.get(collection_id.u128() as usize).is_none() {
        return Err(ContractError::InvalidCollectionId {collection_id});
    }

    let collection = &collections[collection_id.u128() as usize];
    let mut offers = OFFERS.load(deps.storage, collection.addr.clone())?;


    let best_offer: Uint128 = get_best_offer(deps.as_ref(), collection_id, info.sender.clone())?;
    let curr_offer = &offers[(best_offer).u128() as usize];

    if curr_offer.lender.clone() == info.sender.clone() {
        return Err(ContractError::IsLender {});
    }

    let transfer_msg = BankMsg::Send {
        to_address: info.sender.clone().into(),
        amount: coins((curr_offer.amount).into(), "inj"),
    };

    let transfer: Cw721ExecuteMsg::<Empty, Empty> = Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
        recipient: env.contract.address.to_string(),
        token_id: String::from(token_id),
    };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: collection.addr.to_string(),
        msg: to_json_binary(&transfer)?,
        funds: Vec::new(),
    };

    let updated_offer = Offer {
        offer_id: curr_offer.offer_id,
        amount: curr_offer.amount,
        active_till: Timestamp::from_nanos(env.block.time.nanos() + ((curr_offer.duration.u128() as u64) * 1_000_000_000)),
        lender: curr_offer.lender.clone(),
        interest: curr_offer.interest,
        status: Status::Taken,
        borrower: Some(info.sender.clone()),
        token_id: Some(token_id),
        duration: curr_offer.duration,
    };

    offers[(best_offer).u128() as usize] = updated_offer;
    OFFERS.save(deps.storage, collection.addr.clone(), &offers)?;
    
    Ok(Response::new().add_message(transfer_msg).add_message(wasm_msg))

}

pub fn repay(
    deps: DepsMut, 
    env: Env, 
    info: MessageInfo, 
    collection_id: Uint128, 
    offer_id: Uint128, 
) -> Result<Response, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;

    if collections.get(collection_id.u128() as usize).is_none() {
        return Err(ContractError::InvalidCollectionId {collection_id});
    }

    let collection = &collections[collection_id.u128() as usize];
    let mut offers = OFFERS.load(deps.storage, collection.addr.clone())?;


    let curr_offer = &offers[offer_id.u128() as usize];
    let borrower = match curr_offer.borrower.clone() {
        Some(addr) => addr.clone(),
        None => Addr::unchecked("none"),
    };

    if borrower != info.sender.clone() {
        return Err(ContractError::NotBorrower { borrower: borrower });
    }
    let value = must_pay(&info, "inj")?.u128();

    if Uint128::from(value) < (curr_offer.amount + curr_offer.interest) {
        return Err(ContractError::NotEnoughINJ {
            amount: curr_offer.amount + curr_offer.interest
        });
    }

    if curr_offer.status != Status::Taken {
        return Err(ContractError::NotTaken {});
    }

    if env.block.time >= curr_offer.active_till {
        return Err(ContractError::LoanExpired {
            active_till: curr_offer.active_till
        });
    }

    let transfer_msg = BankMsg::Send {
        to_address: curr_offer.lender.clone().into(),
        amount: coins((curr_offer.amount + curr_offer.interest).into(), "inj"),
    };

    let recipient_string = match curr_offer.borrower.clone() {
        Some(addr) => addr.clone().to_string(),
        None => "None".to_string(),
    };
    let token_string = match curr_offer.token_id {
        Some(t) => t.to_string(),
        None => "None".to_string(),
    };
    let transfer: Cw721ExecuteMsg::<Empty, Empty> = Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
        recipient: recipient_string,
        token_id: token_string,
    };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: collection.addr.to_string(),
        msg: to_json_binary(&transfer)?,
        funds: Vec::new(),
    };

    let updated_offer = Offer {
        token_id: curr_offer.token_id,
        offer_id: curr_offer.offer_id,
        amount: curr_offer.amount,
        active_till: curr_offer.active_till,
        lender: curr_offer.lender.clone(),
        interest: curr_offer.interest,
        status: Status::Paid,
        borrower: curr_offer.borrower.clone(),
        duration: curr_offer.duration,
    };

    offers[offer_id.u128() as usize] = updated_offer;
    OFFERS.save(deps.storage, collection.addr.clone(), &offers)?;
    
    Ok(Response::new().add_message(transfer_msg).add_message(wasm_msg))
}

pub fn claim(
    deps: DepsMut, 
    env: Env, 
    info: MessageInfo, 
    collection_id: Uint128, 
    offer_id: Uint128, 
) -> Result<Response, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;

    if collections.get(collection_id.u128() as usize).is_none() {
        return Err(ContractError::InvalidCollectionId {collection_id});
    }

    let collection = &collections[collection_id.u128() as usize];
    let mut offers = OFFERS.load(deps.storage, collection.addr.clone())?;

    let curr_offer = &offers[offer_id.u128() as usize];

    if curr_offer.lender != info.sender {
        return Err(ContractError::NotLender {});
    }

    if curr_offer.status != Status::Taken {
        return Err(ContractError::NotPaid {});
    }

    if env.block.time < curr_offer.active_till {
        return Err(ContractError::LoanActive {
            active_till: curr_offer.active_till
        });
    }

    let recipient_string = curr_offer.lender.to_string();
    let token_string = match curr_offer.token_id {
        Some(t) => t.to_string(),
        None => "None".to_string(),
    };
    let transfer: Cw721ExecuteMsg::<Empty, Empty> = Cw721ExecuteMsg::<Empty, Empty>::TransferNft {
        recipient: recipient_string,
        token_id: token_string,
    };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: collection.addr.to_string(),
        msg: to_json_binary(&transfer)?,
        funds: Vec::new(),
    };

    let updated_offer = Offer {
        token_id: curr_offer.token_id,
        offer_id: curr_offer.offer_id,
        amount: curr_offer.amount,
        active_till: curr_offer.active_till,
        lender: curr_offer.lender.clone(),
        interest: curr_offer.interest,
        status: Status::Failed,
        borrower: curr_offer.borrower.clone(),
        duration: curr_offer.duration,
    };

    offers[offer_id.u128() as usize] = updated_offer;
    OFFERS.save(deps.storage, collection.addr.clone(), &offers)?;
    
    Ok(Response::new().add_message(wasm_msg))
}

pub fn withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    collection_id: Uint128, 
    offer_id: Uint128, 
) -> Result<Response, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;

    if collections.get(collection_id.u128() as usize).is_none() {
        return Err(ContractError::InvalidCollectionId {collection_id});
    }

    let collection = &collections[collection_id.u128() as usize];
    let mut offers = OFFERS.load(deps.storage, collection.addr.clone())?;


    // let best_offer: Uint128 = get_best_offer(deps, collection_id)?;
    let curr_offer = &offers[offer_id.u128() as usize];

    if curr_offer.lender != info.sender {
        return Err(ContractError::NotLender {});
    }

    if curr_offer.status != Status::Open {
        return Err(ContractError::NotOpen {});
    }

    let transfer_msg = BankMsg::Send {
        to_address: curr_offer.lender.clone().into(),
        amount: coins(curr_offer.amount.into(), "inj"),
    };

    let updated_offer = Offer {
        token_id: curr_offer.token_id,
        offer_id: curr_offer.offer_id,
        amount: curr_offer.amount,
        active_till: curr_offer.active_till,
        lender: curr_offer.lender.clone(),
        interest: curr_offer.interest,
        status: Status::Cancelled,
        borrower: curr_offer.borrower.clone(),
        duration: curr_offer.duration,
    };

    offers[offer_id.u128() as usize] = updated_offer;
    OFFERS.save(deps.storage, collection.addr.clone(), &offers)?;
    
    Ok(Response::new().add_message(transfer_msg))

}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    match msg {
        QueryMsg::GetBestOffer { collection_id, from } => Ok(to_json_binary(&get_best_offer(deps, collection_id, from)?)?),
        QueryMsg::CollectionsList {} => Ok(to_json_binary(&get_collections_list(deps)?)?),
        QueryMsg::GetOffers { collection_id, from } => Ok(to_json_binary(&get_offers(deps, collection_id, from)?)?),
        QueryMsg::GetCollectionData { collection_id, from } => Ok(to_json_binary(&get_collections_data(deps, collection_id, from)?)?),
    }
}

fn get_best_offer(deps: Deps, collection_id: Uint128, from: Addr) -> Result<Uint128, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;
    let offers = OFFERS.load(deps.storage, collections[collection_id.u128() as usize].addr.clone())?;
    let mut best_int: i32 = -1;
    let mut best_offer: Option<Offer> = None;
    let mut done = false;
    for i in 0..offers.len() {
        if offers[i].status == Status::Open {
            if offers[i].lender.clone() != from.clone() {
                if !done {
                    best_int = i as i32;
                    best_offer = Some(offers[i].clone());
                    done = true;
                }
                if offers[i].amount > best_offer.clone().unwrap().amount {
                    best_offer = Some(offers[i].clone());
                    best_int = i as i32;
                } else if (offers[i].amount == best_offer.clone().unwrap().amount) && (offers[i].interest < best_offer.clone().unwrap().interest) {
                    best_offer = Some(offers[i].clone());
                    best_int = i as i32;
                } else if (offers[i].amount == best_offer.clone().unwrap().amount) && (offers[i].interest == best_offer.clone().unwrap().interest) && (offers[i].duration == best_offer.clone().unwrap().duration) {
                    best_offer = Some(offers[i].clone());
                    best_int = i as i32;
                }
            }
        }
    }
    if best_int < 0 {
        return Err(ContractError::NoOffer {});
    }
    Ok(Uint128::from(best_int as u64))
}

fn get_collections_list(deps: Deps) -> Result<Vec<Collection>, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;
    Ok(collections)
}

fn get_offers(deps: Deps, collection_id: Uint128, from: Addr) -> Result<OfferData, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;
    let offers = OFFERS.load(deps.storage, collections[collection_id.u128() as usize].addr.clone())?;
    let borrowed: Vec<Offer> = offers
        .iter()
        .filter(|&x| x.borrower.clone() == Some(from.clone()))
        .cloned()
        .collect();
    let lent: Vec<Offer> = offers
        .iter()
        .filter(|&x| x.lender.clone() == from.clone())
        .cloned()
        .collect();
    let offer_data = OfferData {
        lent: lent,
        borrowed: borrowed
    };
    Ok(offer_data)
}

fn get_collections_data(deps: Deps, collection_id: Uint128, from: Addr) -> Result<CollectionData, ContractError> {
    let collections = COLLECTIONS.load(deps.storage)?;
    let offers = OFFERS.load(deps.storage, collections[collection_id.u128() as usize].addr.clone())?;
    let best_offer: Option<Offer> = match get_best_offer(deps, collection_id, from.clone()) {
        Ok(best_offer) => Some(offers[best_offer.u128() as usize].clone()),
        Err(_) => None,
    };
    let mut taken_offers = 0;
    let mut total_pool: u128 = 0;
    for i in 0..offers.len() {
        if offers[i].status != Status::Open {
            taken_offers+=1;
        } else {
            total_pool+=offers[i].amount.u128();
        }
    }
    let collection_data = CollectionData {
        collection_id: collection_id.clone(),
        total_offers: Uint128::from(offers.len() as u64),
        offers_taken: Uint128::from(taken_offers as u64),
        best_offer: best_offer,
        total_pool: Uint128::from(total_pool as u128)
    };
    Ok(collection_data)
}


#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{from_json};

    use super::*;

    #[test]
    fn test () {
        let mut deps = mock_dependencies();

        // instantiate an empty contract
        let instantiate_msg = InstantiateMsg {
            founder_one: Addr::unchecked(&String::from("foun1")),
            founder_two: Addr::unchecked(&String::from("foun1"))
        };
        let info = mock_info(&String::from("anyone"), &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        let add = ExecuteMsg::AddCollection {
            collection_address: Addr::unchecked(&String::from("coll_one")),
            apy: Uint128::from(90 as u128)
        };

        let info = mock_info(&String::from("anyone"), &[]);

        let res = execute(deps.as_mut(), mock_env(), info, add).unwrap();
        assert_eq!(0, res.messages.len());

        let query_msg = QueryMsg::CollectionsList {};
        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let collections: Vec<Collection> = from_json(res).unwrap();
        println!("{:?}", collections);

        // assert_eq!(Addr::unchecked(&String::from("coll_one")), collections[0]);

        let add = ExecuteMsg::Lend {
            collection_id: Uint128::from(0 as u64), 
            duration: Uint128::from(86400 as u64), 
        };

        let info = mock_info(&String::from("caller"), &coins(
            10000000000000000000, "inj"));

        let res = execute(deps.as_mut(), mock_env(), info, add).unwrap();
        println!("{:?}", res.messages);
        let base_amount = (10000000000000) as u64;
        let percentage: u64 = 25;
        let divisor: u64 = 1000;

        // Perform the multiplication and division
        let commission = (base_amount as u64) * percentage / divisor;

        println!("{}", Uint128::from(commission));
        assert_eq!(2, res.messages.len());

        let add = ExecuteMsg::Lend {
            collection_id: Uint128::from(0 as u64), 
            duration: Uint128::from(86400 as u64), 
        };

        let info = mock_info(&String::from("caller"), &coins(10000, "inj"));

        let res = execute(deps.as_mut(), mock_env(), info, add).unwrap();
        assert_eq!(2, res.messages.len());

        let query_msg = QueryMsg::GetBestOffer { collection_id: Uint128::from(0 as u64), from: Addr::unchecked(&String::from("borower")) };
        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let best: Uint128 = from_json(res).unwrap();
        assert_eq!(Uint128::from(0 as u64), best);

        let query_msg = QueryMsg::GetCollectionData { collection_id: Uint128::from(0 as u64), from: Addr::unchecked(&String::from("borrower")) };
        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let best: CollectionData = from_json(res).unwrap();
        // assert_eq!(best, CollectionData {
        //     collection_id: Uint128::from(0 as u64),
        //     total_offers: Uint128::from(2 as u64),
        //     offers_taken: Uint128::from(0 as u64),
        //     best_offer: None,
        //     total_pool: Uint128::from(10100 as u128)
        // });

        let borrow = ExecuteMsg::Borrow {
            collection_id: Uint128::from(0 as u64), 
            token_id: Uint128::from(0 as u64),
        };

        let info = mock_info(&String::from("borrower"), &[]);

        let res = execute(deps.as_mut(), mock_env(), info, borrow).unwrap();
        assert_eq!(2, res.messages.len());

        // let query_msg = QueryMsg::GetCollectionData { collection_id: Uint128::from(0 as u64), from: Addr::unchecked(&String::from("borower")) };
        // let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        // let best: CollectionData = from_json(res).unwrap();
        // assert_eq!(best, CollectionData {
        //     collection_id: Uint128::from(1 as u64),
        //     total_offers: Uint128::from(1 as u64),
        //     offers_taken: Uint128::from(1 as u64),
        //     best_offer: None,
        //     total_pool: Uint128::from(1 as u128)
        // });

        let query_msg = QueryMsg::GetOffers { collection_id: Uint128::from(0 as u64), from:  Addr::unchecked(&String::from("borrower"))};
        let res = query(deps.as_ref(), mock_env(), query_msg).unwrap();
        let offers: OfferData = from_json(res).unwrap();
        let borrower = match offers.borrowed[0].borrower.clone() {
            Some(addr) => addr.clone(),
            None => Addr::unchecked("none"),
        };
        println!{"{:?}", offers};
        assert_eq!(&borrower.to_string(), &String::from("borrower"));

        let borrow = ExecuteMsg::Repay {
            collection_id: Uint128::from(0 as u64), 
            offer_id: Uint128::from(0 as u64),
        };

        let info = mock_info(&String::from("borrower"), &coins(9817600508718498930, "inj"));
        assert_eq!(borrower, info.sender.clone());

        let res = execute(deps.as_mut(), mock_env(), info, borrow).unwrap();
        assert_eq!(2, res.messages.len());


        let withdraw = ExecuteMsg::Withdraw {
            collection_id: Uint128::from(0 as u64), 
            offer_id: Uint128::from(1 as u64),
        };

        let info = mock_info(&String::from("caller"), &[]);

        let res = execute(deps.as_mut(), mock_env(), info, withdraw).unwrap();
        assert_eq!(1, res.messages.len());
    }
}