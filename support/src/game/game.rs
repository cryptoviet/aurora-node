use frame_support::pallet_prelude::DispatchResult;
use sp_runtime::{Percent, TokenError};
use sp_std::vec::Vec;

use super::{TradeConfig, Package, Bundle};

pub type Amount = u32;
pub type Level = u32;
pub trait GameSetting<AccountId, GameId, BlockNumber> {
	/// Do create a new game
	///
	/// Implementing the function create game
	///
	/// Parameters:
	/// - `who`: signer and game owner
	/// - `id`: new game id
	/// - `admin`: admin
	fn do_create_game(who: &AccountId, game: &GameId, admin: &AccountId) -> DispatchResult;

	/// Do set swap fee
	///
	///  Implementing the function set swap fee
	///
	/// Parameters:
	/// - `who`: owner
	/// - `game`: game id
	/// - `fee`: percent of swapping volume
	/// - `start_block`: block apply swap fee
	fn do_set_swap_fee(
		who: &AccountId,
		game: &GameId,
		fee: Percent,
		start_block: BlockNumber,
	) -> DispatchResult;
}

pub trait CreateCollection<AccountId, GameId, CollectionId, CollectionConfig> {
	/// Do create game collection
	///
	/// Create collection for specific game
	///
	/// Parameters:
	/// - `who`: signer and collection owner
	/// - `game`: game id
	/// - `admin`: if admin not provided, owner also an admin
	/// - `config`: collection configuration
	fn do_create_game_collection(
		who: &AccountId,
		game: &GameId,
		admin: &AccountId,
		config: &CollectionConfig,
	) -> DispatchResult;

	/// Do create collection
	///
	/// Create a pure collection
	///
	/// Parameters:
	/// - `who`: signer and collection owner
	/// - `admin`: admin role
	/// - `config`: collection configuration
	fn do_create_collection(
		who: &AccountId,
		admin: &AccountId,
		config: &CollectionConfig,
	) -> DispatchResult;

	/// Do add collection
	///
	/// Add more amount on an existing game
	///
	/// Parameters:
	/// - `who`: signer and collection owner
	/// - `game`: game id
	/// - `collection_ids`: collection ids
	fn do_add_collection(
		who: &AccountId,
		game: &GameId,
		collection_ids: &Vec<CollectionId>,
	) -> DispatchResult;
}

pub trait CreateItem<AccountId, CollectionId, ItemId, ItemConfig> {
	/// Do Create item
	///
	/// Create items for collection
	///
	/// Parameters:
	/// - `who`: signer
	/// - `collection`: collection id
	/// - `item`: item id
	/// - `amount`: amount
	fn do_create_item(
		who: &AccountId,
		collection: &CollectionId,
		item: &ItemId,
		config: &ItemConfig,
		amount: Amount,
	) -> DispatchResult;

	/// Do add item
	///
	/// Add number amount of item in collection
	///
	/// Parameters:
	/// - `who`: signer
	/// - `collection`: collection id
	/// - `item`: item id
	/// - `amount`: amount
	fn do_add_item(
		who: &AccountId,
		collection: &CollectionId,
		item: &ItemId,
		amount: Amount,
	) -> DispatchResult;
}

pub trait MutateItem<AccountId, GameId, CollectionId, ItemId> {
	/// Mint
	///
	/// Random mint item in the collection
	///
	/// Parameters:
	/// - `_who`: sender
	/// - `_collection`: collection id
	/// - `_target`: recipient account, default `minter`
	///
	/// By default, this is not a supported operation.
	fn do_mint(
		_who: &AccountId,
		_collection: &CollectionId,
		_target: &AccountId,
		_amount: Amount,
	) -> DispatchResult {
		Err(TokenError::Unsupported.into())
	}

	/// Burn
	///
	/// Burn item
	///
	/// Parameters:
	/// - `who`: item owner
	/// - `collection`: collection id
	/// - `item`: item id
	/// - `amount`: amount of items to burn
	fn do_burn(
		who: &AccountId,
		collection: &CollectionId,
		item: &ItemId,
		amount: Amount,
	) -> DispatchResult;
}

pub trait UpgradeItem<AccountId, Balance, CollectionId, ItemId, ItemConfig, StringLimit> {
	/// Do Set Upgrade Item
	///
	/// Set upgrade item                          
	///
	/// Parameters:
	/// - `who`: item owner
	/// - `collection`: collection id
	/// - `item`: item id
	/// - `data`: metadata
	/// - `level`: upgrade level
	/// - `fee`: upgrade fee
	fn do_set_upgrade_item(
		who: &AccountId,
		collection: &CollectionId,
		item: &ItemId,
		new_item: &ItemId,
		config: &ItemConfig,
		level: Level,
		fee: Balance,
	) -> DispatchResult;

	/// Do Upgrade Item
	///
	/// Upgrade item to the next level
	///
	/// Parameters:
	/// - `who`: who
	/// - `collection`: collection id
	/// - `item`: item id
	/// - `amount`: amount of items
	fn do_upgrade_item(
		who: &AccountId,
		collection: &CollectionId,
		item: &ItemId,
		amount: Amount,
	) -> DispatchResult;
}

pub trait TransferItem<AccountId, CollectionId, ItemId> {

	/// Do Transfer Item
	///
	/// Transfer amount of item from `who` to `distination`
	///
	/// Parameters:
	/// - `who`: from account
	/// - `collection`: collection id
	/// - `item`: item id
	/// - `destination`: destination account
	/// - `amount`: amount of items
	fn do_transfer_item(
		who: &AccountId,
		collection: &CollectionId,
		item: &ItemId,
		destination: &AccountId,
		amount: Amount,
	) -> DispatchResult;

	fn swap() -> DispatchResult;
}

pub trait Trade<AccountId, CollectionId, ItemId, TradeId, Price> {

	/// Do Set Price
	///
	/// Set item price for selling
	///
	/// Parameters:
	/// - `who`: seller
	/// - `collection`: collection id
	/// - `item`: item id
	/// - `config`: trade config
	fn do_set_price(
		who: &AccountId,
		collection: &CollectionId,
		item: &ItemId,
		config: &TradeConfig<Price>,
	) -> DispatchResult;

	/// Do Buy Item
	/// 
	/// Buy items from specific seller
	/// 
	/// Parameters:
	/// - `who`: buyer
	/// - `collection`: collection id
	/// - `item`: item id
	/// - `seller`: seller
	/// - `amount`: amount
	/// - `bid_price`: price of each item
	fn do_buy_item(
		who: &AccountId,
		collection: &CollectionId,
		item: &ItemId,
		seller: &AccountId,
		amount: Amount,
		bid_price: Price
	) -> DispatchResult;

	/// Do Set Bundle
	/// 
	/// Bundling for sale
	/// 
	/// Parameters:
	/// - `id`: bundle id
	/// - `who`: seller
	/// - `bundle`: bundle
	/// - `price`: price of bundle
	fn do_set_bundle(
		id: &TradeId,
		who: &AccountId,
		bundle: Bundle<CollectionId, ItemId>,
		price: Price,
	) -> DispatchResult;

	/// Do Buy Bundle
	/// 
	/// Buy a bundle from bundle id
	/// 
	/// Parameters:
	/// - `id`: bundle id
	/// - `who`: buyer
	/// - `bid_price`: the bid price for the bundle
	fn do_buy_bundle(
		id: &TradeId,
		who: &AccountId,
		bid_price: Price,
	) -> DispatchResult;

}

pub trait Destroy<E> {
	fn destroy() -> Result<(), E>;
}

// pub trait Support {
// 	fn gen_id() -> CollectionId;
// }
