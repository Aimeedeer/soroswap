extern crate alloc;
use alloc::boxed::Box;
use core::{
    result,
    fmt,
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

pub enum SoroswapClient<T> {
    TokenClient(Env, T),
    PairClient(Env, T),
    FactoryClient(Env, T),
    None
}

impl<T> fmt::Display for SoroswapClient<T> {
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

pub enum SoroswapClientError<T> {
    WrongBindingType(SoroswapClient<T>),
    InvokeUndefined(SoroswapClient<T>)

}

impl<T> fmt::Display for SoroswapClientError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::WrongBindingType(client_type) => "Wrong binding for type {client_type}",
                Self::InvokeUndefined(client_type) => "Undefined invoke parameters for type {client_type}",
            }
        )
    }
}

trait SoroswapError {
    fn dispatch_error(self) -> ! ;
}

impl<T> SoroswapError for SoroswapClientError<T>
{
    fn dispatch_error(self) -> ! {
        panic!("{}", self)
    }
}

pub trait SoroswapClientTrait<'a> {
    type ClientType;
    fn new(env: &Env, address: Address) -> SoroswapClient<Self::ClientType>;
    fn copy(&'a mut self) -> SoroswapClient<Self::ClientType>;
    fn client(&'a mut self) -> Self::ClientType;
    fn address(&'a mut self) -> &Address;
    fn mock_auth_helper(&'a mut self, alice: &'a Address, invoke: &'a MockAuthInvoke) -> MockAuth {
        let sub_invoke: Box<[MockAuthInvoke<'a>; 0]> = Box::<[MockAuthInvoke<'a>; 0]>::new([]); // TODO: implement sub_invoke .
        MockAuth {
            address: alice,
            invoke: invoke,
        }
    }
    fn mock_auth_invoke(&'a mut self, fn_name: &'a str, args: Vec<Val>) -> MockAuthInvoke {
        MockAuthInvoke {
            contract: &self.address(),
            fn_name,
            args: args.clone(),
            sub_invokes: &[],
        }
    }
}

impl<'a> SoroswapClientTrait<'a> for SoroswapClient<TokenClient<'a>> {
    type ClientType = TokenClient<'a>;
    fn new(env: &Env, address: Address) -> SoroswapClient<Self::ClientType> {
        let client = TokenClient::new(&env, &env.register_stellar_asset_contract(address));
        Self::TokenClient(env.clone(), client)
    }
    fn copy(&'a mut self) -> SoroswapClient<Self::ClientType> {
        let SoroswapClient::TokenClient(env, client) = self else { 
            SoroswapClientError::WrongBindingType(self.copy()).dispatch_error()
        };
        Self::new(&env.clone(), client.address.clone())
    }
    fn address(&'a mut self) -> &Address {
        let SoroswapClient::TokenClient(_, client) = self else { SoroswapClientError::WrongBindingType(self.copy()).dispatch_error() };
        &client.address
    }
    fn client(&'a mut self) -> Self::ClientType {    
        match self {
            Self::TokenClient(_, client) => { 
                let Self::TokenClient(_, client_copy) = Self::new(&client.env, client.address.clone()) else { SoroswapClientError::WrongBindingType(self.copy()).dispatch_error() };
                client_copy
            },
            _ => SoroswapClientError::WrongBindingType(self.copy()).dispatch_error(),
        }
    }
}

impl<'a> SoroswapClientTrait<'a> for SoroswapClient<SoroswapPairClient<'a>> {
    type ClientType = SoroswapPairClient<'a>;
    fn new(env: &Env, address: Address) -> SoroswapClient<Self::ClientType> {
        Self::PairClient(env.clone(), SoroswapPairClient::new(&env, &address))
    }
    fn copy(&'a mut self) -> SoroswapClient<Self::ClientType> {
        let SoroswapClient::PairClient(env, client) = self else { SoroswapClientError::WrongBindingType(self.copy()).dispatch_error() };
        Self::new(&env.clone(), client.address.clone())
    }
    fn address(&'a mut self) -> &Address {
        let SoroswapClient::PairClient(_, client) = self else { SoroswapClientError::WrongBindingType(self.copy()).dispatch_error() };
        &client.address
    }
    fn client(&'a mut self) -> Self::ClientType {
        match self {
            Self::PairClient(_, client) => { 
                let Self::PairClient(_, client_copy) = Self::new(&client.env, client.address.clone()) else { SoroswapClientError::WrongBindingType(self.copy()).dispatch_error() };
                client_copy
             },
            _ => SoroswapClientError::WrongBindingType(self.copy()).dispatch_error(),
        }
    }
}

impl<'a> SoroswapClientTrait<'a> for SoroswapClient<SoroswapFactoryClient<'a>> {
    type ClientType = SoroswapFactoryClient<'a>;
    fn new(env: &Env, address: Address) -> SoroswapClient<Self::ClientType> {
        Self::FactoryClient(env.clone(), SoroswapFactoryClient::new(&env, &env.register_stellar_asset_contract(address)))
    }
    fn copy(&'a mut self) -> SoroswapClient<Self::ClientType> {
        let SoroswapClient::FactoryClient(env, client) = self else { SoroswapClientError::WrongBindingType(self.copy()).dispatch_error() };
        Self::new(&env.clone(), client.address.clone())
    }
    fn address(&'a mut self) -> &Address {
        let SoroswapClient::FactoryClient(_, client) = self else { SoroswapClientError::WrongBindingType(self.copy()).dispatch_error() };
        &client.address
    }
    fn client(&'a mut self) -> Self::ClientType {
        match self {
            Self::FactoryClient(_, client) => { 
                let Self::FactoryClient(_, client_copy) = Self::new(&client.env, client.address.clone()) else { SoroswapClientError::WrongBindingType(self.copy()).dispatch_error() };
                client_copy
             },
            _ => SoroswapClientError::WrongBindingType(self.copy()).dispatch_error(),
        }
    }
}

pub struct SoroswapTestApi<'a, T: SoroswapClientTrait<'a>> {
    env: Env,
    client: T,
    alice: Address,
    mock_auth_invoke: Option<MockAuthInvoke<'a>>,
    sub_invoke: Option<Box<[MockAuthInvoke<'a>]>>,
    mock_auth: Option<MockAuth<'a>>,
    auth_vec: Option<Box<&'a [MockAuth<'a>]>>,
}

impl<'a> SoroswapTestApi<'a, SoroswapClient<TokenClient<'a>>> 
{
    pub fn new(alice: &Address, env: &Env) -> Self {
        let client = SoroswapClient::<TokenClient<'a>>::new(&env, alice.clone()) else { todo!() };
        Self {
            env: env.clone(),
            client,
            alice: alice.clone(),
            mock_auth_invoke: None,
            sub_invoke: None,
            mock_auth: None,
            auth_vec: None,
        }
    }
    pub fn auth(&'a mut self, alice: &'a Address, contract: &'a Address, fn_name: &'a str, args: Vec<Val>) {
        let mock_auth = self.client.mock_auth_helper(alice, &self.mock_auth_invoke.as_ref().expect("Wrong Type."));
    }
}

impl<'a> SoroswapTestApi<'a, SoroswapClient<SoroswapFactoryClient<'a>>> 
{
    pub fn new(alice: &Address, env: &Env) -> Self {
        let client = SoroswapClient::<SoroswapFactoryClient<'a>>::new(&env, alice.clone()) else { todo!() };
        Self {
            env: env.clone(),
            client,
            alice: alice.clone(),
            mock_auth_invoke: None,
            sub_invoke: None,
            mock_auth: None,
            auth_vec: None,
        }
    }
    fn invoke(&'a mut self, alice: &'a Address, fn_name: &'a str, args: Vec<Val>) -> MockAuthInvoke<'a> {
        let env = &self.env;
        let binding = Some(self.client.mock_auth_invoke(fn_name, args));
        self.mock_auth_invoke = binding.clone();

        binding.expect("Should be unreachable: the variable is created as Some(x) within the scope.")
    }
    pub fn auth(&'a mut self, alice: &'a Address, fn_name: &'a str, args: Vec<Val>) {
        match self.mock_auth_invoke {
            Some(ref invoke) => self.client.mock_auth_helper(alice, invoke),
            None => {
                panic!("{}",SoroswapClientError::InvokeUndefined(self.client.copy()))
                
            },
        };
    }
}

#[test]
fn pair_initialization<'a>() {
    let env: Env = Default::default();
    let alice = Address::random(&env);
    let token_api_0 = SoroswapTestApi::<SoroswapClient<TokenClient<'a>>>::new(&alice, &env);
    let token_api_1 = SoroswapTestApi::<SoroswapClient<TokenClient<'a>>>::new(&alice, &env);
    let mut factory_api = SoroswapTestApi::<SoroswapClient<SoroswapFactoryClient<'a>>>::new(&alice, &env);
    let pair_hash = env.deployer().upload_contract_wasm(pair::WASM);
    factory_api.invoke(&alice, "initialize", (alice.clone(), pair_hash.clone(),).into_val(&env));
    // factory_api.auth(&alice, "initialize", (alice.clone(), pair_hash.clone(),).into_val(&env));
    // soroswap_factory_client.mock_auth_helper(&alice_clone, &invoke);
    // let client = soroswap_factory_client.client();
    // let SoroswapClient::FactoryClient(factory) = soroswap_factory_client else { todo!() };
    // factory.create_pair(&token_0.address, &token_1.address);
    // let factory_pair_address = factory.get_pair(&token_0.address, &token_1.address);
    // let new = SoroswapPairClient::new(&env, &factory_pair_address);
    // let pair = Pair::new(token_0.address, token_1.address);
    // assert_eq!((pair.0.clone(), pair.1.clone()), (new.token_0(), new.token_1()));
}