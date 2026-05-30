#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address, Env, Symbol,
};

#[derive(Clone)]
#[contracttype]
pub struct FeeConfig {
    pub platform_fee_bps: u32,
    pub network_fee_bps: u32,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    SystemParam(Symbol),
    FeeConfig,
    Anchor(Address),
}

#[contracterror]
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum GovernanceError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidFeeBps = 4,
    AnchorMissing = 5,
}

#[contract]
pub struct GovernanceContract;

#[contractimpl]
impl GovernanceContract {
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, GovernanceError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn update_system_param(env: Env, key: Symbol, value: i128) {
        let admin = read_admin(&env);
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::SystemParam(key.clone()), &value);
        env.events()
            .publish((symbol_short!("sys_param"), key), value);
    }

    pub fn get_system_param(env: Env, key: Symbol) -> Option<i128> {
        env.storage().persistent().get(&DataKey::SystemParam(key))
    }

    pub fn set_fee_config(env: Env, config: FeeConfig) {
        let admin = read_admin(&env);
        admin.require_auth();

        if config.platform_fee_bps > 10_000 || config.network_fee_bps > 10_000 {
            panic_with_error!(&env, GovernanceError::InvalidFeeBps);
        }

        env.storage().persistent().set(&DataKey::FeeConfig, &config.clone());
        env.events().publish(
            (symbol_short!("fee_cfg"),),
            (config.platform_fee_bps, config.network_fee_bps),
        );
    }

    pub fn get_fee_config(env: Env) -> Option<FeeConfig> {
        env.storage().persistent().get(&DataKey::FeeConfig)
    }

    pub fn upsert_anchor(env: Env, asset: Address, anchor: Address) {
        let admin = read_admin(&env);
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::Anchor(asset.clone()), &anchor.clone());
        env.events().publish((symbol_short!("anchor_up"), asset), anchor);
    }

    pub fn remove_anchor(env: Env, asset: Address) {
        let admin = read_admin(&env);
        admin.require_auth();
        let key = DataKey::Anchor(asset.clone());

        if !env.storage().persistent().has(&key) {
            panic_with_error!(&env, GovernanceError::AnchorMissing);
        }

        env.storage().persistent().remove(&key);
        env.events().publish((symbol_short!("anchor_rm"), asset), true);
    }

    pub fn get_anchor(env: Env, asset: Address) -> Option<Address> {
        env.storage().persistent().get(&DataKey::Anchor(asset))
    }
}

fn read_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, GovernanceError::NotInitialized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};

    fn setup() -> (Env, GovernanceContractClient<'static>, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, GovernanceContract);
        let client = GovernanceContractClient::new(&env, &contract_id);
        client.init(&admin);
        (env, client, admin)
    }

    #[test]
    fn updates_system_parameters() {
        let (env, client, _admin) = setup();
        let key = Symbol::new(&env, "max_settle");
        let before = env.events().all().len();
        client.update_system_param(&key, &1440);
        assert_eq!(client.get_system_param(&key), Some(1440));
        assert!(env.events().all().len() > before);
    }

    #[test]
    fn sets_fee_config() {
        let (env, client, _admin) = setup();
        let cfg = FeeConfig {
            platform_fee_bps: 120,
            network_fee_bps: 35,
        };
        let before = env.events().all().len();
        client.set_fee_config(&cfg);
        let got = client.get_fee_config().expect("expected config");
        assert_eq!(got.platform_fee_bps, 120);
        assert_eq!(got.network_fee_bps, 35);
        assert!(env.events().all().len() > before);
    }

    #[test]
    fn upserts_and_removes_anchor() {
        let (env, client, _admin) = setup();
        let asset = Address::generate(&env);
        let anchor = Address::generate(&env);

        let before_upsert = env.events().all().len();
        client.upsert_anchor(&asset, &anchor);
        assert_eq!(client.get_anchor(&asset), Some(anchor.clone()));
        assert!(env.events().all().len() > before_upsert);

        let before_remove = env.events().all().len();
        client.remove_anchor(&asset);
        assert_eq!(client.get_anchor(&asset), None);
        assert!(env.events().all().len() > before_remove);
    }

    #[test]
    #[should_panic]
    fn rejects_invalid_fee_bps() {
        let (_env, client, _admin) = setup();
        let cfg = FeeConfig {
            platform_fee_bps: 20_000,
            network_fee_bps: 10,
        };
        client.set_fee_config(&cfg);
    }

    #[test]
    #[should_panic]
    fn rejects_removing_unknown_anchor() {
        let (env, client, _admin) = setup();
        let missing_asset = Address::generate(&env);
        client.remove_anchor(&missing_asset);
    }

    #[test]
    #[should_panic]
    fn rejects_reinitialization() {
        let (env, client, admin) = setup();
        client.init(&admin);
        let _ = env;
    }
}
