#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Bytes, Env,
    Symbol, Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub index: u32,
    pub timestamp: u64,
    pub event_type: Symbol,
    pub submitter: Address,
    pub metadata: Bytes,
}

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Owner,
    GlobalMaxLogs,
    TotalEvents,
    EventCapSet(Symbol),
    EventMaxLogs(Symbol),
    EventTypeIndices(Symbol),
    EventData(u32),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ContractError {
    CallerNotOwner = 1,
    GlobalMaxLogsReached = 2,
    EventTypeMaxLogsReached = 3,
    EventDoesNotExist = 4,
    EventTypeIndexOutOfBounds = 5,
    NewOwnerIsZero = 6,
    CapNotSet = 7,
}

const NULL_ACCOUNT: &str = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

#[contract]
pub struct AuditLedger;

#[contractimpl]
impl AuditLedger {
    pub fn initialize(env: Env, owner: Address, global_max_logs: u32) {
        owner.require_auth();
        env.storage().instance().set(&DataKey::Owner, &owner);
        env.storage()
            .instance()
            .set(&DataKey::GlobalMaxLogs, &global_max_logs);
        env.storage().instance().set(&DataKey::TotalEvents, &0u32);
    }

    pub fn log_event(env: Env, submitter: Address, event_type: Symbol, metadata: Bytes) -> u32 {
        submitter.require_auth();

        let global_max: u32 = env
            .storage()
            .instance()
            .get(&DataKey::GlobalMaxLogs)
            .unwrap();
        let total: u32 = env.storage().instance().get(&DataKey::TotalEvents).unwrap();

        if total >= global_max {
            panic_with_error!(&env, ContractError::GlobalMaxLogsReached);
        }

        if env
            .storage()
            .instance()
            .has(&DataKey::EventCapSet(event_type.clone()))
        {
            let cap: u32 = env
                .storage()
                .instance()
                .get(&DataKey::EventMaxLogs(event_type.clone()))
                .unwrap();
            let count = Self::event_type_count(&env, event_type.clone());
            if count >= cap {
                panic_with_error!(&env, ContractError::EventTypeMaxLogsReached);
            }
        }

        let index = total;
        let timestamp = env.ledger().timestamp();
        let evt = Event {
            index,
            timestamp,
            event_type: event_type.clone(),
            submitter: submitter.clone(),
            metadata: metadata.clone(),
        };

        env.storage()
            .instance()
            .set(&DataKey::EventData(index), &evt);

        let mut indices: Vec<u32> = env
            .storage()
            .instance()
            .get(&DataKey::EventTypeIndices(event_type.clone()))
            .unwrap_or(Vec::new(&env));
        indices.push_back(index);
        env.storage()
            .instance()
            .set(&DataKey::EventTypeIndices(event_type.clone()), &indices);

        env.storage()
            .instance()
            .set(&DataKey::TotalEvents, &(total + 1));

        env.events().publish(
            (Symbol::new(&env, "log_event"), event_type, submitter),
            (index, timestamp, metadata),
        );

        index
    }

    pub fn total_events(env: Env) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::TotalEvents)
            .unwrap_or(0)
    }

    pub fn get_event(env: Env, index: u32) -> Event {
        env.storage()
            .instance()
            .get(&DataKey::EventData(index))
            .unwrap_or_else(|| {
                panic_with_error!(&env, ContractError::EventDoesNotExist);
            })
    }

    pub fn event_count(env: Env, event_type: Symbol) -> u32 {
        Self::event_type_count(&env, event_type)
    }

    pub fn get_event_by_type(env: Env, event_type: Symbol, type_index: u32) -> Event {
        let indices: Vec<u32> = env
            .storage()
            .instance()
            .get(&DataKey::EventTypeIndices(event_type.clone()))
            .unwrap_or_else(|| {
                panic_with_error!(&env, ContractError::EventTypeIndexOutOfBounds);
            });

        let global_index = indices.get(type_index).unwrap_or_else(|| {
            panic_with_error!(&env, ContractError::EventTypeIndexOutOfBounds);
        });

        env.storage()
            .instance()
            .get(&DataKey::EventData(global_index))
            .unwrap_or_else(|| {
                panic_with_error!(&env, ContractError::EventDoesNotExist);
            })
    }

    pub fn set_global_max_logs(env: Env, caller: Address, new_max: u32) {
        caller.require_auth();
        Self::require_owner(&env, &caller);
        env.storage()
            .instance()
            .set(&DataKey::GlobalMaxLogs, &new_max);
    }

    pub fn set_event_max_logs(env: Env, caller: Address, event_type: Symbol, new_max: u32) {
        caller.require_auth();
        Self::require_owner(&env, &caller);
        env.storage()
            .instance()
            .set(&DataKey::EventCapSet(event_type.clone()), &true);
        env.storage()
            .instance()
            .set(&DataKey::EventMaxLogs(event_type), &new_max);
    }

    pub fn remove_event_cap(env: Env, caller: Address, event_type: Symbol) {
        caller.require_auth();
        Self::require_owner(&env, &caller);
        if !env
            .storage()
            .instance()
            .has(&DataKey::EventCapSet(event_type.clone()))
        {
            panic_with_error!(&env, ContractError::CapNotSet);
        }
        env.storage()
            .instance()
            .remove(&DataKey::EventCapSet(event_type.clone()));
        env.storage()
            .instance()
            .remove(&DataKey::EventMaxLogs(event_type));
    }

    pub fn transfer_ownership(env: Env, caller: Address, new_owner: Address) {
        caller.require_auth();
        Self::require_owner(&env, &caller);
        if new_owner == Address::from_str(&env, NULL_ACCOUNT) {
            panic_with_error!(&env, ContractError::NewOwnerIsZero);
        }
        env.storage().instance().set(&DataKey::Owner, &new_owner);
    }

    fn require_owner(env: &Env, addr: &Address) {
        let owner: Address = env.storage().instance().get(&DataKey::Owner).unwrap();
        if addr != &owner {
            panic_with_error!(env, ContractError::CallerNotOwner);
        }
    }

    fn event_type_count(env: &Env, event_type: Symbol) -> u32 {
        env.storage()
            .instance()
            .get(&DataKey::EventTypeIndices(event_type))
            .map(|v: Vec<u32>| v.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod test;
