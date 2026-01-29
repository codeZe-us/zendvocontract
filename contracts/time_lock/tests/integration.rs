#![cfg(test)]
extern crate std;

use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Bytes, BytesN, Env, String, xdr::ToXdr};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use zendvo_time_lock::{TimeLockContract, TimeLockContractClient};

#[test]
fn test_claim_gift() {
    let env = Env::default();
    env.mock_all_auths();

    let mut csprng = OsRng;
    let oracle_keypair = SigningKey::generate(&mut csprng);
    let oracle_pub_bytes = oracle_keypair.verifying_key().to_bytes();
    let oracle_pk = BytesN::from_array(&env, &oracle_pub_bytes);

    let contract_id = env.register(TimeLockContract, ());
    let client = TimeLockContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let oracle_address = Address::generate(&env);
    client.initialize(&admin, &oracle_pk, &oracle_address);

    let sender = Address::generate(&env);
    let recipient_phone_hash = String::from_str(&env, "phone_hash");
    let amount = 10_000_000;
    let unlock_time = env.ledger().timestamp() + 100;

    let gift_id = client.create_gift(&sender, &amount, &unlock_time, &recipient_phone_hash);

    let claimant = Address::generate(&env);
    let mut payload = Bytes::new(&env);
    payload.append(&claimant.clone().to_xdr(&env));
    payload.append(&recipient_phone_hash.clone().to_xdr(&env));
    
    let mut payload_vec = std::vec![0u8; payload.len() as usize];
    payload.copy_into_slice(&mut payload_vec);
    let signature = oracle_keypair.sign(&payload_vec);
    let proof = BytesN::from_array(&env, &signature.to_bytes());

    let res = client.try_claim_gift(&claimant, &gift_id, &proof);
    assert!(res.is_err()); // Early claim

    env.ledger().set_timestamp(unlock_time + 1);
    let res = client.try_claim_gift(&claimant, &gift_id, &proof);
    assert!(res.is_ok());
}

#[test]
fn test_withdraw_to_bank_success() {
    let env = Env::default();
    env.mock_all_auths();

    let mut csprng = OsRng;
    let oracle_keypair = SigningKey::generate(&mut csprng);
    let oracle_pk = BytesN::from_array(&env, &oracle_keypair.verifying_key().to_bytes());

    let contract_id = env.register(TimeLockContract, ());
    let client = TimeLockContractClient::new(&env, &contract_id);
    client.initialize(&Address::generate(&env), &oracle_pk, &Address::generate(&env));

    let recipient_phone_hash = String::from_str(&env, "h");
    let gift_id = client.create_gift(&Address::generate(&env), &10_000_000, &0, &recipient_phone_hash);

    let claimant = Address::generate(&env);
    let mut payload = Bytes::new(&env);
    payload.append(&claimant.clone().to_xdr(&env));
    payload.append(&recipient_phone_hash.clone().to_xdr(&env));
    let mut payload_vec = std::vec![0u8; payload.len() as usize];
    payload.copy_into_slice(&mut payload_vec);
    let proof = BytesN::from_array(&env, &oracle_keypair.sign(&payload_vec).to_bytes());

    client.claim_gift(&claimant, &gift_id, &proof);

    let res = client.try_withdraw_to_bank(&gift_id, &String::from_str(&env, "memo"), &Address::generate(&env));
    assert!(res.is_ok());
}

#[test]
fn test_withdraw_to_bank_slippage_fail() {
    let env = Env::default();
    env.mock_all_auths();

    let mut csprng = OsRng;
    let oracle_keypair = SigningKey::generate(&mut csprng);
    let oracle_pk = BytesN::from_array(&env, &oracle_keypair.verifying_key().to_bytes());
    let contract_id = env.register(TimeLockContract, ());
    let client = TimeLockContractClient::new(&env, &contract_id);
    client.initialize(&Address::generate(&env), &oracle_pk, &Address::generate(&env));

    // Actual rate is 0.99 (990,000). Oracle is 1.0 (1,000,000).
    // Set max slippage to 0.5% (50 bps). 
    // Min expected = 1.0 * (1 - 0.005) = 0.995.
    // 0.99 < 0.995 -> Should fail.
    client.set_max_slippage(&50);
    // Force slippage failure

    let recipient_phone_hash = String::from_str(&env, "h");
    let gift_id = client.create_gift(&Address::generate(&env), &10_000_000, &0, &recipient_phone_hash);
    
    let claimant = Address::generate(&env);
    let mut payload = Bytes::new(&env);
    payload.append(&claimant.clone().to_xdr(&env));
    payload.append(&recipient_phone_hash.clone().to_xdr(&env));
    let mut payload_vec = std::vec![0u8; payload.len() as usize];
    payload.copy_into_slice(&mut payload_vec);
    let proof = BytesN::from_array(&env, &oracle_keypair.sign(&payload_vec).to_bytes());
    client.claim_gift(&claimant, &gift_id, &proof);

    let res = client.try_withdraw_to_bank(&gift_id, &String::from_str(&env, "memo"), &Address::generate(&env));
    assert!(res.is_err());
}

#[test]
fn test_withdraw_to_bank_insufficient_liquidity() {
    let env = Env::default();
    env.mock_all_auths();

    let mut csprng = OsRng;
    let oracle_keypair = SigningKey::generate(&mut csprng);
    let oracle_pk = BytesN::from_array(&env, &oracle_keypair.verifying_key().to_bytes());
    let contract_id = env.register(TimeLockContract, ());
    let client = TimeLockContractClient::new(&env, &contract_id);
    client.initialize(&Address::generate(&env), &oracle_pk, &Address::generate(&env));

    let amount = 200_000_000; 
    let recipient_phone_hash = String::from_str(&env, "h");
    let gift_id = client.create_gift(&Address::generate(&env), &amount, &0, &recipient_phone_hash);

    let claimant = Address::generate(&env);
    let mut payload = Bytes::new(&env);
    payload.append(&claimant.clone().to_xdr(&env));
    payload.append(&recipient_phone_hash.clone().to_xdr(&env));
    let mut payload_vec = std::vec![0u8; payload.len() as usize];
    payload.copy_into_slice(&mut payload_vec);
    let proof = BytesN::from_array(&env, &oracle_keypair.sign(&payload_vec).to_bytes());
    client.claim_gift(&claimant, &gift_id, &proof);

    let res = client.try_withdraw_to_bank(&gift_id, &String::from_str(&env, "memo"), &Address::generate(&env));
    assert!(res.is_err());
}

#[test]
fn test_withdraw_to_bank_invalid_status() {
    let env = Env::default();
    env.mock_all_auths();

    let oracle_pk = BytesN::from_array(&env, &[0u8; 32]);
    let contract_id = env.register(TimeLockContract, ());
    let client = TimeLockContractClient::new(&env, &contract_id);
    client.initialize(&Address::generate(&env), &oracle_pk, &Address::generate(&env));

    let gift_id = client.create_gift(&Address::generate(&env), &10_000_000, &0, &String::from_str(&env, "h"));
    let res = client.try_withdraw_to_bank(&gift_id, &String::from_str(&env, "h"), &Address::generate(&env));
    assert!(res.is_err());
}
