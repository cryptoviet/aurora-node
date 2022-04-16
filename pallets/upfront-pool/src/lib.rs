
// This file is part of Gafi Network.

// Copyright (C) 2021-2022 CryptoViet.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
use crate::weights::WeightInfo;
use frame_support::{
	dispatch::{DispatchResult, Vec},
	pallet_prelude::*,
	traits::{
		tokens::{ExistenceRequirement, WithdrawReasons},
		ReservableCurrency, Currency
	},
};
use frame_system::pallet_prelude::*;
use gafi_primitives::pool::{GafiPool, Level, Service};
use gafi_primitives::{
	pool::{Ticket, TicketType, MasterPool},
};
pub use pallet::*;
use pallet_timestamp::{self as timestamp};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {

	use super::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types it depends on.
	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config + pallet_balances::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Synchronization remove_player by call remove_player on Master Pool
		type MasterPool: MasterPool<Self::AccountId>;

		/// Max number of player can join the Upfront Pool
		#[pallet::constant]
		type MaxPlayerStorage: Get<u32>;
	}

	/// on_finalize following by steps:
	/// 1. Check if current timestamp is the correct time to charge service fee
	///	2. Charge player in the IngamePlayers - Kick player when they can't pay
	///	3. Move all players from NewPlayer to IngamePlayers
	/// 4. Update new Marktime
	///
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_block_number: BlockNumberFor<T>) {
			let _now = <timestamp::Pallet<T>>::get();

			if _now - Self::mark_time() >= Self::time_service().try_into().ok().unwrap() {
				let _ = Self::charge_ingame();
				let _ = Self::move_newplayer_to_ingame();
				MarkTime::<T>::put(_now);
				Self::deposit_event(<Event<T>>::ChargePoolService);
			}
		}
	}

	//** Storage **//

	/// Holding the number of Max Player can join the Upfront Pool
	#[pallet::storage]
	#[pallet::getter(fn max_player)]
	pub type MaxPlayer<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Holding the mark time to check if correct time to charge service fee
	/// The default value is at the time chain launched
	#[pallet::type_value]
	pub fn DefaultMarkTime<T: Config>() -> T::Moment {
		<timestamp::Pallet<T>>::get()
	}
	#[pallet::storage]
	#[pallet::getter(fn mark_time)]
	pub type MarkTime<T: Config> = StorageValue<_, T::Moment, ValueQuery, DefaultMarkTime<T>>;


	/// Honding the specific period of time to charge service fee
	/// The default value is 1 hours
	#[pallet::type_value]
	pub fn DefaultTimeService() -> u128 {
		// 1 hour
		3_600_000u128
	}
	#[pallet::storage]
	#[pallet::getter(fn time_service)]
	pub type TimeService<T: Config> = StorageValue<_, u128, ValueQuery, DefaultTimeService>;

	/// Count player on the pool to make sure not exceed the MaxPlayer
	#[pallet::storage]
	#[pallet::getter(fn player_count)]
	pub(super) type PlayerCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Holding the tickets that player used to join the pool
	#[pallet::storage]
	pub type Tickets<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Ticket<T::AccountId>>;

	/// Holding the services to serve to players, means service detail can change on runtime
	#[pallet::storage]
	#[pallet::getter(fn services)]
	pub type Services<T: Config> = StorageMap<_, Twox64Concat, Level, Service, ValueQuery>;

	/// The new players join the pool before the TimeService, whose are without charge
	#[pallet::storage]
	pub(super) type NewPlayers<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::MaxPlayerStorage>, ValueQuery>;

	/// Holing players, who stay in the pool longer than TimeService
	#[pallet::storage]
	pub type IngamePlayers<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, T::MaxPlayerStorage>, ValueQuery>;

	//** Genesis Conguration **//
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub max_player: u32,
		pub time_service: u128,
		pub services: [(Level, Service); 3],
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				max_player: 1000,
				time_service: 3_600_000u128,
				services: [
					(Level::Basic, Service::new(TicketType::Upfront(Level::Basic))),
					(Level::Medium, Service::new(TicketType::Upfront(Level::Medium))),
					(Level::Advance, Service::new(TicketType::Upfront(Level::Advance))),
				],
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<MaxPlayer<T>>::put(self.max_player);
			<MarkTime<T>>::put(<timestamp::Pallet<T>>::get());
			<TimeService<T>>::put(self.time_service);
			for service in self.services {
				Services::<T>::insert(service.0, service.1);
			}
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		PlayerNotFound,
		PlayerCountOverflow,
		ExceedMaxPlayer,
		CanNotClearNewPlayers,
		IntoBalanceFail,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ChargePoolService,
	}

	impl<T: Config> GafiPool<T::AccountId> for Pallet<T> {

		/// Join Upfront Pool
		///
		/// The origin must be Signed
		///
		/// Parameters:
		/// - `level`: The level of ticket Basic - Medium - Advance
		///
		/// Weight: `O(1)`
		fn join(sender: T::AccountId, level: Level) -> DispatchResult {
			let new_player_count =
				Self::player_count().checked_add(1).ok_or(<Error<T>>::PlayerCountOverflow)?;

			ensure!(new_player_count <= Self::max_player(), <Error<T>>::ExceedMaxPlayer);
			{
				let service = Self::get_service(level);
				let service_fee = Self::u128_try_to_balance(service.value)?;
				let double_service_fee = Self::u128_try_to_balance(service.value * 2)?;
				ensure!(T::Currency::free_balance(&sender) > double_service_fee, pallet_balances::Error::<T>::InsufficientBalance);
				<NewPlayers<T>>::try_mutate(|newplayers| newplayers.try_push(sender.clone()))
					.map_err(|_| <Error<T>>::ExceedMaxPlayer)?;
				T::Currency::reserve(
					&sender,
					service_fee,
				)?;
				T::Currency::withdraw(
					&sender,
					service_fee,
					WithdrawReasons::FEE,
					ExistenceRequirement::KeepAlive,
				)?;
			}
			Self::join_pool(sender.clone(), level, new_player_count);
			Ok(())
		}

		/// Leave Upfront Pool
		///
		/// The origin must be Signed
		///
		/// Weight: `O(1)`
		fn leave(sender: T::AccountId) -> DispatchResult {
			if let Some(ticket) = Tickets::<T>::get(sender.clone()) {
				if let Some(level) = Self::get_player_level(sender.clone()) {
					let join_time = ticket.join_time;
					let _now = Self::moment_to_u128(<timestamp::Pallet<T>>::get());

					let service_fee;
					let charge_fee;
					{
						let service = Self::get_service(level);
						let refund_fee = Self::get_refund_balance(_now, join_time, service.value);
						charge_fee = Self::u128_try_to_balance(service.value - refund_fee)?;
						service_fee = Self::u128_try_to_balance(service.value)?;
					}

					T::Currency::unreserve(&sender, service_fee);
					T::Currency::withdraw(
						&sender,
						charge_fee,
						WithdrawReasons::FEE,
						ExistenceRequirement::KeepAlive,
					)?;

					let new_player_count = Self::player_count()
						.checked_sub(1)
						.ok_or(<Error<T>>::PlayerCountOverflow)?;
					Self::remove_player(&sender, new_player_count);
				}
				Ok(())
			} else {
				Err(Error::<T>::PlayerNotFound.into())
			}
		}

		fn get_service(level: Level) -> Service {
			Services::<T>::get(level)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set MaxPlayer
		///
		/// The root must be Signed
		///
		/// Parameters:
		/// - `max_player`: new value of MaxPlayer
		///
		/// Weight: `O(1)`
		#[pallet::weight(0)]
		pub fn set_max_player(origin: OriginFor<T>, max_player: u32) -> DispatchResult {
			ensure_root(origin)?;
			<MaxPlayer<T>>::put(max_player);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn join_pool(sender: T::AccountId, level: Level, new_player_count: u32) {
		let _now = <timestamp::Pallet<T>>::get();
		let ticket = Ticket::<T::AccountId> {
			address: sender.clone(),
			join_time: Self::moment_to_u128(_now),
			ticket_type: TicketType::Upfront(level),
		};
		Tickets::<T>::insert(sender, ticket);
		<PlayerCount<T>>::put(new_player_count);
	}

	fn move_newplayer_to_ingame() -> Result<(), Error<T>> {
		let new_players: Vec<T::AccountId> = NewPlayers::<T>::get().into_inner();
		for new_player in new_players {
			<IngamePlayers<T>>::try_append(new_player).map_err(|_| <Error<T>>::ExceedMaxPlayer)?;
		}
		<NewPlayers<T>>::kill();
		Ok(())
	}

	fn charge_fee(sender: &T::AccountId, fee: BalanceOf<T>) -> DispatchResult {
		let withdraw = T::Currency::withdraw(
			&sender,
			fee,
			WithdrawReasons::FEE,
			ExistenceRequirement::KeepAlive,
		);

		match withdraw {
			Ok(_) => Ok(()),
			Err(err) => Err(err),
		}
	}

	fn get_refund_balance(
		leave_time: u128,
		join_time: u128,
		service_fee: u128,
	) -> u128 {
		let period_time = leave_time.saturating_sub(join_time);
		let fee: u128;
		if period_time < Self::time_service() {
			fee = service_fee;
		} else {
			let extra = period_time % Self::time_service();
			let serive_fee = service_fee;
			let actual_fee = serive_fee
				.saturating_mul(Self::time_service().saturating_sub(extra))
				.saturating_div(Self::time_service());
			fee = actual_fee;
		}
		fee
	}

	fn remove_player(player: &T::AccountId, new_player_count: u32) {
		T::MasterPool::remove_player(player);
		Tickets::<T>::remove(player);

		<IngamePlayers<T>>::mutate(|players| {
			if let Some(ind) = players.iter().position(|id| id == player) {
				players.swap_remove(ind);
			}
		});

		<NewPlayers<T>>::mutate(|players| {
			if let Some(ind) = players.iter().position(|id| id == player) {
				players.swap_remove(ind);
			}
		});

		<PlayerCount<T>>::put(new_player_count);
	}

	fn charge_ingame() -> Result<(), Error<T>> {
		let ingame_players: Vec<T::AccountId> = IngamePlayers::<T>::get().into_inner();
		for player in ingame_players {
			if let Some(service) = Self::get_player_service(player.clone()) {
				let fee_value = Self::u128_try_to_balance(service.value)?;



				match Self::charge_fee(&player, fee_value) {
					Ok(_) => {},
					Err(_) => {
						let new_player_count = Self::player_count()
							.checked_sub(1)
							.ok_or(<Error<T>>::PlayerCountOverflow)?;
						let _ = Self::remove_player(&player, new_player_count);
					},
				}
			}
		}
		Ok(())
	}

	fn get_player_service(player: T::AccountId) -> Option<Service> {
		if let Some(level) = Self::get_player_level(player) {
			return Some(Self::get_service(level));
		}
		None
	}

	fn get_player_level(player: T::AccountId) -> Option<Level> {
		if let Some(ticket) = Tickets::<T>::get(player) {
			if let TicketType::Upfront(level) = ticket.ticket_type {
				return Some(level);
			}
		}
		None
	}

	fn u128_try_to_balance(input: u128) -> Result<BalanceOf<T>, Error<T>> {
		match input.try_into().ok() {
			Some(val) => Ok(val),
			None => Err(<Error<T>>::IntoBalanceFail),
		}
	}

	fn moment_to_u128(input: T::Moment) -> u128 {
		sp_runtime::SaturatedConversion::saturated_into(input)
	}
}
