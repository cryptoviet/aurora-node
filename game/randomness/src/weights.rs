
//! Autogenerated weights for game_randomness
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-08-07, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `admin`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/gafi-node
// benchmark
// pallet
// --chain
// dev
// --wasm-execution
// compiled
// --pallet
// game_randomness
// --extrinsic
// *
// --steps
// 50
// --repeat
// 20
// --execution
// wasm
// --output
// ./benchmarking/randomness/weights.rs
// --template
// .maintain/frame-weight-template.hbs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for game_randomness.
pub trait WeightInfo {
	fn submit_random_seed_unsigned() -> Weight;
}

/// Weights for game_randomness using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: GameRandomness NextUnsignedAt (r:0 w:1)
	/// Proof: GameRandomness NextUnsignedAt (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	/// Storage: GameRandomness RandomSeed (r:0 w:1)
	/// Proof: GameRandomness RandomSeed (max_values: Some(1), max_size: Some(36), added: 531, mode: MaxEncodedLen)
	fn submit_random_seed_unsigned() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(10_000_000, 0)
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	/// Storage: GameRandomness NextUnsignedAt (r:0 w:1)
	/// Proof: GameRandomness NextUnsignedAt (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
	/// Storage: GameRandomness RandomSeed (r:0 w:1)
	/// Proof: GameRandomness RandomSeed (max_values: Some(1), max_size: Some(36), added: 531, mode: MaxEncodedLen)
	fn submit_random_seed_unsigned() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(10_000_000, 0)
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
}
