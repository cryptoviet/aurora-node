use crate::*;
use frame_support::{pallet_prelude::*, traits::ExistenceRequirement};
use gafi_support::game::{Mining, NFT};

impl<T: Config<I>, I: 'static>
	Mining<T::AccountId, BalanceOf<T, I>, T::CollectionId, T::ItemId, T::PoolId> for Pallet<T, I>
{
	fn do_create_dynamic_pool(
		pool: &T::PoolId,
		who: &T::AccountId,
		loot_table: LootTable<T::CollectionId, T::ItemId>,
		fee: BalanceOf<T, I>,
		admin: &T::AccountId,
	) -> DispatchResult {
		// ensure pool is available
		ensure!(
			PoolOf::<T, I>::get(pool).is_none(),
			Error::<T, I>::PoolIdInUse
		);

		// Deposit balance
		<T as Config<I>>::Currency::reserve(&who, T::MiningPoolDeposit::get())?;

		// reserve resource
		for loot in &loot_table {
			if let Some(nft) = &loot.maybe_nft {
				Self::reserved_item(who, &nft.collection, &nft.item, loot.weight)?;
			}
		}

		let table = LootTableFor::<T, I>::try_from(loot_table.clone())
			.map_err(|_| Error::<T, I>::ExceedMaxLoot)?;
		LootTableOf::<T, I>::insert(pool, table);

		// create new pool
		let pool_details = PoolDetails {
			pool_type: PoolType::Dynamic,
			fee,
			owner: who.clone(),
			owner_deposit: T::MiningPoolDeposit::get(),
			admin: admin.clone(),
		};

		// insert storage
		PoolOf::<T, I>::insert(pool, pool_details);
		Self::deposit_event(Event::<T, I>::MiningPoolCreated {
			pool: *pool,
			who: who.clone(),
			pool_type: PoolType::Dynamic,
			table: loot_table,
		});

		Ok(())
	}

	fn do_create_stable_pool(
		pool: &T::PoolId,
		who: &T::AccountId,
		loot_table: LootTable<T::CollectionId, T::ItemId>,
		fee: BalanceOf<T, I>,
		admin: &T::AccountId,
	) -> DispatchResult {
		// ensure collection owner & infinite supply
		for fraction in &loot_table {
			if let Some(nft) = &fraction.maybe_nft {
				Self::ensure_collection_owner(who, &nft.collection)?;
				ensure!(
					Self::is_infinite(&nft.collection, &nft.item),
					Error::<T, I>::NotInfiniteSupply
				);
			}
		}

		<T as Config<I>>::Currency::reserve(&who, T::MiningPoolDeposit::get())?;

		// store for random
		let table = LootTableFor::<T, I>::try_from(loot_table.clone())
			.map_err(|_| Error::<T, I>::ExceedMaxLoot)?;

		LootTableOf::<T, I>::insert(pool, table);

		let pool_details = PoolDetails {
			pool_type: PoolType::Stable,
			fee,
			owner: who.clone(),
			owner_deposit: T::MiningPoolDeposit::get(),
			admin: admin.clone(),
		};

		PoolOf::<T, I>::insert(pool, pool_details);
		Self::deposit_event(Event::<T, I>::MiningPoolCreated {
			pool: *pool,
			who: who.clone(),
			pool_type: PoolType::Stable,
			table: loot_table,
		});
		Ok(())
	}

	fn do_mint(
		pool: &T::PoolId,
		who: &T::AccountId,
		target: &T::AccountId,
		amount: u32,
	) -> DispatchResult {
		if let Some(pool_details) = PoolOf::<T, I>::get(pool) {
			// SBP-M2: `return` can only be used before match, instead of using in both the cases.
			match pool_details.pool_type {
				PoolType::Dynamic => {
					Self::do_mint_dynamic_pool(pool, who, target, amount)?;
					return Ok(())
				},
				PoolType::Stable => {
					Self::do_mint_stable_pool(pool, who, target, amount)?;
					return Ok(())
				},
			};
		}
		Err(Error::<T, I>::UnknowMiningPool.into())
	}

	fn do_mint_dynamic_pool(
		pool: &T::PoolId,
		who: &T::AccountId,
		target: &T::AccountId,
		amount: u32,
	) -> DispatchResult {
		// validating item amount
		let mut table = LootTableOf::<T, I>::get(pool).clone().into();
		{
			let total_weight = Self::total_weight(&table);
			ensure!(total_weight > 0, Error::<T, I>::SoldOut);
			ensure!(amount <= total_weight, Error::<T, I>::ExceedTotalAmount);
			ensure!(
				amount <= T::MaxMintItem::get(),
				Error::<T, I>::ExceedAllowedAmount
			);
		}

		if let Some(pool_details) = PoolOf::<T, I>::get(pool) {
			// make a deposit
			<T as pallet::Config<I>>::Currency::transfer(
				&who,
				&pool_details.owner,
				pool_details.fee * amount.into(),
				ExistenceRequirement::KeepAlive,
			)?;

			// random minting
			// SBP-M2: can `Vec::new()` be incorporated?
			let mut nfts: Vec<NFT<T::CollectionId, T::ItemId>> = [].to_vec();
			{
				let mut total_weight = Self::total_weight(&table);
				let mut maybe_position = Self::random_number(total_weight, Self::gen_random());
				for _ in 0..amount {
					if let Some(position) = maybe_position {
						// ensure position
						ensure!(position < total_weight, Error::<T, I>::MintFailed);
						// SBP-M2: Try to apply match directly on Self::take_loot()
						let loot = Self::take_loot(&mut table, position);
						match loot {
							Some(maybe_nft) =>
								if let Some(nft) = maybe_nft {
									Self::repatriate_reserved_item(
										&pool_details.owner,
										&nft.collection,
										&nft.item,
										target,
										1,
										ItemBalanceStatus::Free,
									)?;
									nfts.push(nft);
								},
							None => return Err(Error::<T, I>::MintFailed.into()),
						};

						total_weight = total_weight.saturating_sub(1);
						maybe_position = Self::random_number(total_weight, position);
					} else {
						return Err(Error::<T, I>::SoldOut.into())
					}
				}

				let table = LootTableFor::<T, I>::try_from(table)
					.map_err(|_| Error::<T, I>::ExceedMaxLoot)?;
				LootTableOf::<T, I>::insert(pool, table);
				Self::deposit_event(Event::<T, I>::Minted {
					pool: *pool,
					who: who.clone(),
					target: target.clone(),
					nfts,
				});
				return Ok(())
			}
		}
		Err(Error::<T, I>::UnknowMiningPool.into())
	}

	fn do_mint_stable_pool(
		pool: &T::PoolId,
		who: &T::AccountId,
		target: &T::AccountId,
		amount: u32,
	) -> DispatchResult {
		// validating item amount
		ensure!(
			amount <= T::MaxMintItem::get(),
			Error::<T, I>::ExceedAllowedAmount
		);

		if let Some(pool_details) = PoolOf::<T, I>::get(pool) {
			// make a deposit
			<T as pallet::Config<I>>::Currency::transfer(
				&who,
				&pool_details.owner,
				pool_details.fee * amount.into(),
				ExistenceRequirement::KeepAlive,
			)?;

			// random minting
			// SBP-M2: Can `Vec::new()` be incorporated?
			let mut nfts: Vec<NFT<T::CollectionId, T::ItemId>> = [].to_vec();
			{
				let table = LootTableOf::<T, I>::get(pool).into();
				let total_weight = Self::total_weight(&table);
				let mut maybe_position = Self::random_number(total_weight, Self::gen_random());

				for _ in 0..amount {
					if let Some(position) = maybe_position {
						// ensure position
						ensure!(position < total_weight, Error::<T, I>::MintFailed);
						// SBP-M2: Why this additional variable declaration? Can't pattern-match applied directly on `Self::get_loot()`?
						let loot = Self::get_loot(&table, position);
						match loot {
							Some(maybe_nft) =>
								if let Some(nft) = maybe_nft {
									Self::add_item_balance(target, &nft.collection, &nft.item, 1)?;
									nfts.push(nft);
								},
							None => return Err(Error::<T, I>::MintFailed.into()),
						};

						maybe_position = Self::random_number(total_weight, position);
					} else {
						return Err(Error::<T, I>::SoldOut.into())
					}
				}
			}

			Self::deposit_event(Event::<T, I>::Minted {
				pool: *pool,
				who: who.clone(),
				target: target.clone(),
				nfts,
			});
			return Ok(())
		}
		Err(Error::<T, I>::UnknowMiningPool.into())
	}
}
