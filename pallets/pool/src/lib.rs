// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, ReservableCurrency},
};
use frame_system::pallet_prelude::*;
use gafi_primitives::{
	pool::{GafiPool, Level, Service, Ticket, TicketType, PlayerTicket, MasterPool},
};
use pallet_timestamp::{self as timestamp};

use crate::weights::WeightInfo;
#[cfg(feature = "std")]
use frame_support::serde::{Deserialize, Serialize};
use scale_info::TypeInfo;
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{Twox64Concat};

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_timestamp::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Currency: Currency<Self::AccountId>;
		type UpfrontPool: GafiPool<Self::AccountId>;
		type StakingPool: GafiPool<Self::AccountId>;
		type WeightInfo: WeightInfo;
		// type SponsoredPool: GafiPool<Self::AccountId>;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
	#[derive(Eq, PartialEq, Clone, Copy, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
	pub struct TicketInfo {
		pub ticket_type: TicketType,
		pub ticket_remain: u32,
	}

	impl TicketInfo {
		pub fn withdraw_ticket(&self) -> Option<Self> {
			if let Some(new_ticket_remain) = self.ticket_remain.checked_sub(1) {
				return Some(TicketInfo {
					ticket_remain: new_ticket_remain,
					ticket_type: self.ticket_type,
				});
			}
			None
		}

		pub  fn renew_ticket(&self, new_remain: u32) -> Self {
			TicketInfo {
				ticket_remain: new_remain,
				ticket_type: self.ticket_type,
			}
		}

	}


	#[pallet::storage]
	pub(super) type Tickets<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, TicketInfo>;

	/// Holding the mark time to check if correct time to charge service fee
	/// The default value is at the time chain launched
	#[pallet::type_value]
	pub fn DefaultMarkTime<T: Config>() -> u128 {
		<timestamp::Pallet<T>>::get().try_into().ok().unwrap()
	}
	#[pallet::storage]
	#[pallet::getter(fn mark_time)]
	pub type MarkTime<T: Config> = StorageValue<_, u128, ValueQuery, DefaultMarkTime<T>>;

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

	/// on_finalize following by steps:
	/// 1. Check if current timestamp is the correct time to charge service fee
	///	2. Charge player in the IngamePlayers - Kick player when they can't pay
	///	3. Move all players from NewPlayer to IngamePlayers
	/// 4. Update new Marktime
	///
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_block_number: BlockNumberFor<T>) {
			let _now: u128 = <timestamp::Pallet<T>>::get().try_into().ok().unwrap();

			if _now - Self::mark_time() >= Self::get_timeservice() {

				MarkTime::<T>::put(_now);
			}
		}
	}


	//** Genesis Conguration **//
	#[pallet::genesis_config]
	pub struct GenesisConfig {
		pub time_service: u128,
	}

	#[cfg(feature = "std")]
	impl Default for GenesisConfig {
		fn default() -> Self {
			Self {
				time_service: 3_600_000u128,
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig {
		fn build(&self) {
			<TimeService<T>>::put(self.time_service);
			let _now: u128 = <timestamp::Pallet<T>>::get().try_into().ok().unwrap();
			<MarkTime<T>>::put(_now);
		}
	}
		

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Joined { sender: T::AccountId, ticket: TicketType },
		Leaved { sender: T::AccountId, ticket: TicketType },
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyJoined,
		NotFoundInPool,
		ComingSoon,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(<T as pallet::Config>::WeightInfo::join(100u32, *ticket))]
		pub fn join(origin: OriginFor<T>, ticket: TicketType) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Tickets::<T>::get(sender.clone()) == None, <Error<T>>::AlreadyJoined);

			match ticket {
				TicketType::Upfront(level) => T::UpfrontPool::join(sender.clone(), level)?,
				TicketType::Staking(level) => T::StakingPool::join(sender.clone(), level)?,
				TicketType::Sponsored(_) => {
					return Err(Error::<T>::ComingSoon.into());
				},
			}

			let service = Self::get_service(ticket);

			let ticket_info = TicketInfo {
				ticket_type: ticket,
				ticket_remain: service.tx_limit,
			};

			Tickets::<T>::insert(sender.clone(), ticket_info);
			Self::deposit_event(Event::<T>::Joined { sender, ticket });
			Ok(())
		}

		#[pallet::weight(<T as pallet::Config>::WeightInfo::leave(100u32))]
		pub fn leave(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			if let Some(ticket) = Tickets::<T>::get(sender.clone()) {
				match ticket.ticket_type {
					TicketType::Upfront(_) => T::UpfrontPool::leave(sender.clone())?,
					TicketType::Staking(_) => T::StakingPool::leave(sender.clone())?,
					TicketType::Sponsored(_) => {
						return Err(Error::<T>::ComingSoon.into());
					},
				}
				Tickets::<T>::remove(sender.clone());
				Self::deposit_event(Event::<T>::Leaved { sender: sender, ticket: ticket.ticket_type});
				Ok(())
			} else {
				return Err(Error::<T>::NotFoundInPool.into());
			}
		}
	}

	impl<T: Config> PlayerTicket<T::AccountId> for Pallet<T> {
		fn use_ticket(player: T::AccountId) -> Option<TicketType> {
			if let Some(ticket_info) = Tickets::<T>::get(player.clone()) {
				if let Some(new_ticket_info) = ticket_info.withdraw_ticket() {
					Tickets::<T>::insert(player, new_ticket_info);
					return Some(new_ticket_info.ticket_type);
				}
			}
			None
		}

		fn get_service(ticket: TicketType) -> Service {
			match ticket {
				TicketType::Upfront(level) => T::UpfrontPool::get_service(level),
				TicketType::Staking(level) => T::StakingPool::get_service(level),
				TicketType::Sponsored(_) => todo!(),
			}
		}
	}

	impl<T: Config> MasterPool<T::AccountId> for Pallet<T> {
		fn remove_player(player: &T::AccountId) {
			Tickets::<T>::remove(&player);
		}

		fn get_timeservice() -> u128 {
			TimeService::<T>::get()
		}

		fn get_marktime() -> u128 {
			MarkTime::<T>::get()
		}

		// fn renew_ticket(player: &T::AccountId) {
		// 	if let Some(ticket_info) = Tickets::<T>::get(player.clone()) {
		// 			let service = Self::get_service(ticket_info.ticket_type);
		// 			let new_ticket = ticket_info.renew_ticket(service.tx_limit);
		// 			Tickets::<T>::insert(player, new_ticket);
		// 	}
		// }
	}
}
