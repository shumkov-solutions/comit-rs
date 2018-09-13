extern crate bitcoin_htlc;
extern crate bitcoin_support;
extern crate ethereum_support;
extern crate event_store;
extern crate rocket;
extern crate rocket_contrib;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bitcoin_rpc_client;
extern crate comit_node;
extern crate common_types;
extern crate env_logger;
extern crate ethereum_wallet;
extern crate ganache_rust_web3;
extern crate hex;
extern crate secp256k1_support;
extern crate tc_parity_parity;
#[macro_use]
extern crate log;
extern crate reqwest;
extern crate serde_json;
extern crate spectral;
extern crate tc_trufflesuite_ganachecli;
extern crate tc_web3_client;
extern crate testcontainers;
extern crate uuid;
#[macro_use]
extern crate lazy_static;

mod parity_client;
use comit_node::{
    gas_price_service::StaticGasPriceService,
    swap_protocols::rfc003::ledger_htlc_service::{
        Erc20HtlcParams, EthereumService, LedgerHtlcService,
    },
};
use common_types::{seconds::Seconds, secret::Secret};
use ethereum_support::*;
use ethereum_wallet::fake::StaticFakeWallet;
use parity_client::ParityClient;
use secp256k1_support::KeyPair;
use spectral::prelude::*;
use std::sync::Arc;
use tc_parity_parity::ParityEthereum;
use testcontainers::{clients::DockerCli, Docker};

const SECRET: &[u8; 32] = b"hello world, you are beautiful!!";

#[test]
fn given_deployed_erc20_htlc_when_redeemed_with_secret_then_money_is_transferred() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(ParityEthereum::default());
    let (_event_loop, web3) = tc_web3_client::new(&container);

    let client = ParityClient::new(web3);

    let token_contract = client.deploy_token_contract();

    let alice_keypair = KeyPair::from_secret_key_hex(
        "63be4b0d638d44b5fee5b050ab0beeeae7b68cde3d829a3321f8009cdd76b992",
    ).unwrap();
    let alice = alice_keypair.public_key().to_ethereum_address();

    let bob_keypair = KeyPair::from_secret_key_hex(
        "f8218ebf6e2626bd1415c18321496e0c5725f0e1d774c7c2eab69f7650ad6e82",
    ).unwrap();
    let bob = bob_keypair.public_key().to_ethereum_address();

    client.mint_1000_tokens(token_contract, alice);
    client.give_eth_to(alice, EthereumQuantity::from_eth(1.0));

    let service = EthereumService::new(
        Arc::new(StaticFakeWallet::from_key_pair(alice_keypair.clone())),
        Arc::new(StaticGasPriceService::default()),
        Arc::new(tc_web3_client::new(&container)),
        0,
    );

    let secret = Secret::from(SECRET.clone());

    let htlc_params = Erc20HtlcParams {
        refund_address: alice,
        success_address: bob,
        time_lock: Seconds::new(60 * 60),
        amount: U256::from(400),
        secret_hash: secret.hash(),
        token_contract_address: token_contract,
    };

    let result = service.deploy_htlc(htlc_params);

    assert_that(&result).is_ok();

    let contract_address = client.get_contract_address(result.unwrap());

    assert_eq!(client.get_token_balance(token_contract, bob), U256::from(0));
    assert_eq!(
        client.get_token_balance(token_contract, alice),
        U256::from(600)
    );
    assert_eq!(
        client.get_token_balance(token_contract, contract_address),
        U256::from(400)
    );

    let _ = client.send_data(contract_address, Some(Bytes(SECRET.to_vec())));

    assert_eq!(
        client.get_token_balance(token_contract, bob),
        U256::from(400)
    );
    assert_eq!(
        client.get_token_balance(token_contract, alice),
        U256::from(600)
    );
    assert_eq!(
        client.get_token_balance(token_contract, contract_address),
        U256::from(0)
    );
}

#[test]
fn given_deployed_htlc_when_refunded_after_timeout_then_money_is_refunded() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(ParityEthereum::default());
    let (_event_loop, web3) = tc_web3_client::new(&container);

    let client = ParityClient::new(web3);

    let token_contract = client.deploy_token_contract();

    let alice_keypair = KeyPair::from_secret_key_hex(
        "63be4b0d638d44b5fee5b050ab0beeeae7b68cde3d829a3321f8009cdd76b992",
    ).unwrap();
    let alice = alice_keypair.public_key().to_ethereum_address();

    let bob_keypair = KeyPair::from_secret_key_hex(
        "f8218ebf6e2626bd1415c18321496e0c5725f0e1d774c7c2eab69f7650ad6e82",
    ).unwrap();
    let bob = bob_keypair.public_key().to_ethereum_address();

    client.mint_1000_tokens(token_contract, alice);
    client.give_eth_to(alice, EthereumQuantity::from_eth(1.0));

    let service = EthereumService::new(
        Arc::new(StaticFakeWallet::from_key_pair(alice_keypair.clone())),
        Arc::new(StaticGasPriceService::default()),
        Arc::new(tc_web3_client::new(&container)),
        0,
    );

    let secret = Secret::from(SECRET.clone());

    let htlc_timeout = Seconds::new(2);

    let htlc_params = Erc20HtlcParams {
        refund_address: alice,
        success_address: bob,
        time_lock: htlc_timeout,
        amount: U256::from(400),
        secret_hash: secret.hash(),
        token_contract_address: token_contract,
    };

    let result = service.deploy_htlc(htlc_params);

    assert_that(&result).is_ok();

    let contract_address = client.get_contract_address(result.unwrap());

    assert_eq!(client.get_token_balance(token_contract, bob), U256::from(0));
    assert_eq!(
        client.get_token_balance(token_contract, alice),
        U256::from(600)
    );
    assert_eq!(
        client.get_token_balance(token_contract, contract_address),
        U256::from(400)
    );

    // Wait for the contract to expire
    ::std::thread::sleep(htlc_timeout.into());
    ::std::thread::sleep(htlc_timeout.into());

    let _ = client.send_data(contract_address, None);

    assert_eq!(client.get_token_balance(token_contract, bob), U256::from(0));
    assert_eq!(
        client.get_token_balance(token_contract, alice),
        U256::from(1000)
    );
    assert_eq!(
        client.get_token_balance(token_contract, contract_address),
        U256::from(0)
    );
}

#[test]
fn given_deployed_htlc_when_timeout_not_yet_reached_and_wrong_secret_then_nothing_happens() {
    let _ = env_logger::try_init();

    let container = DockerCli::new().run(ParityEthereum::default());
    let (_event_loop, web3) = tc_web3_client::new(&container);

    let client = ParityClient::new(web3);

    let token_contract = client.deploy_token_contract();

    let alice_keypair = KeyPair::from_secret_key_hex(
        "63be4b0d638d44b5fee5b050ab0beeeae7b68cde3d829a3321f8009cdd76b992",
    ).unwrap();
    let alice = alice_keypair.public_key().to_ethereum_address();

    let bob_keypair = KeyPair::from_secret_key_hex(
        "f8218ebf6e2626bd1415c18321496e0c5725f0e1d774c7c2eab69f7650ad6e82",
    ).unwrap();
    let bob = bob_keypair.public_key().to_ethereum_address();

    client.mint_1000_tokens(token_contract, alice);
    client.give_eth_to(alice, EthereumQuantity::from_eth(1.0));

    let service = EthereumService::new(
        Arc::new(StaticFakeWallet::from_key_pair(alice_keypair.clone())),
        Arc::new(StaticGasPriceService::default()),
        Arc::new(tc_web3_client::new(&container)),
        0,
    );

    let secret = Secret::from(SECRET.clone());

    let htlc_timeout = Seconds::new(10);

    let htlc_params = Erc20HtlcParams {
        refund_address: alice,
        success_address: bob,
        time_lock: htlc_timeout,
        amount: U256::from(400),
        secret_hash: secret.hash(),
        token_contract_address: token_contract,
    };

    let result = service.deploy_htlc(htlc_params);

    assert_that(&result).is_ok();

    let contract_address = client.get_contract_address(result.unwrap());

    assert_eq!(client.get_token_balance(token_contract, bob), U256::from(0));
    assert_eq!(
        client.get_token_balance(token_contract, alice),
        U256::from(600)
    );
    assert_eq!(
        client.get_token_balance(token_contract, contract_address),
        U256::from(400)
    );

    // We don't wait for the contract to expire
    let _ = client.send_data(contract_address, None);

    assert_eq!(client.get_token_balance(token_contract, bob), U256::from(0));
    assert_eq!(
        client.get_token_balance(token_contract, alice),
        U256::from(600)
    );
    assert_eq!(
        client.get_token_balance(token_contract, contract_address),
        U256::from(400)
    );
}
