use crate::{mock::*, Error, Tickets, Whitelist};
use codec::Decode;
use frame_support::{assert_err, assert_ok, traits::Currency};
use gafi_primitives::{
	constant::ID,
	currency::{unit, NativeToken::GAKI},
};
use sp_core::{
	offchain::{testing::TestOffchainExt, OffchainWorkerExt},
	sr25519, H160,
};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{
	app_crypto::RuntimePublic,
	offchain::{testing, TransactionPoolExt},
	Permill,
};
use sponsored_pool::{PoolOwned, Pools};
use std::{str::FromStr, sync::Arc};

#[cfg(feature = "runtime-benchmarks")]
use sponsored_pool::CustomPool;

fn make_deposit(account: &sr25519::Public, balance: u128) {
	let _ = pallet_balances::Pallet::<Test>::deposit_creating(account, balance);
}

fn new_account(account: u32, balance: u128) -> sr25519::Public {
	let keystore = KeyStore::new();
	let acc: sr25519::Public = keystore
		.sr25519_generate_new(sp_runtime::KeyTypeId::from(account), None)
		.unwrap();
	make_deposit(&acc, balance);
	assert_eq!(Balances::free_balance(&acc), balance);
	return acc
}

#[test]
fn join_staking_pool_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);

		assert_ok!(Pool::join(
			Origin::signed(account.clone()),
			STAKING_BASIC_ID,
		));

		assert_eq!(
			Balances::free_balance(account),
			account_balance - 1000 * unit(GAKI)
		);
	})
}

#[test]
fn leave_all_system_pool_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);
		assert_ok!(Pool::join(
			Origin::signed(account.clone()),
			STAKING_BASIC_ID,
		));
		assert_ok!(Pool::leave_all(Origin::signed(account.clone())));

		assert_eq!(
			Tickets::<Test>::iter_prefix_values(account.clone()).count(),
			0
		);

		assert_ok!(Pool::join(
			Origin::signed(account.clone()),
			UPFRONT_BASIC_ID,
		));
		assert_ok!(Pool::leave_all(Origin::signed(account.clone())));

		assert_eq!(
			Tickets::<Test>::iter_prefix_values(account.clone()).count(),
			0
		);
	})
}

fn create_pool(
	account: sr25519::Public,
	targets: Vec<H160>,
	pool_value: u128,
	tx_limit: u32,
	discount: Permill,
) -> ID {
	let account_balance: u128 = Balances::free_balance(&account);
	assert_ok!(SponsoredPool::create_pool(
		Origin::signed(account.clone()),
		targets,
		pool_value,
		discount,
		tx_limit
	));

	assert_eq!(
		Balances::free_balance(&account),
		account_balance - pool_value
	);
	let pool_owned = PoolOwned::<Test>::get(account.clone());
	let new_pool = Pools::<Test>::get(pool_owned[pool_owned.len() - 1]).unwrap();
	assert_eq!(new_pool.owner, account);
	assert_eq!(new_pool.tx_limit, tx_limit);
	assert_eq!(new_pool.discount, discount);
	new_pool.id
}

#[test]
fn leave_all_custom_pool_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);
		let pool_value = 1000 * unit(GAKI);

		let account2 = new_account(1, account_balance);
		{
			let pool_id = create_pool(
				account.clone(),
				vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
				pool_value,
				10,
				Permill::from_percent(70),
			);
			assert_ok!(Pool::join(Origin::signed(account2.clone()), pool_id));
		}

		// next random value
		run_to_block(2);
		{
			let pool_id = create_pool(
				account.clone(),
				vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
				pool_value,
				10,
				Permill::from_percent(70),
			);
			assert_ok!(Pool::join(Origin::signed(account2.clone()), pool_id));
		}

		assert_ok!(Pool::leave_all(Origin::signed(account2.clone())));
		assert_eq!(PoolOwned::<Test>::get(account2.clone()), [].to_vec());
		assert_eq!(
			Tickets::<Test>::iter_prefix_values(account2.clone()).count(),
			0
		);
	})
}

#[test]
#[cfg(feature = "runtime-benchmarks")]
fn get_ticket_service_works() {
	ExtBuilder::default().build_and_execute(|| {
		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);
		let id = [1; 32];

		SponsoredPool::add_default(account.clone(), id);
		let service = Pool::get_ticket_service(id).unwrap();

		assert_eq!(service.tx_limit, 0);
		assert_eq!(service.discount, Permill::from_percent(0));
	})
}

#[test]
fn whitelist_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);

		let balance = 100_000_000 * unit(GAKI);
		let account = new_account(0, balance);
		let pool_value = 1000 * unit(GAKI);

		let pool_id = create_pool(
			account.clone(),
			vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
			pool_value,
			10,
			Permill::from_percent(70),
		);

		let player = new_account(1, balance);

		assert_ok!(Pool::query_whitelist(
			Origin::signed(player.clone()),
			pool_id
		));

		assert_eq!(Whitelist::<Test>::get(player.clone()).unwrap(), pool_id);
	})
}

fn test_pub() -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([1u8; 32])
}

#[test]
fn should_submit_raw_unsigned_transaction_on_chain() {
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();

	let keystore = KeyStore::new();

	let mut t = sp_io::TestExternalities::default();
	t.register_extension(OffchainWorkerExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	t.execute_with(|| {
		// when query pool
		let mut pool_id;
		let account_balance = 1_000_000 * unit(GAKI);
		let player = new_account(1, account_balance);
		run_to_block(1);
		{
			let account = new_account(0, account_balance);
			let pool_value = 1000 * unit(GAKI);

			pool_id = create_pool(
				account.clone(),
				vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
				pool_value,
				10,
				Permill::from_percent(70),
			);

			assert_ok!(Pool::query_whitelist(
				Origin::signed(player.clone()),
				pool_id
			));
		}

		assert_eq!(Whitelist::<Test>::get(player.clone()).unwrap(), pool_id);
		assert_ok!(Pool::verify_whitelist_and_send_raw_unsign(2));

		// then
		let tx = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature, None);
		assert_eq!(
			tx.call,
			Call::Pool(crate::Call::approve_whitelist_unsigned {
				player,
				pool_id,
			})
		);
	});
}

#[test]
fn approve_whitelist_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);

		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);
		let pool_value = 1000 * unit(GAKI);

		let pool_id = create_pool(
			account.clone(),
			vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
			pool_value,
			10,
			Permill::from_percent(70),
		);

		let player = new_account(1, account_balance);

		assert_ok!(Pool::query_whitelist(
			Origin::signed(player.clone()),
			pool_id
		));

		assert_ok!(Pool::approve_whitelist(
			Origin::signed(account),
			player,
			pool_id
		));

		assert_eq!(Whitelist::<Test>::get(player).is_none(), true);
	})
}

#[test]
fn query_whitelist_should_fail_pool_not_found() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);
		let pool_id = [0; 32];
		let account_balance = 1_000_000 * unit(GAKI);

		let player = new_account(1, account_balance);

		assert_err!(
			Pool::query_whitelist(Origin::signed(player.clone()), pool_id),
			Error::<Test>::PoolNotFound
		);
	})
}

#[test]
fn approve_whitelist_should_fail_pool_not_found() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);

		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);
		let pool_value = 1000 * unit(GAKI);

		let pool_id = create_pool(
			account.clone(),
			vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
			pool_value,
			10,
			Permill::from_percent(70),
		);

		let player = new_account(1, account_balance);
		assert_ok!(Pool::query_whitelist(
			Origin::signed(player.clone()),
			pool_id
		));
		let pool_id = [0; 32];
		assert_err!(
			Pool::approve_whitelist(Origin::signed(account), player, pool_id),
			Error::<Test>::PoolNotFound
		);
	})
}

#[test]
fn approve_whitelist_should_fail_not_pool_owner() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);

		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);
		let pool_value = 1000 * unit(GAKI);

		let pool_id = create_pool(
			account.clone(),
			vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
			pool_value,
			10,
			Permill::from_percent(70),
		);

		let player = new_account(1, account_balance);
		assert_ok!(Pool::query_whitelist(
			Origin::signed(player.clone()),
			pool_id
		));

		let account2 = new_account(2, account_balance);
		assert_err!(
			Pool::approve_whitelist(Origin::signed(account2), player, pool_id),
			Error::<Test>::NotPoolOwner
		);
	})
}

#[test]
fn approve_whitelist_should_fail_player_not_whitelist() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);

		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);
		let pool_value = 1000 * unit(GAKI);

		let pool_id = create_pool(
			account.clone(),
			vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
			pool_value,
			10,
			Permill::from_percent(70),
		);

		let player = new_account(1, account_balance);

		assert_err!(
			Pool::approve_whitelist(Origin::signed(account), player, pool_id),
			Error::<Test>::PlayerNotWhitelist
		);
	})
}

#[test]
fn approve_whitelist_unsigned_works() {
	ExtBuilder::default().build_and_execute(|| {
		run_to_block(1);

		let account_balance = 1_000_000 * unit(GAKI);
		let account = new_account(0, account_balance);
		let pool_value = 1000 * unit(GAKI);

		let pool_id = create_pool(
			account.clone(),
			vec![H160::from_str("b28049C6EE4F90AE804C70F860e55459E837E84b").unwrap()],
			pool_value,
			10,
			Permill::from_percent(70),
		);

		let player = new_account(1, account_balance);

		assert_ok!(Pool::query_whitelist(
			Origin::signed(player.clone()),
			pool_id
		));

		assert_ok!(Pool::approve_whitelist(
			Origin::signed(account),
			player,
			pool_id
		));
	})
}
