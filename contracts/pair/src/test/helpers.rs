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
        Ledger,
    },
    Val,
    Vec,
    vec,
};
extern crate alloc;
use alloc::boxed::Box;

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

pub enum SoroswapClient<'a> {
    TokenClient(TokenClient<'a>),
    PairClient(SoroswapPairClient<'a>),
    FactoryClient(SoroswapFactoryClient<'a>)
}

enum TestAuth<'a> {
    Mock(MockAuth<'a>)
}

impl<'a> Clone for TestAuth<'a> {
    fn clone(&self) -> TestAuth<'a> {
        let TestAuth::Mock(mock_auth) = self;
        TestAuth::Mock(
            MockAuth {
                address: &mock_auth.address,
                invoke: &mock_auth.invoke,
            }
        )
    }
}

pub struct SoroswapTestApi<'a> {
    client: SoroswapClient<'a>,
    alice: Address,
    mock_auth_invoke: MockAuthInvoke<'a>,
    sub_invoke: Box<[MockAuthInvoke<'a>]>,
    mock_auth: TestAuth<'a>,
    auth_vec: Box<&'a [MockAuth<'a>]>,
}



impl<'a> SoroswapTestApi<'a> {
    pub fn mock_auth_helper(&'a mut self, alice: &'a Address, contract: &'a Address, fn_name: &'a str, args: Vec<Val>) {
        self.alice = alice.clone();
        self.sub_invoke = Box::new([]);
        self.mock_auth_invoke = MockAuthInvoke {
            contract,
            fn_name,
            args: args.clone(),
            sub_invokes: &self.sub_invoke,
        };
        self.mock_auth = TestAuth::Mock(MockAuth {
            address: &self.alice,
            invoke: &self.mock_auth_invoke,
        });
        // self.auth_vec = Box::new(&[
        //         self.mock_auth,
        //     ]);
        // self.client 
        match &self.client {
            SoroswapClient::TokenClient(token_client) => {
                let TestAuth::Mock(mock_auth) = self.mock_auth.clone();
                let token_client = token_client;
                let auth = [mock_auth,];
                let client = token_client.mock_auths(&auth);
                // SoroswapClient::TokenClient(token_client)
            },
            SoroswapClient::PairClient(pair_client) => {
                let TestAuth::Mock(mock_auth) = self.mock_auth.clone();
                let pair_client = pair_client;
                let auth = &[mock_auth,];
                // SoroswapClient::PairClient(pair_client.mock_auths(auth))
            },
            SoroswapClient::FactoryClient(ref factory_client) => {
                let TestAuth::Mock(mock_auth) = self.mock_auth.clone();
                let factory_client = factory_client;
                // SoroswapClient::FactoryClient(factory_client.mock_auths(&[mock_auth,]))
            },
        };
    }

}
