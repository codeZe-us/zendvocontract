#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, Env, Address, String, symbol_short};

mod types;
mod errors;
mod constants;
mod oracle;
mod slippage;
mod events;
mod test;

use errors::Error;
use oracle::OracleConfig;
use slippage::SlippageConfig;
use events::*;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PriceCache {
    pub rate: i128,
    pub timestamp: u64,
}

#[contract]
pub struct TimeLockContract;

/// Helper: Get admin from storage
fn get_admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .persistent()
        .get::<_, Address>(&symbol_short!("admin"))
        .ok_or(Error::Unauthorized)
}

/// Helper: Get oracle config from storage
fn get_oracle_config(env: &Env) -> Result<OracleConfig, Error> {
    env.storage()
        .persistent()
        .get::<_, OracleConfig>(&symbol_short!("oracle"))
        .ok_or(Error::OracleUnavailable)
}

/// Helper: Get slippage config from storage
fn get_slippage_config_internal(env: &Env) -> Result<SlippageConfig, Error> {
    env.storage()
        .persistent()
        .get::<_, SlippageConfig>(&symbol_short!("slippage"))
        .ok_or(Error::Unauthorized)
}

/// Helper: Verify admin auth and return admin address
fn require_admin_auth(env: &Env) -> Result<Address, Error> {
    let admin = get_admin(env)?;
    admin.require_auth();
    Ok(admin)
}

#[contractimpl]
impl TimeLockContract {
    /// Initialize contract with admin and oracle address
    pub fn initialize(env: Env, admin: Address, oracle_address: Address) -> Result<(), Error> {
        if env.storage().persistent().has(&symbol_short!("admin")) {
            return Err(Error::Unauthorized);
        }

        env.storage()
            .persistent()
            .set(&symbol_short!("admin"), &admin);

        let oracle_config = oracle::default_oracle_config(oracle_address);
        env.storage()
            .persistent()
            .set(&symbol_short!("oracle"), &oracle_config);

        let slippage_config = slippage::default_slippage_config(admin.clone());
        env.storage()
            .persistent()
            .set(&symbol_short!("slippage"), &slippage_config);

        Ok(())
    }

    /// Get current oracle configuration (public view)
    pub fn get_oracle_status(env: Env) -> Result<OracleConfig, Error> {
        env.storage()
            .persistent()
            .get::<_, OracleConfig>(&symbol_short!("oracle"))
            .ok_or(Error::OracleUnavailable)
    }

    /// Set oracle address (admin only)
    pub fn set_oracle_address(env: Env, new_oracle_address: Address) -> Result<(), Error> {
        let _admin = require_admin_auth(&env)?;

        let mut oracle_config = get_oracle_config(&env)?;
        let old_address = oracle_config.oracle_address.clone();
        oracle_config.oracle_address = new_oracle_address.clone();

        env.storage()
            .persistent()
            .set(&symbol_short!("oracle"), &oracle_config);

        env.events().publish(
            (symbol_short!("oracle_ad"),),
            OracleAddressUpdated {
                old_address,
                new_address: new_oracle_address,
            },
        );

        Ok(())
    }

    /// Set maximum oracle data age (admin only)
    pub fn set_max_oracle_age(env: Env, max_age: u64) -> Result<(), Error> {
        let _admin = require_admin_auth(&env)?;

        let mut oracle_config = get_oracle_config(&env)?;
        oracle_config.max_oracle_age = max_age;

        env.storage()
            .persistent()
            .set(&symbol_short!("oracle"), &oracle_config);

        Ok(())
    }

    /// Pause oracle checks (emergency admin function)
    pub fn pause_oracle_checks(env: Env) -> Result<(), Error> {
        let _admin = require_admin_auth(&env)?;

        let mut oracle_config = get_oracle_config(&env)?;
        oracle_config.is_paused = true;

        env.storage()
            .persistent()
            .set(&symbol_short!("oracle"), &oracle_config);

        Ok(())
    }

    /// Resume oracle checks (admin function)
    pub fn resume_oracle_checks(env: Env) -> Result<(), Error> {
        let _admin = require_admin_auth(&env)?;

        let mut oracle_config = get_oracle_config(&env)?;
        oracle_config.is_paused = false;

        env.storage()
            .persistent()
            .set(&symbol_short!("oracle"), &oracle_config);

        Ok(())
    }

    /// Set maximum slippage (admin only)
    pub fn set_max_slippage(env: Env, slippage_bps: u32) -> Result<(), Error> {
        slippage::validate_slippage_bounds(slippage_bps)
            .map_err(|_| Error::InvalidSlippageConfig)?;

        let admin = require_admin_auth(&env)?;

        let mut slippage_config = get_slippage_config_internal(&env)?;
        let old_slippage = slippage_config.max_slippage_bps;
        slippage_config.max_slippage_bps = slippage_bps;

        env.storage()
            .persistent()
            .set(&symbol_short!("slippage"), &slippage_config);

        env.events().publish(
            (symbol_short!("slip_upd"),),
            SlippageConfigUpdated {
                old_slippage,
                new_slippage: slippage_bps,
                admin,
            },
        );

        Ok(())
    }

    /// Get current slippage configuration
    pub fn get_slippage_config(env: Env) -> Result<SlippageConfig, Error> {
        get_slippage_config_internal(&env)
    }

    /// Query current exchange rate from cache or oracle
    /// Returns rate with precision factor (1000000 = 1.0)
    pub fn check_exchange_rate(env: Env, _currency_pair: String) -> Result<i128, Error> {
        let oracle_config = get_oracle_config(&env)?;

        if oracle_config.is_paused {
            return Err(Error::OraclePaused);
        }

        // Try to get cached price first
        if let Some(cached) = env
            .storage()
            .temporary()
            .get::<_, PriceCache>(&symbol_short!("price"))
        {
            let current_ledger = env.ledger().timestamp();
            if current_ledger.saturating_sub(cached.timestamp) < oracle_config.max_oracle_age {
                return Ok(cached.rate);
            }
        }

        // Placeholder for actual oracle call
        let oracle_rate: i128 = 1_000_000; // 1.0 USDC/NGN (placeholder)
        let current_timestamp = env.ledger().timestamp();

        // Validate rate bounds
        oracle::validate_rate_bounds(oracle_rate).map_err(|_| Error::InvalidExchangeRate)?;

        // Cache the rate
        env.storage().temporary().set(
            &symbol_short!("price"),
            &PriceCache {
                rate: oracle_rate,
                timestamp: current_timestamp,
            },
        );

        env.events().publish(
            (symbol_short!("price_q"),),
            OracleRateQueried {
                timestamp: current_timestamp,
                rate: oracle_rate,
                source: String::from_str(&env, "oracle"),
            },
        );

        Ok(oracle_rate)
    }

    /// Validate slippage before transaction
    /// Returns error if slippage exceeds threshold
    pub fn validate_slippage(env: Env, oracle_rate: i128, actual_rate: i128) -> Result<(), Error> {
        let slippage_config = get_slippage_config_internal(&env)?;
        let rate_diff = slippage::calculate_rate_difference(oracle_rate, actual_rate);

        if rate_diff.abs() > slippage_config.max_slippage_bps as i128 {
            env.events().publish(
                (symbol_short!("slip_f"),),
                SlippageCheckFailed {
                    expected_rate: oracle_rate,
                    actual_rate,
                    threshold: slippage_config.max_slippage_bps,
                },
            );
            return Err(Error::SlippageExceeded);
        }

        Ok(())
    }
}

