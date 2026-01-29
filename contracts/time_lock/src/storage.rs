use crate::oracle::OracleConfig;
use crate::slippage::SlippageConfig;
use crate::types::{Gift, PriceCache};
use soroban_sdk::{contracttype, Address, BytesN, Env};

const DAY_IN_LEDGERS: u32 = 17280;
const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = DAY_IN_LEDGERS;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    OracleAuthKey,
    OracleConfig,
    SlippageConfig,
    NextGiftId,
    Gift(u64),
    PriceCache,
}

pub fn extend_instance_ttl(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
    extend_instance_ttl(env);
}

pub fn has_admin(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn get_oracle_auth_key(env: &Env) -> BytesN<32> {
    env.storage()
        .instance()
        .get(&DataKey::OracleAuthKey)
        .expect("Contract not initialized")
}

pub fn set_oracle_auth_key(env: &Env, key: &BytesN<32>) {
    env.storage().instance().set(&DataKey::OracleAuthKey, key);
    extend_instance_ttl(env);
}

pub fn get_oracle_config(env: &Env) -> Option<OracleConfig> {
    env.storage().instance().get(&DataKey::OracleConfig)
}

pub fn set_oracle_config(env: &Env, config: &OracleConfig) {
    env.storage().instance().set(&DataKey::OracleConfig, config);
    extend_instance_ttl(env);
}

pub fn get_slippage_config(env: &Env) -> Option<SlippageConfig> {
    env.storage().instance().get(&DataKey::SlippageConfig)
}

pub fn set_slippage_config(env: &Env, config: &SlippageConfig) {
    env.storage()
        .instance()
        .set(&DataKey::SlippageConfig, config);
    extend_instance_ttl(env);
}

pub fn get_next_gift_id(env: &Env) -> u64 {
    env.storage().instance().get(&DataKey::NextGiftId).unwrap_or(1)
}

pub fn increment_next_gift_id(env: &Env) -> u64 {
    let id = get_next_gift_id(env);
    env.storage().instance().set(&DataKey::NextGiftId, &(id + 1));
    extend_instance_ttl(env);
    id
}

pub fn get_gift(env: &Env, id: u64) -> Option<Gift> {
    env.storage().instance().get(&DataKey::Gift(id))
}

pub fn set_gift(env: &Env, id: u64, gift: &Gift) {
    env.storage().instance().set(&DataKey::Gift(id), gift);
    extend_instance_ttl(env);
}


pub fn get_price_cache(env: &Env) -> Option<PriceCache> {
    env.storage().instance().get(&DataKey::PriceCache)
}

pub fn set_price_cache(env: &Env, cache: &PriceCache) {
    env.storage().instance().set(&DataKey::PriceCache, cache);
    extend_instance_ttl(env);
}
