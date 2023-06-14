use crate::*;
use frame_support::pallet_prelude::*;
use gafi_support::game::{Amount, CreateItem};
impl<T: Config<I>, I: 'static> CreateItem<T::AccountId, T::CollectionId, T::ItemId, ItemConfig>
	for Pallet<T, I>
{
	fn do_create_item(
		who: &T::AccountId,
		collection: &T::CollectionId,
		item: &T::ItemId,
		config: &ItemConfig,
		max_supply: Option<u32>,
	) -> DispatchResult {
		if let Some(collection_owner) = T::Nfts::collection_owner(collection) {
			ensure!(
				T::Nfts::is_admin(collection, who) | T::Nfts::is_issuer(collection, who),
				Error::<T, I>::NoPermission
			);

			T::Nfts::mint_into(&collection, &item, &collection_owner, &config, false)?;

			if let Some(supply) = max_supply {
				// issues new amount of item
				Self::add_item_balance(&collection_owner, collection, item, supply)?;
			}
			MaxSupplyOf::<T, I>::insert(collection, item, max_supply);

			// Self::deposit_event(Event::<T, I>::ItemCreated {
			// 	who: who.clone(),
			// 	collection: *collection,
			// 	item: *item,
			// 	amount,
			// });
			return Ok(())
		}
		return Err(Error::<T, I>::UnknownCollection.into())
	}

	// fn do_add_item(
	// 	who: &T::AccountId,
	// 	collection: &T::CollectionId,
	// 	item: &T::ItemId,
	// 	amount: Amount,
	// ) -> DispatchResult {
	// 	// ensure permission
	// 	if let Some(collection_owner) = T::Nfts::collection_owner(collection) {
	// 		ensure!(
	// 			T::Nfts::is_admin(collection, who) | T::Nfts::is_issuer(collection, who),
	// 			Error::<T, I>::NoPermission
	// 		);
	// 		// issues amount of item
	// 		Self::add_item_balance(&collection_owner, collection, item, amount)?;

	// 		Self::deposit_event(Event::<T, I>::ItemAdded {
	// 			who: who.clone(),
	// 			collection: *collection,
	// 			item: *item,
	// 			amount,
	// 		});
	// 		return Ok(())
	// 	}
	// 	return Err(Error::<T, I>::UnknownCollection.into())
	// }
}
