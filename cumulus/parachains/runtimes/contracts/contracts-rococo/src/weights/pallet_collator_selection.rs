// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Autogenerated weights for `pallet_collator_selection`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2025-02-21, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `0108b0983295`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --extrinsic=*
// --runtime=target/production/wbuild/contracts-rococo-runtime/contracts_rococo_runtime.wasm
// --pallet=pallet_collator_selection
// --header=/__w/polkadot-sdk/polkadot-sdk/cumulus/file_header.txt
// --output=./cumulus/parachains/runtimes/contracts/contracts-rococo/src/weights
// --wasm-execution=compiled
// --steps=50
// --repeat=20
// --heap-pages=4096
// --no-storage-info
// --no-min-squares
// --no-median-slopes
// --genesis-builder-policy=none
// --exclude-pallets=pallet_xcm

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_collator_selection`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_collator_selection::WeightInfo for WeightInfo<T> {
	/// Storage: `Session::NextKeys` (r:20 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::Invulnerables` (r:0 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 20]`.
	fn set_invulnerables(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `41 + b * (79 ±0)`
		//  Estimated: `1030 + b * (2554 ±0)`
		// Minimum execution time: 9_970_000 picoseconds.
		Weight::from_parts(6_811_549, 0)
			.saturating_add(Weight::from_parts(0, 1030))
			// Standard Error: 9_340
			.saturating_add(Weight::from_parts(4_092_958, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(Weight::from_parts(0, 2554).saturating_mul(b.into()))
	}
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 19]`.
	/// The range of component `c` is `[1, 99]`.
	fn add_invulnerable(b: u32, c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `623 + b * (32 ±0) + c * (53 ±0)`
		//  Estimated: `6287 + b * (37 ±0) + c * (53 ±0)`
		// Minimum execution time: 44_284_000 picoseconds.
		Weight::from_parts(44_913_672, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 10_427
			.saturating_add(Weight::from_parts(133_492, 0).saturating_mul(b.into()))
			// Standard Error: 1_976
			.saturating_add(Weight::from_parts(144_419, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 37).saturating_mul(b.into()))
			.saturating_add(Weight::from_parts(0, 53).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[2, 20]`.
	fn remove_invulnerable(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `8 + b * (32 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 10_016_000 picoseconds.
		Weight::from_parts(10_124_278, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 1_457
			.saturating_add(Weight::from_parts(150_553, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::DesiredCandidates` (r:0 w:1)
	/// Proof: `CollatorSelection::DesiredCandidates` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_desired_candidates() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_133_000 picoseconds.
		Weight::from_parts(4_354_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:100 w:100)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:100)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[0, 100]`.
	/// The range of component `k` is `[0, 100]`.
	fn set_candidacy_bond(c: u32, k: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + c * (180 ±0) + k * (112 ±0)`
		//  Estimated: `6287 + c * (901 ±29) + k * (901 ±29)`
		// Minimum execution time: 8_827_000 picoseconds.
		Weight::from_parts(9_194_000, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 176_244
			.saturating_add(Weight::from_parts(5_973_083, 0).saturating_mul(c.into()))
			// Standard Error: 176_244
			.saturating_add(Weight::from_parts(5_604_851, 0).saturating_mul(k.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(k.into())))
			.saturating_add(Weight::from_parts(0, 901).saturating_mul(c.into()))
			.saturating_add(Weight::from_parts(0, 901).saturating_mul(k.into()))
	}
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[2, 100]`.
	fn update_bond(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `221 + c * (49 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 26_598_000 picoseconds.
		Weight::from_parts(29_925_480, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 2_047
			.saturating_add(Weight::from_parts(143_639, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[1, 99]`.
	fn register_as_candidate(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `528 + c * (52 ±0)`
		//  Estimated: `6287 + c * (54 ±0)`
		// Minimum execution time: 33_828_000 picoseconds.
		Weight::from_parts(39_373_178, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 2_744
			.saturating_add(Weight::from_parts(172_026, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 54).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:2)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[2, 100]`.
	fn take_candidate_slot(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `657 + c * (53 ±0)`
		//  Estimated: `6287 + c * (55 ±0)`
		// Minimum execution time: 50_767_000 picoseconds.
		Weight::from_parts(56_932_944, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 3_561
			.saturating_add(Weight::from_parts(188_764, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 55).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[2, 100]`.
	fn leave_intent(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `202 + c * (48 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 28_366_000 picoseconds.
		Weight::from_parts(32_664_626, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 2_706
			.saturating_add(Weight::from_parts(157_539, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	fn note_author() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `52`
		//  Estimated: `6196`
		// Minimum execution time: 40_636_000 picoseconds.
		Weight::from_parts(41_340_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:100 w:0)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::DesiredCandidates` (r:1 w:0)
	/// Proof: `CollatorSelection::DesiredCandidates` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:97 w:97)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 100]`.
	/// The range of component `c` is `[1, 100]`.
	fn new_session(r: u32, c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2045 + c * (97 ±0) + r * (112 ±0)`
		//  Estimated: `6287 + c * (2519 ±0) + r * (2603 ±0)`
		// Minimum execution time: 15_976_000 picoseconds.
		Weight::from_parts(16_203_000, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 311_393
			.saturating_add(Weight::from_parts(14_019_707, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(c.into())))
			.saturating_add(Weight::from_parts(0, 2519).saturating_mul(c.into()))
			.saturating_add(Weight::from_parts(0, 2603).saturating_mul(r.into()))
	}
}
