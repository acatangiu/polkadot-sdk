// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2023 Snowfork <hello@snowfork.com>
//! Converts messages from Ethereum to XCM messages

use codec::{Decode, DecodeLimit, Encode};
use core::marker::PhantomData;
use frame_support::PalletError;
use scale_info::TypeInfo;
use snowbridge_core::TokenId;
use sp_core::{Get, RuntimeDebug, H160, H256};
use sp_runtime::traits::MaybeEquivalence;
use sp_std::prelude::*;
use xcm::{
	prelude::{Junction::AccountKey20, *},
	MAX_XCM_DECODE_DEPTH,
};

const LOG_TARGET: &str = "snowbridge-router-primitives";

/// The ethereum side sends messages which are transcoded into XCM on BH. These messages are
/// self-contained, in that they can be transcoded using only information in the message.
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Message {
	/// The origin address
	pub origin: H160,
	/// The assets
	pub assets: Vec<Asset>,
	/// The command originating from the Gateway contract
	pub xcm: Vec<u8>,
	/// The claimer in the case that funds get trapped.
	pub claimer: Option<Vec<u8>>,
}

/// An asset that will be transacted on AH. The asset will be reserved/withdrawn and placed into
/// the holding register. The user needs to provide additional xcm to deposit the asset
/// in a beneficiary account.
#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
pub enum Asset {
	NativeTokenERC20 {
		/// The native token ID
		token_id: H160,
		/// The monetary value of the asset
		value: u128,
	},
	ForeignTokenERC20 {
		/// The foreign token ID
		token_id: H256,
		/// The monetary value of the asset
		value: u128,
	},
}

/// Reason why a message conversion failed.
#[derive(Copy, Clone, TypeInfo, PalletError, Encode, Decode, RuntimeDebug)]
pub enum ConvertMessageError {
	/// The XCM provided with the message could not be decoded into XCM.
	InvalidXCM,
	/// The XCM provided with the message could not be decoded into versioned XCM.
	InvalidVersionedXCM,
	/// Invalid claimer MultiAddress provided in payload.
	InvalidClaimer,
	/// Invalid foreign ERC20 token ID
	InvalidAsset,
}

pub trait ConvertMessage {
	fn convert(message: Message, origin_account: Location) -> Result<Xcm<()>, ConvertMessageError>;
}

pub struct MessageToXcm<EthereumNetwork, InboundQueuePalletInstance, ConvertAssetId, XcmPrologueFee>
where
	EthereumNetwork: Get<NetworkId>,
	InboundQueuePalletInstance: Get<u8>,
	ConvertAssetId: MaybeEquivalence<TokenId, Location>,
	XcmPrologueFee: Get<u128>,
{
	_phantom:
		PhantomData<(EthereumNetwork, InboundQueuePalletInstance, ConvertAssetId, XcmPrologueFee)>,
}

impl<EthereumNetwork, InboundQueuePalletInstance, ConvertAssetId, XcmPrologueFee> ConvertMessage
	for MessageToXcm<EthereumNetwork, InboundQueuePalletInstance, ConvertAssetId, XcmPrologueFee>
where
	EthereumNetwork: Get<NetworkId>,
	InboundQueuePalletInstance: Get<u8>,
	ConvertAssetId: MaybeEquivalence<TokenId, Location>,
	XcmPrologueFee: Get<u128>,
{
	fn convert(
		message: Message,
		origin_account_location: Location,
	) -> Result<Xcm<()>, ConvertMessageError> {
		let mut message_xcm: Xcm<()> = Xcm::new();
		if message.xcm.len() > 0 {
			// Decode xcm
			let versioned_xcm = VersionedXcm::<()>::decode_with_depth_limit(
				MAX_XCM_DECODE_DEPTH,
				&mut message.xcm.as_ref(),
			)
			.map_err(|_| ConvertMessageError::InvalidVersionedXCM)?;
			message_xcm = versioned_xcm.try_into().map_err(|_| ConvertMessageError::InvalidXCM)?;
		}

		log::debug!(target: LOG_TARGET,"xcm decoded as {:?}", message_xcm);

		let network = EthereumNetwork::get();

		let origin_location = Location::new(
			2,
			[GlobalConsensus(network), AccountKey20 { key: message.origin.into(), network: None }],
		);

		let network = EthereumNetwork::get();

		let fee_asset = Location::new(1, Here);
		let fee: xcm::prelude::Asset = (fee_asset.clone(), XcmPrologueFee::get()).into();
		let mut instructions = vec![
			ReceiveTeleportedAsset(fee.clone().into()),
			PayFees { asset: fee },
			DescendOrigin(PalletInstance(InboundQueuePalletInstance::get()).into()),
			UniversalOrigin(GlobalConsensus(network)),
		];

		for asset in &message.assets {
			match asset {
				Asset::NativeTokenERC20 { token_id, value } => {
					let token_location: Location = Location::new(
						2,
						[
							GlobalConsensus(EthereumNetwork::get()),
							AccountKey20 { network: None, key: (*token_id).into() },
						],
					);
					instructions.push(ReserveAssetDeposited((token_location, *value).into()));
				},
				Asset::ForeignTokenERC20 { token_id, value } => {
					let asset_id = ConvertAssetId::convert(&token_id)
						.ok_or(ConvertMessageError::InvalidAsset)?;
					instructions.push(WithdrawAsset((asset_id, *value).into()));
				},
			}
		}

		if let Some(claimer) = message.claimer {
			let claimer = Junction::decode(&mut claimer.as_ref())
				.map_err(|_| ConvertMessageError::InvalidClaimer)?;
			let claimer_location: Location = Location::new(0, [claimer.into()]);
			instructions.push(SetAssetClaimer { location: claimer_location });
		}

		// Set the alias origin to the original sender on Ethereum. Important to be before the
		// arbitrary XCM that is appended to the message on the next line.
		// instructions.push(AliasOrigin(origin_location.into()));

		// Add the XCM sent in the message to the end of the xcm instruction
		instructions.extend(message_xcm.0);

		let appendix = vec![
			RefundSurplus,
			// Refund excess fees to the relayer
			// TODO maybe refund all fees to the relayer instead of just DOT?
			DepositAsset {
				assets: Wild(AllOf { id: AssetId(fee_asset.into()), fun: WildFungible }),
				beneficiary: origin_account_location,
			},
		];

		instructions.extend(appendix);

		Ok(instructions.into())
	}
}

#[cfg(test)]
mod tests {
	use crate::inbound::v2::{ConvertMessage, Message, MessageToXcm};
	use codec::Decode;
	use frame_support::{assert_ok, parameter_types};
	use hex_literal::hex;
	use sp_core::H256;
	use sp_runtime::traits::{ConstU128, ConstU8};
	use xcm::prelude::*;

	use snowbridge_core::TokenId;
	use sp_runtime::traits::MaybeEquivalence;

	const NETWORK: NetworkId = Ethereum { chain_id: 11155111 };

	parameter_types! {
		pub EthereumNetwork: NetworkId = NETWORK;
	}

	pub struct MockTokenIdConvert;
	impl MaybeEquivalence<TokenId, Location> for MockTokenIdConvert {
		fn convert(_id: &TokenId) -> Option<Location> {
			Some(Location::parent())
		}
		fn convert_back(_loc: &Location) -> Option<TokenId> {
			None
		}
	}

	#[test]
	fn convert_message() {
		let payload = hex!("29e3b139f4393adda86303fcdaa35f60bb7092bf040197874824853fb4ad04794ccfd1cc8d2a7463839cfcbc6a315a1045c60ab85f400000b2d3595bf00600000000000000000000").to_vec();
		let origin_account =
			Location::new(0, AccountId32 { id: H256::random().into(), network: None });

		let message = Message::decode(&mut payload.as_ref());
		assert_ok!(message.clone());

		let result = MessageToXcm::<
			EthereumNetwork,
			ConstU8<80>,
			MockTokenIdConvert,
			ConstU128<1_000_000_000_000>,
		>::convert(message.unwrap(), origin_account);
		assert_ok!(result);
	}
}
