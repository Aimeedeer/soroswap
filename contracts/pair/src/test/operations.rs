use soroban_sdk::{
    contracttype, 
    xdr::ToXdr, 
    Address, 
    Bytes, 
    BytesN, 
    Env, 
    IntoVal,
    testutils::{
        MockAuth,
        MockAuthInvoke,
    },
};

mod token {
    soroban_sdk::contractimport!(file = "../token/soroban_token_contract.wasm");
    pub type TokenClient<'a> = Client<'a>;
}
mod pair {
    soroban_sdk::contractimport!(file = "./target/wasm32-unknown-unknown/release/soroswap_pair.wasm");
}
mod factory {
    soroban_sdk::contractimport!(file = "../factory/target/wasm32-unknown-unknown/release/soroswap_factory.wasm");
    pub type SoroswapFactoryClient<'a> = Client<'a>; 
}

use soroban_sdk::testutils::Address as _;
use crate::{
    SoroswapPair, 
    SoroswapPairClient,
};
use token::TokenClient;
use factory::{
    SoroswapFactoryClient,
    WASM as FACTORY_WASM,
};


// A simple Pair for ordering the token's addresses and biding the salt.
#[contracttype]
#[derive(Clone)]
pub struct Pair(Address, Address);
impl Pair {
    pub fn new(a: Address, b: Address) -> Self {
        if a < b {
            Pair(a, b)
        } else {
            Pair(b, a)
        }
    }

    pub fn salt(&self, e: &Env) -> BytesN<32> {
        let mut salt = Bytes::new(e);

        // Append the bytes of token_a and token_b to the salt
        salt.append(&self.0.clone().to_xdr(e)); // can be simplified to salt.append(&self.clone().to_xdr(e)); but changes the hash
        salt.append(&self.1.clone().to_xdr(e));

        // Hash the salt using SHA256 to generate a new BytesN<32> value
        e.crypto().sha256(&salt)
    }

    pub fn token_a(&self) -> &Address {
        &self.0
    }

    pub fn token_b(&self) -> &Address {
        &self.1
    }
}

// As a general rule we will refer to alice as the deployer of contracts, bob and charlie are secondary 
// token identifiers that could or not sign contract deployements, depending on the functionality being 
// tested. The rule is alice is the main identifier token generated from the cryptographic
// methods of the library, in this case using the random generator provided.

#[test]
fn pair_initialization() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    let factory_address = &env.register_contract_wasm(None, FACTORY_WASM);
    let factory = SoroswapFactoryClient::new(&env, &factory_address);
    factory
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &factory.address,
                    fn_name: "initialize",
                    args: (alice.clone(), pair_hash.clone(),).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .initialize(&alice.clone(), &pair_hash.clone());
    factory.create_pair(&token_0.address, &token_1.address);
    let factory_pair_address = factory.get_pair(&token_0.address, &token_1.address);
    let new = SoroswapPairClient::new(&env, &factory_pair_address);
    let pair = Pair::new(token_0.address, token_1.address);
    assert_eq!((pair.0.clone(), pair.1.clone()), (new.token_0(), new.token_1()));
}

#[test]
#[should_panic]
fn token_mint_not_auth() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let bob = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let amount: i128 = 10_000;
    token_0
    .mint(&bob.clone(), &amount);
    let bob_balance = token_0.balance(&bob.clone());
    assert_eq!(bob_balance, amount);
}

#[test]
fn token_mint_mock_auths() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let bob = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let amount: i128 = 10_000;
    token_0
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_0.address,
                    fn_name: "mint",
                    args: (bob.clone(), amount,).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&bob.clone(), &amount);
    
    let bob_balance = token_0.balance(&bob.clone());

    assert_eq!(bob_balance, amount);
}

#[test]
fn pair_init_zero_balance_alice() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    let factory_address = &env.register_contract_wasm(None, FACTORY_WASM);
    let factory = SoroswapFactoryClient::new(&env, &factory_address);
    factory
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &factory.address,
                    fn_name: "initialize",
                    args: (alice.clone(), pair_hash.clone(),).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .initialize(&alice.clone(), &pair_hash.clone());
    factory.create_pair(&token_0.address, &token_1.address);
    let factory_pair_address = factory.get_pair(&token_0.address, &token_1.address);
    let new = SoroswapPairClient::new(&env, &factory_pair_address);
    let asserted: (i128, i128) = (token_0.balance(&alice.clone()), token_1.balance(&alice.clone()));
    assert_eq!(asserted, (0,0));
}

#[test]
fn token_init_zero_balance_bob() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let bob = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let asserted: (i128, i128) = (token_0.balance(&bob.clone()), token_1.balance(&bob.clone()));
    assert_eq!(asserted, (0,0));
}

#[test]
fn token_init_some_balance_alice() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    token_0
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_0.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),1_001_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &1001);
    token_1
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_1.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),1_002_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &1002);
    let asserted: (i128, i128) = (token_0.balance(&alice.clone()), token_1.balance(&alice.clone()));
    assert_eq!(asserted, (1001,1002));
}

#[test]
fn token_init_some_balance_bob() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let bob = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    token_0
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_0.address.clone(),
                    fn_name: "mint",
                    args: (bob.clone(),1_001_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&bob, &1001);
    token_1
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_1.address.clone(),
                    fn_name: "mint",
                    args: (bob.clone(),1_002_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&bob, &1002);
    let asserted: (i128, i128) = (token_0.balance(&bob.clone()), token_1.balance(&bob.clone()));
    assert_eq!(asserted, (1001,1002));
}

#[test]
fn pair_mock_auth_initialization() {
    let env: Env = Default::default();
    // env.mock_all_auths();
    let alice = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    let factory_address = &env.register_contract_wasm(None, FACTORY_WASM);
    let factory = SoroswapFactoryClient::new(&env, &factory_address);
    factory
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &factory.address,
                    fn_name: "initialize",
                    args: (alice.clone(), pair_hash.clone(),).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .initialize(&alice.clone(), &pair_hash.clone());
    factory.create_pair(&token_0.address, &token_1.address);
    let factory_pair_address = factory.get_pair(&token_0.address, &token_1.address);
    let new = SoroswapPairClient::new(&env, &factory_pair_address);
    token_0
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_0.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),2_002_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &2002);
    token_1
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_1.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(), 1_002_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &1002);
    token_0
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_0.address.clone(),
                    fn_name: "transfer",
                    args: (alice.clone(),&new.address.clone(),1_001_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .transfer(&alice.clone(), &new.address.clone(), &1001);
    token_1
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_1.address.clone(),
                    fn_name: "transfer",
                    args: (alice.clone(),&new.address.clone(),1_002_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .transfer(&alice.clone(), &new.address.clone(), &1002);

    let x = token_0.balance(&alice.clone());
    assert_eq!(x, 1001);

    let y = token_1.balance(&alice.clone());
    assert!(y == 0);
    // assert_eq!(y, 1002);

    let l = new.deposit(&alice.clone());
    // assert_eq!(new.balance().checked_mul());

    let b = new.my_balance(&alice.clone());
    assert!(b == 1);

}

#[test]
fn mint_double_factory_initialization() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    let factory_address = &env.register_contract_wasm(None, FACTORY_WASM);
    let factory = SoroswapFactoryClient::new(&env, &factory_address);
    factory
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &factory.address,
                    fn_name: "initialize",
                    args: (alice.clone(), pair_hash.clone(),).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .initialize(&alice.clone(), &pair_hash.clone());
    factory.create_pair(&token_0.address, &token_1.address);
    let factory_pair_address = factory.get_pair(&token_0.address, &token_1.address);
    let new = SoroswapPairClient::new(&env, &factory_pair_address);
    token_0
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_0.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),1_001_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice.clone(), &1001);
    token_1
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_1.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),1_001_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &1001);
    let factory_a = SoroswapFactoryClient::new(&env, &new.factory());
    let factory_b = SoroswapFactoryClient::new(&env, &new.factory());
    assert!(factory_a.pair_exists(&token_0.address.clone(), &token_1.address.clone()));
    assert!(factory_b.pair_exists(&token_0.address.clone(), &token_1.address.clone()));
    assert_eq!(factory_a.address, factory_b.address);
}

#[test]
fn factory_is_unique_and_pair_not_created() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    let factory_address = &env.register_contract_wasm(None, FACTORY_WASM);
    let factory = SoroswapFactoryClient::new(&env, &factory_address);
    factory
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &factory.address,
                    fn_name: "initialize",
                    args: (alice.clone(), pair_hash.clone(),).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .initialize(&alice.clone(), &pair_hash.clone());
    factory.create_pair(&token_0.address, &token_1.address);
    let factory_pair_address = factory.get_pair(&token_0.address, &token_1.address);
    let new = SoroswapPairClient::new(&env, &factory_pair_address);
    token_0
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_0.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),1_001_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &1001);
    token_1
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_1.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),1_001_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &1001);
    let factory_a = SoroswapFactoryClient::new(&env, &new.factory());
    let factory_b = SoroswapFactoryClient::new(&env, &new.factory());
    let token_2 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_3 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    assert!(factory.pair_exists(&token_0.address.clone(), &token_1.address.clone()));
    assert!(factory_a.pair_exists(&token_0.address.clone(), &token_1.address.clone()));
    assert!(factory_b.pair_exists(&token_0.address.clone(), &token_1.address.clone()));
    assert!(!factory_a.pair_exists(&token_2.address.clone(), &token_3.address.clone()));
    assert!(!factory_b.pair_exists(&token_2.address.clone(), &token_3.address.clone()));
    assert_eq!(factory_a.address, factory_b.address);
}

#[test]
fn two_pairs_initialization_alice() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let token_0 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_1 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_2 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    let token_3 = TokenClient::new(&env, &env.register_stellar_asset_contract(alice.clone()));
    assert_ne!(token_0.address, token_1.address);
    assert_ne!(token_2.address, token_3.address);
    token_0
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_0.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),10_000_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &10_000);
    token_1
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_1.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),10_000_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &10_000);
    token_2
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_2.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),10_000_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &10_000);
    token_3
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &token_3.address.clone(),
                    fn_name: "mint",
                    args: (alice.clone(),10_000_i128).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .mint(&alice, &10_000);
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    let factory_address = &env.register_contract_wasm(None, FACTORY_WASM);
    let factory = SoroswapFactoryClient::new(&env, &factory_address);
    factory
    .mock_auths(&[
        MockAuth {
            address: &alice.clone(),
            invoke: 
                &MockAuthInvoke {
                    contract: &factory.address,
                    fn_name: "initialize",
                    args: (alice.clone(), pair_hash.clone(),).into_val(&env),
                    sub_invokes: &[],
                },
        }
    ])
    .initialize(&alice.clone(), &pair_hash.clone());
    
    let factory_pair_address_0_1 = factory.create_pair(&token_0.address.clone(), &token_1.address.clone());
    let factory_pair_address_2_3 = factory.create_pair(&token_2.address, &token_3.address);

    assert!(factory.pair_exists(&token_0.address.clone(), &token_1.address.clone()));
    assert!(factory.pair_exists(&token_2.address.clone(), &token_3.address.clone()));

    assert_ne!(factory_pair_address_0_1, factory_pair_address_2_3);

}