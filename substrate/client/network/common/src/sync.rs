// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Abstract interfaces and data structures related to network sync.

pub mod message;

/// Sync operation mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SyncMode {
	/// Full block download and verification.
	Full,
	/// Download blocks and the latest state.
	LightState {
		/// Skip state proof download and verification.
		skip_proofs: bool,
		/// Download indexed transactions for recent blocks.
		storage_chain_mode: bool,
	},
	/// Warp sync - verify GRANDPA authority set transitions and the latest state.
	Warp,
	/// Warp sync using BEEFY finality proofs instead of GRANDPA justifications.
	BeefyWarp,
}

impl SyncMode {
	/// Returns `true` if `self` is a warp sync variant ([`Self::Warp`] or [`Self::BeefyWarp`]).
	pub fn is_warp(&self) -> bool {
		matches!(self, Self::Warp | Self::BeefyWarp)
	}

	/// Returns `true` if `self` is [`Self::LightState`].
	pub fn light_state(&self) -> bool {
		matches!(self, Self::LightState { .. })
	}
}

impl Default for SyncMode {
	fn default() -> Self {
		Self::Full
	}
}
