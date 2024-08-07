// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

#![allow(missing_docs)]

use cumulus_primitives_core::ParaId;
use cumulus_test_runtime::{AccountId, Signature};
use parachains_common::AuraId;
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

type AccountPublic = <Signature as Verify>::Signer;

/// Helper function to generate an account ID from seed.
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Get the chain spec for a specific parachain ID.
/// The given accounts are initialized with funds in addition
/// to the default known accounts.
pub fn get_chain_spec_with_extra_endowed(
	id: Option<ParaId>,
	extra_endowed_accounts: Vec<AccountId>,
	code: &[u8],
) -> ChainSpec {
	ChainSpec::builder(
		code,
		Extensions { para_id: id.unwrap_or(cumulus_test_runtime::PARACHAIN_ID.into()).into() },
	)
	.with_name("Local Testnet")
	.with_id("local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(testnet_genesis_with_default_endowed(
		extra_endowed_accounts.clone(),
		id,
	))
	.build()
}

/// Get the chain spec for a specific parachain ID.
pub fn get_chain_spec(id: Option<ParaId>) -> ChainSpec {
	get_chain_spec_with_extra_endowed(
		id,
		Default::default(),
		cumulus_test_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	)
}

/// Get the chain spec for a specific parachain ID.
pub fn get_elastic_scaling_chain_spec(id: Option<ParaId>) -> ChainSpec {
	get_chain_spec_with_extra_endowed(
		id,
		Default::default(),
		cumulus_test_runtime::elastic_scaling::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
	)
}

/// Local testnet genesis for testing.
pub fn testnet_genesis_with_default_endowed(
	mut extra_endowed_accounts: Vec<AccountId>,
	self_para_id: Option<ParaId>,
) -> serde_json::Value {
	let mut endowed = vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
		get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
		get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
	];
	endowed.append(&mut extra_endowed_accounts);
	let invulnerables = vec![
		get_collator_keys_from_seed::<AuraId>("Alice"),
		get_collator_keys_from_seed::<AuraId>("Bob"),
		get_collator_keys_from_seed::<AuraId>("Charlie"),
		get_collator_keys_from_seed::<AuraId>("Dave"),
		get_collator_keys_from_seed::<AuraId>("Eve"),
		get_collator_keys_from_seed::<AuraId>("Ferdie"),
	];
	testnet_genesis(
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		invulnerables,
		endowed,
		self_para_id,
	)
}

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed<AuraId: Public>(seed: &str) -> <AuraId::Pair as Pair>::Public {
	get_from_seed::<AuraId>(seed)
}

/// Creates a local testnet genesis with endowed accounts.
pub fn testnet_genesis(
	root_key: AccountId,
	invulnerables: Vec<AuraId>,
	endowed_accounts: Vec<AccountId>,
	self_para_id: Option<ParaId>,
) -> serde_json::Value {
	let self_para_id = self_para_id.unwrap_or(cumulus_test_runtime::PARACHAIN_ID.into());
	serde_json::json!({
		"balances": cumulus_test_runtime::BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		"sudo": cumulus_test_runtime::SudoConfig { key: Some(root_key) },
		"parachainInfo": {
			"parachainId": self_para_id,
		},
		"aura": cumulus_test_runtime::AuraConfig { authorities: invulnerables }
	})
}
