#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, Address, BytesN, Env,
};

const BPS_DENOMINATOR: i128 = 10_000;

#[derive(Clone)]
#[contracttype]
pub struct SettlementRule {
    pub fee_bps: u32,
    pub settlement_delay_ledger: u32,
    pub auto_settle: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct FeeSplit {
    pub gross_amount: i128,
    pub fee_amount: i128,
    pub merchant_amount: i128,
    pub fee_bps: u32,
}

#[derive(Clone)]
#[contracttype]
pub struct PaymentRecord {
    pub merchant: Address,
    pub amount: i128,
    pub fee_amount: i128,
    pub merchant_amount: i128,
    pub fee_bps: u32,
    pub ledger: u32,
}

#[derive(Clone)]
#[contracttype]
enum DataKey {
    Admin,
    Merchant(Address),
    Rule(Address),
    Payment(BytesN<32>),
}

#[contracterror]
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(u32)]
pub enum SettlementError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    MerchantExists = 4,
    MerchantMissing = 5,
    InvalidFeeBps = 6,
    InvalidAmount = 7,
    DuplicatePaymentReference = 8,
}

#[contract]
pub struct SettlementContract;

#[contractimpl]
impl SettlementContract {
    pub fn init(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic_with_error!(&env, SettlementError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    pub fn register_merchant(env: Env, merchant: Address) {
        let admin = read_admin(&env);
        admin.require_auth();

        let key = DataKey::Merchant(merchant.clone());
        if env.storage().persistent().has(&key) {
            panic_with_error!(&env, SettlementError::MerchantExists);
        }

        env.storage().persistent().set(&key, &true);
        env.events()
            .publish((symbol_short!("merchant"), merchant), true);
    }

    pub fn set_settlement_rule(env: Env, merchant: Address, rule: SettlementRule) {
        let admin = read_admin(&env);
        admin.require_auth();

        if !is_merchant_registered_internal(&env, merchant.clone()) {
            panic_with_error!(&env, SettlementError::MerchantMissing);
        }
        if rule.fee_bps > BPS_DENOMINATOR as u32 {
            panic_with_error!(&env, SettlementError::InvalidFeeBps);
        }

        env.storage()
            .persistent()
            .set(&DataKey::Rule(merchant.clone()), &rule.clone());
        env.events()
            .publish((symbol_short!("set_rule"), merchant), rule.fee_bps);
    }

    pub fn store_payment_reference(env: Env, merchant: Address, reference: BytesN<32>, amount: i128) -> FeeSplit {
        merchant.require_auth();

        if !is_merchant_registered_internal(&env, merchant.clone()) {
            panic_with_error!(&env, SettlementError::MerchantMissing);
        }
        if amount <= 0 {
            panic_with_error!(&env, SettlementError::InvalidAmount);
        }

        let payment_key = DataKey::Payment(reference.clone());
        if env.storage().persistent().has(&payment_key) {
            panic_with_error!(&env, SettlementError::DuplicatePaymentReference);
        }

        let rule = read_rule_or_default(&env, merchant.clone());
        let split = calculate_split(amount, rule.fee_bps);
        let record = PaymentRecord {
            merchant: merchant.clone(),
            amount,
            fee_amount: split.fee_amount,
            merchant_amount: split.merchant_amount,
            fee_bps: rule.fee_bps,
            ledger: env.ledger().sequence(),
        };

        env.storage().persistent().set(&payment_key, &record);
        env.events()
            .publish((symbol_short!("payment"), merchant.clone()), reference);
        env.events().publish(
            (symbol_short!("split"), merchant),
            (split.gross_amount, split.fee_amount, split.merchant_amount),
        );

        split
    }

    pub fn is_merchant_registered(env: Env, merchant: Address) -> bool {
        is_merchant_registered_internal(&env, merchant)
    }

    pub fn get_settlement_rule(env: Env, merchant: Address) -> Option<SettlementRule> {
        env.storage().persistent().get(&DataKey::Rule(merchant))
    }

    pub fn calculate_fee_split(env: Env, merchant: Address, amount: i128) -> FeeSplit {
        if !is_merchant_registered_internal(&env, merchant.clone()) {
            panic_with_error!(&env, SettlementError::MerchantMissing);
        }
        if amount <= 0 {
            panic_with_error!(&env, SettlementError::InvalidAmount);
        }
        let rule = read_rule_or_default(&env, merchant);
        calculate_split(amount, rule.fee_bps)
    }

    pub fn get_payment_reference(env: Env, reference: BytesN<32>) -> Option<PaymentRecord> {
        env.storage().persistent().get(&DataKey::Payment(reference))
    }
}

fn read_admin(env: &Env) -> Address {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .unwrap_or_else(|| panic_with_error!(env, SettlementError::NotInitialized))
}

fn is_merchant_registered_internal(env: &Env, merchant: Address) -> bool {
    env.storage()
        .persistent()
        .get(&DataKey::Merchant(merchant))
        .unwrap_or(false)
}

fn read_rule_or_default(env: &Env, merchant: Address) -> SettlementRule {
    env.storage()
        .persistent()
        .get(&DataKey::Rule(merchant))
        .unwrap_or(SettlementRule {
            fee_bps: 0,
            settlement_delay_ledger: 0,
            auto_settle: false,
        })
}

fn calculate_split(amount: i128, fee_bps: u32) -> FeeSplit {
    let fee_amount = amount * (fee_bps as i128) / BPS_DENOMINATOR;
    let merchant_amount = amount - fee_amount;
    FeeSplit {
        gross_amount: amount,
        fee_amount,
        merchant_amount,
        fee_bps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Events};

    fn setup() -> (Env, SettlementContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let merchant = Address::generate(&env);
        let contract_id = env.register_contract(None, SettlementContract);
        let client = SettlementContractClient::new(&env, &contract_id);
        client.init(&admin);
        (env, client, admin, merchant)
    }

    #[test]
    fn registers_merchant_and_persists_flag() {
        let (env, client, _admin, merchant) = setup();
        let before = env.events().all().len();
        client.register_merchant(&merchant);
        assert!(client.is_merchant_registered(&merchant));
        assert!(env.events().all().len() > before);
    }

    #[test]
    fn sets_and_reads_settlement_rule() {
        let (env, client, _admin, merchant) = setup();
        client.register_merchant(&merchant);

        let rule = SettlementRule {
            fee_bps: 175,
            settlement_delay_ledger: 42,
            auto_settle: true,
        };

        let before = env.events().all().len();
        client.set_settlement_rule(&merchant, &rule);
        let got = client
            .get_settlement_rule(&merchant)
            .expect("expected settlement rule");

        assert_eq!(got.fee_bps, 175);
        assert_eq!(got.settlement_delay_ledger, 42);
        assert!(got.auto_settle);
        assert!(env.events().all().len() > before);
    }

    #[test]
    fn stores_payment_reference_once_and_calculates_split() {
        let (env, client, _admin, merchant) = setup();
        client.register_merchant(&merchant);

        let rule = SettlementRule {
            fee_bps: 250,
            settlement_delay_ledger: 0,
            auto_settle: false,
        };
        client.set_settlement_rule(&merchant, &rule);

        let reference = BytesN::from_array(&env, &[7; 32]);
        let before = env.events().all().len();
        let split = client.store_payment_reference(&merchant, &reference, &20_000);
        let stored = client
            .get_payment_reference(&reference)
            .expect("expected payment record");

        assert_eq!(split.fee_amount, 500);
        assert_eq!(split.merchant_amount, 19_500);
        assert_eq!(stored.fee_bps, 250);
        assert_eq!(stored.amount, 20_000);
        assert!(env.events().all().len() >= before + 2);
    }

    #[test]
    fn calculates_split_without_storing_reference() {
        let (_env, client, _admin, merchant) = setup();
        client.register_merchant(&merchant);
        let split = client.calculate_fee_split(&merchant, &50_000);
        assert_eq!(split.fee_amount, 0);
        assert_eq!(split.merchant_amount, 50_000);
    }

    #[test]
    #[should_panic]
    fn rejects_duplicate_merchant() {
        let (_env, client, _admin, merchant) = setup();
        client.register_merchant(&merchant);
        client.register_merchant(&merchant);
    }

    #[test]
    #[should_panic]
    fn rejects_duplicate_payment_reference() {
        let (env, client, _admin, merchant) = setup();
        client.register_merchant(&merchant);
        let reference = BytesN::from_array(&env, &[1; 32]);
        client.store_payment_reference(&merchant, &reference, &1000);
        client.store_payment_reference(&merchant, &reference, &2000);
    }

    #[test]
    #[should_panic]
    fn rejects_invalid_amount() {
        let (env, client, _admin, merchant) = setup();
        client.register_merchant(&merchant);
        let reference = BytesN::from_array(&env, &[2; 32]);
        client.store_payment_reference(&merchant, &reference, &0);
    }

    #[test]
    #[should_panic]
    fn rejects_invalid_fee_bps() {
        let (_env, client, _admin, merchant) = setup();
        client.register_merchant(&merchant);
        let bad_rule = SettlementRule {
            fee_bps: 10_001,
            settlement_delay_ledger: 0,
            auto_settle: false,
        };
        client.set_settlement_rule(&merchant, &bad_rule);
    }
}
