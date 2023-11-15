extern crate alloc;
use alloc::boxed::Box;
use core::{
    result,
    fmt,
    marker::PhantomData
};
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

pub mod token {
    soroban_sdk::contractimport!(file = "../token/soroban_token_contract.wasm");
    pub type TokenClient<'a> = Client<'a>;
}
pub mod pair {
    soroban_sdk::contractimport!(file = "./target/wasm32-unknown-unknown/release/soroswap_pair.wasm");
}
pub mod factory {
    soroban_sdk::contractimport!(file = "../factory/target/wasm32-unknown-unknown/release/soroswap_factory.wasm");
    pub type FactoryClient<'a> = Client<'a>; 
}

use soroban_sdk::testutils::Address as _;
use crate::{
    SoroswapPair, 
    SoroswapPairClient,
};
use token::TokenClient;
use factory::{
    FactoryClient,
    WASM as FACTORY_WASM,
};

#[derive(Copy, Clone)]
pub enum SoroswapClient<'a, T> {
    TokenClient(&'a Env, T),
    PairClient(&'a Env, T),
    FactoryClient(&'a Env, T),
    None
}

impl<'a> SoroswapClient<'a, TokenClient<'a>> {
    // initialize
    pub fn from(env: &'a Env, address: &Address) -> SoroswapClient<'a, TokenClient<'a>> {
        Self::TokenClient(&env, TokenClient::new(&env, &env.register_stellar_asset_contract(address.clone())))
    }

}

impl<'a> SoroswapClient<'a, FactoryClient<'a>> {
    // initialize
    pub fn from(env: &'a Env) -> SoroswapClient<'a, FactoryClient<'a>> {
        SoroswapClient::FactoryClient(env, FactoryClient::new(&env, &env.register_contract_wasm(None, FACTORY_WASM)) )
    }
}

impl<T> fmt::Display for SoroswapClient<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::TokenClient(_, client_type) => "TokenClient",
                Self::PairClient(_, client_type) => "PairClient",
                Self::FactoryClient(_, client_type) => "FactoryClient",
                Self::None => "None"
            }
        )
    }
}

pub enum SoroswapClientError<'a, T> {
    WrongBindingType(&'a SoroswapClient<'a, T>),
    InvokeUndefined(&'a SoroswapClient<'a, T>),
    CodeUnreachable,

}

impl<T> fmt::Display for SoroswapClientError<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::WrongBindingType(client_type) => "Wrong binding for type {client_type}",
                Self::InvokeUndefined(client_type) => "Undefined invoke parameters for type {client_type}",
                Self::CodeUnreachable => "This code is intended to be unreachable."
            }
        )
    }
}

trait SoroswapError {
    fn dispatch_error(self) -> ! ;
}

impl<'a, T> SoroswapError for SoroswapClientError<'a, T>
{
    fn dispatch_error(self) -> ! {
        panic!("{}", self)
    }
}
// 
//  Please note that type MockAuth references to data of unknown size.
// 
// fn mock_auth<'a>(alice: &'a Address, contract: &'a Address, fn_name: &'a str, args: Vec<Val>) -> MockAuth<'a> {
//     let args_clone = args.clone();

//     let sub_invoke: Box<[MockAuthInvoke<'a>; 0]> = Box::<[MockAuthInvoke<'a>; 0]>::new([]); // TODO: implement sub_invoke .
//     let mock_auth = MockAuth {
//         address: alice,
//         invoke: &invoke,
//     };
//     mock_auth
// }
//

// Expected behaviour from clients.
pub trait SoroswapClientTrait<'a, ClientType: 'a>
where Self: Sized
{
    fn env(&'a self) -> &'a Env;
    fn client(&'a self) -> &'a ClientType;
    fn address(&self) -> &Address;
    fn mock_auth_helper(&'a self, env: &'a Env, alice: &'a Address, mock_auths: &'a [MockAuth<'a>; 1]) -> Self;
}

impl<'a> SoroswapClientTrait<'a, TokenClient<'a>> for SoroswapClient<'a, TokenClient<'a>> {
    fn env(&'a self) -> &'a Env {
        match self {
            Self::TokenClient(env, _) => { 
                env
             },
            _ => SoroswapClientError::WrongBindingType(self).dispatch_error(),
        }
    }
    fn address(&self) -> &Address {
        let SoroswapClient::TokenClient(_, client) = self else { SoroswapClientError::WrongBindingType(self).dispatch_error() };
        &client.address
    }
    fn client(&'a self) -> &'a TokenClient<'a> {    
        match self {
            Self::TokenClient(_, client) => { 
                &client
            },
            _ => SoroswapClientError::WrongBindingType(&self).dispatch_error(),
        }
    }
    fn mock_auth_helper(&'a self, env: &'a Env, alice: &'a Address, mock_auths: &'a [MockAuth<'a>; 1]) -> Self {
        let ref client = self.client();
        Self::TokenClient(env, client.mock_auths(mock_auths))
    }
}

impl<'a> SoroswapClientTrait<'a, SoroswapPairClient<'a>> for SoroswapClient<'a, SoroswapPairClient<'a>> {
    fn env(&'a self) -> &'a Env {
        match self {
            Self::PairClient(env, _) => { 
                env
             },
            _ => SoroswapClientError::WrongBindingType(self).dispatch_error(),
        }
    }
    fn address(&self) -> &Address {
        let SoroswapClient::PairClient(_, client) = self else { SoroswapClientError::WrongBindingType(self).dispatch_error() };
        &client.address
    }
    fn client(&'a self) -> &'a SoroswapPairClient<'a> {
        match self {
            Self::PairClient(_, client) => { 
                &client
             },
            _ => SoroswapClientError::WrongBindingType(&self).dispatch_error(),
        }
    }
    fn mock_auth_helper(&'a self, env: &'a Env, alice: &'a Address, mock_auths: &'a [MockAuth<'a>; 1]) -> Self {
        let ref client = self.client();
        Self::PairClient(env, client.mock_auths(mock_auths))
    }
}

impl<'a> SoroswapClientTrait<'a, FactoryClient<'a>> for SoroswapClient<'a, FactoryClient<'a>> {
    fn env(&'a self) -> &'a Env {
        match self {
            Self::FactoryClient(env, _) => { 
                env
             },
            _ => SoroswapClientError::WrongBindingType(self).dispatch_error(),
        }
    }
    fn address(&self) -> &Address {
        let SoroswapClient::FactoryClient(_, client) = self else { SoroswapClientError::WrongBindingType(self).dispatch_error() };
        &client.address
    }
    fn client(&'a self) -> &'a FactoryClient<'a> {
        match self {
            Self::FactoryClient(_, client) => { 
                &client
             },
            _ => SoroswapClientError::WrongBindingType(&self).dispatch_error(),
        }
    }
    fn mock_auth_helper(&'a self, env: &'a Env, alice: &'a Address, mock_auths: &'a [MockAuth<'a>; 1]) -> Self {
        let ref client = self.client();
        Self::FactoryClient(env, client.mock_auths(mock_auths))
    }
}

// SoroswapTest struct is used to indirect all the side effects of the isolated test.
// #[derive(Clone)]
pub struct SoroswapTest<'a, T, U: SoroswapClientTrait<'a, T>>
{
    pub env: Env,
    client: PhantomData<&'a T>,
    pub test_client: &'a mut U, // SoroswapClient<'a, T>,
    pub alice: Address,
    mock_auths: &'a [MockAuth<'a>; 1]
}

impl<'a, T> SoroswapTest<'a, T, SoroswapClient<'a, T>>
 where SoroswapClient<'a, T>: SoroswapClientTrait<'a, T>
{
    fn address(&'a self) -> &'a Address {
        self.test_client.address()
    }
}

impl<'a> SoroswapTest<'a, SoroswapPairClient<'a>, SoroswapClient<'a, SoroswapPairClient<'a>>> {

}

impl<'a> SoroswapTest<'a, FactoryClient<'a>, SoroswapClient<'a, FactoryClient<'a>>> {
    pub fn initialize(env: &'a Env, alice: &'a Address, test_client: &'a mut SoroswapClient<'a, FactoryClient<'a>>, mock_auths: &'a [MockAuth<'a>; 1]) -> Self {
        let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
        test_client.client().initialize(&alice.clone(), &pair_hash.clone());
        let contract_address = test_client.address();
        Self {
            env: env.clone(),
            client:PhantomData,
            test_client,
            alice: alice.clone(),
            mock_auths
        }
    }
    pub fn create_a_pair(&'a mut self) -> Address {
        let token_0 = SoroswapClient::<TokenClient>::from(&self.env, &self.alice);
        let token_1 = SoroswapClient::<TokenClient>::from(&self.env, &self.alice);
        let client = self.test_client.client();
        client.create_pair(token_0.address(), token_1.address());
        client.get_pair(token_0.address(), token_1.address())
    }
}

#[test]
fn pair_initialization() {
    let env: Env = Default::default();
    let alice: Address = Address::random(&env);
    let test_client = SoroswapClient::<FactoryClient>::from(&env);
    let contract_address = test_client.address().clone();
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    let invoke = MockAuthInvoke {
        contract: &contract_address,
        fn_name: "initialize",
        args: (alice.clone(), pair_hash.clone(),).into_val(&env),
        sub_invokes: &[],
    };
    let mock_auth = MockAuth {
        address: &alice,
        invoke: &invoke,
    };
    let mock_auths = &[mock_auth];
    let mut mocked_client = test_client.mock_auth_helper(&env, &alice, mock_auths);
    assert_eq!(test_client.address(), mocked_client.address());
    let factory_api = SoroswapTest::<FactoryClient, SoroswapClient<FactoryClient>>::initialize(&env, &alice, &mut mocked_client, mock_auths);
    factory_api.test_client.client().initialize(&alice.clone(), &pair_hash.clone());
    let client = factory_api.test_client.client();

    let token_0 = SoroswapClient::<TokenClient>::from(&env, &alice);
    let token_1 = SoroswapClient::<TokenClient>::from(&env, &alice);

    client.create_pair(&token_0.address(), &token_1.address());
    let first_pair_call = client.get_pair(&token_0.address(), &token_1.address());
    let second_pair_call = client.get_pair(&token_1.address(), &token_0.address());
    assert_eq!(first_pair_call, second_pair_call);
    let _factory_address = factory_api.address();
}