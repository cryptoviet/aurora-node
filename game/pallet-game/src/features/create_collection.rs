use crate::*;
use frame_support::{
	pallet_prelude::*,
	traits::tokens::nonfungibles_v2::{Create, Inspect},
};
use gafi_support::game::CreateCollection;
use pallet_nfts::{CollectionRole, CollectionRoles};
use sp_std::vec::Vec;

impl<T: Config<I>, I: 'static>
	CreateCollection<T::AccountId, T::GameId, T::CollectionId, CollectionConfigFor<T, I>>
	for Pallet<T, I>
{
	fn do_create_game_collection(
		who: &T::AccountId,
		game_id: &T::GameId,
		admin: &T::AccountId,
		config: &CollectionConfigFor<T, I>,
	) -> DispatchResult {
		// verify create collection role
		ensure!(
			GameRoleOf::<T, I>::get(game_id, &who) ==
				Some(CollectionRoles(
					CollectionRole::Issuer | CollectionRole::Freezer | CollectionRole::Admin
				)),
			Error::<T, I>::NoPermission
		);

		let collection_id = T::Nfts::create_collection(&who, &admin, &config);

		if let Ok(id) = collection_id {
			// insert game collections
			CollectionsOf::<T, I>::try_mutate(&game_id, |collection_vec| -> DispatchResult {
				collection_vec.try_push(id).map_err(|_| Error::<T, I>::ExceedMaxCollection)?;
				Ok(())
			})?;

			// insert collection game
			GameOf::<T, I>::insert(id, game_id);
			GameCollectionConfigOf::<T, I>::insert(id, config);
			Self::deposit_event(Event::<T, I>::CollectionCreated { collection_id: id });
		}
		Ok(())
	}

	fn do_create_collection(
		who: &T::AccountId,
		admin: &T::AccountId,
		config: &CollectionConfigFor<T, I>,
	) -> DispatchResult {
		let collection_id = T::Nfts::create_collection(&who, &admin, &config);
		if let Ok(id) = collection_id {
			GameCollectionConfigOf::<T, I>::insert(id, config);
			Self::deposit_event(Event::<T, I>::CollectionCreated { collection_id: id });
		}
		Ok(())
	}

	fn do_add_collection(
		who: &T::AccountId,
		game_id: &T::GameId,
		collection_ids: &Vec<T::CollectionId>,
	) -> DispatchResult {
		// make sure signer is game owner
		Self::ensure_game_owner(who, game_id)?;

		// make sure signer is collection owner
		for id in collection_ids {
			if let Some(owner) = T::Nfts::collection_owner(&id) {
				ensure!(owner == who.clone(), Error::<T, I>::NoPermission);
			} else {
				return Err(Error::<T, I>::UnknownCollection.into())
			}
		}

		CollectionsOf::<T, I>::try_mutate(&game_id, |collection_vec| -> DispatchResult {
			collection_vec
				.try_extend(collection_ids.clone().into_iter())
				.map_err(|_| <Error<T, I>>::ExceedMaxCollection)?;
			Ok(())
		})?;

		for id in collection_ids {
			GameOf::<T, I>::insert(id, game_id);
		}

		Ok(())
	}
}
