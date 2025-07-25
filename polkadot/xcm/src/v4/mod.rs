// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Version 4 of the Cross-Consensus Message format data structures.

pub use super::v3::GetWeight;
use super::{
	v3::{
		Instruction as OldInstruction, PalletInfo as OldPalletInfo,
		QueryResponseInfo as OldQueryResponseInfo, Response as OldResponse, Xcm as OldXcm,
	},
	v5::{
		Instruction as NewInstruction, PalletInfo as NewPalletInfo,
		QueryResponseInfo as NewQueryResponseInfo, Response as NewResponse, Xcm as NewXcm,
	},
};
use crate::{utils::decode_xcm_instructions, DoubleEncoded};
use alloc::{vec, vec::Vec};
use bounded_collections::{parameter_types, BoundedVec};
use codec::{
	self, Decode, DecodeWithMemTracking, Encode, Error as CodecError, Input as CodecInput,
	MaxEncodedLen,
};
use core::{fmt::Debug, result};
use derive_where::derive_where;
use frame_support::dispatch::GetDispatchInfo;
use scale_info::TypeInfo;

mod asset;
mod junction;
pub(crate) mod junctions;
mod location;
mod traits;

pub use asset::{
	Asset, AssetFilter, AssetId, AssetInstance, Assets, Fungibility, WildAsset, WildFungibility,
	MAX_ITEMS_IN_ASSETS,
};
pub use junction::{BodyId, BodyPart, Junction, NetworkId};
pub use junctions::Junctions;
pub use location::{Ancestor, AncestorThen, InteriorLocation, Location, Parent, ParentThen};
pub use traits::{
	send_xcm, validate_send, Error, ExecuteXcm, Outcome, PreparedMessage, Reanchorable, Result,
	SendError, SendResult, SendXcm, Weight, XcmHash,
};
// These parts of XCM v3 are unchanged in XCM v4, and are re-imported here.
pub use super::v3::{MaxDispatchErrorLen, MaybeErrorCode, OriginKind, WeightLimit};

/// This module's XCM version.
pub const VERSION: super::Version = 4;

/// An identifier for a query.
pub type QueryId = u64;

#[derive(Default, Encode, DecodeWithMemTracking, TypeInfo)]
#[derive_where(Clone, Eq, PartialEq, Debug)]
#[codec(encode_bound())]
#[codec(decode_bound())]
#[scale_info(bounds(), skip_type_params(Call))]
pub struct Xcm<Call>(pub Vec<Instruction<Call>>);

impl<Call> Decode for Xcm<Call> {
	fn decode<I: CodecInput>(input: &mut I) -> core::result::Result<Self, CodecError> {
		Ok(Xcm(decode_xcm_instructions(input)?))
	}
}

impl<Call> Xcm<Call> {
	/// Create an empty instance.
	pub fn new() -> Self {
		Self(vec![])
	}

	/// Return `true` if no instructions are held in `self`.
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Return the number of instructions held in `self`.
	pub fn len(&self) -> usize {
		self.0.len()
	}

	/// Return a reference to the inner value.
	pub fn inner(&self) -> &[Instruction<Call>] {
		&self.0
	}

	/// Return a mutable reference to the inner value.
	pub fn inner_mut(&mut self) -> &mut Vec<Instruction<Call>> {
		&mut self.0
	}

	/// Consume and return the inner value.
	pub fn into_inner(self) -> Vec<Instruction<Call>> {
		self.0
	}

	/// Return an iterator over references to the items.
	pub fn iter(&self) -> impl Iterator<Item = &Instruction<Call>> {
		self.0.iter()
	}

	/// Return an iterator over mutable references to the items.
	pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Instruction<Call>> {
		self.0.iter_mut()
	}

	/// Consume and return an iterator over the items.
	pub fn into_iter(self) -> impl Iterator<Item = Instruction<Call>> {
		self.0.into_iter()
	}

	/// Consume and either return `self` if it contains some instructions, or if it's empty, then
	/// instead return the result of `f`.
	pub fn or_else(self, f: impl FnOnce() -> Self) -> Self {
		if self.0.is_empty() {
			f()
		} else {
			self
		}
	}

	/// Return the first instruction, if any.
	pub fn first(&self) -> Option<&Instruction<Call>> {
		self.0.first()
	}

	/// Return the last instruction, if any.
	pub fn last(&self) -> Option<&Instruction<Call>> {
		self.0.last()
	}

	/// Return the only instruction, contained in `Self`, iff only one exists (`None` otherwise).
	pub fn only(&self) -> Option<&Instruction<Call>> {
		if self.0.len() == 1 {
			self.0.first()
		} else {
			None
		}
	}

	/// Return the only instruction, contained in `Self`, iff only one exists (returns `self`
	/// otherwise).
	pub fn into_only(mut self) -> core::result::Result<Instruction<Call>, Self> {
		if self.0.len() == 1 {
			self.0.pop().ok_or(self)
		} else {
			Err(self)
		}
	}
}

impl<Call> From<Vec<Instruction<Call>>> for Xcm<Call> {
	fn from(c: Vec<Instruction<Call>>) -> Self {
		Self(c)
	}
}

impl<Call> From<Xcm<Call>> for Vec<Instruction<Call>> {
	fn from(c: Xcm<Call>) -> Self {
		c.0
	}
}

/// A prelude for importing all types typically used when interacting with XCM messages.
pub mod prelude {
	mod contents {
		pub use super::super::{
			send_xcm, validate_send, Ancestor, AncestorThen, Asset,
			AssetFilter::{self, *},
			AssetId,
			AssetInstance::{self, *},
			Assets, BodyId, BodyPart, Error as XcmError, ExecuteXcm,
			Fungibility::{self, *},
			Instruction::*,
			InteriorLocation,
			Junction::{self, *},
			Junctions::{self, Here},
			Location, MaybeErrorCode,
			NetworkId::{self, *},
			OriginKind, Outcome, PalletInfo, Parent, ParentThen, PreparedMessage, QueryId,
			QueryResponseInfo, Reanchorable, Response, Result as XcmResult, SendError, SendResult,
			SendXcm, Weight,
			WeightLimit::{self, *},
			WildAsset::{self, *},
			WildFungibility::{self, Fungible as WildFungible, NonFungible as WildNonFungible},
			XcmContext, XcmHash, XcmWeightInfo, VERSION as XCM_VERSION,
		};
	}
	pub use super::{Instruction, Xcm};
	pub use contents::*;
	pub mod opaque {
		pub use super::{
			super::opaque::{Instruction, Xcm},
			contents::*,
		};
	}
}

parameter_types! {
	pub MaxPalletNameLen: u32 = 48;
	pub MaxPalletsInfo: u32 = 64;
}

#[derive(
	Clone, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen,
)]
pub struct PalletInfo {
	#[codec(compact)]
	pub index: u32,
	pub name: BoundedVec<u8, MaxPalletNameLen>,
	pub module_name: BoundedVec<u8, MaxPalletNameLen>,
	#[codec(compact)]
	pub major: u32,
	#[codec(compact)]
	pub minor: u32,
	#[codec(compact)]
	pub patch: u32,
}

impl TryInto<OldPalletInfo> for PalletInfo {
	type Error = ();

	fn try_into(self) -> result::Result<OldPalletInfo, Self::Error> {
		OldPalletInfo::new(
			self.index,
			self.name.into_inner(),
			self.module_name.into_inner(),
			self.major,
			self.minor,
			self.patch,
		)
		.map_err(|_| ())
	}
}

impl TryInto<NewPalletInfo> for PalletInfo {
	type Error = ();

	fn try_into(self) -> result::Result<NewPalletInfo, Self::Error> {
		NewPalletInfo::new(
			self.index,
			self.name.into_inner(),
			self.module_name.into_inner(),
			self.major,
			self.minor,
			self.patch,
		)
		.map_err(|_| ())
	}
}

impl PalletInfo {
	pub fn new(
		index: u32,
		name: Vec<u8>,
		module_name: Vec<u8>,
		major: u32,
		minor: u32,
		patch: u32,
	) -> result::Result<Self, Error> {
		let name = BoundedVec::try_from(name).map_err(|_| Error::Overflow)?;
		let module_name = BoundedVec::try_from(module_name).map_err(|_| Error::Overflow)?;

		Ok(Self { index, name, module_name, major, minor, patch })
	}
}

/// Response data to a query.
#[derive(
	Clone, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo, MaxEncodedLen,
)]
pub enum Response {
	/// No response. Serves as a neutral default.
	Null,
	/// Some assets.
	Assets(Assets),
	/// The outcome of an XCM instruction.
	ExecutionResult(Option<(u32, Error)>),
	/// An XCM version.
	Version(super::Version),
	/// The index, instance name, pallet name and version of some pallets.
	PalletsInfo(BoundedVec<PalletInfo, MaxPalletsInfo>),
	/// The status of a dispatch attempt using `Transact`.
	DispatchResult(MaybeErrorCode),
}

impl Default for Response {
	fn default() -> Self {
		Self::Null
	}
}

impl TryFrom<OldResponse> for Response {
	type Error = ();

	fn try_from(old: OldResponse) -> result::Result<Self, Self::Error> {
		use OldResponse::*;
		Ok(match old {
			Null => Self::Null,
			Assets(assets) => Self::Assets(assets.try_into()?),
			ExecutionResult(result) =>
				Self::ExecutionResult(result.map(|(num, old_error)| (num, old_error.into()))),
			Version(version) => Self::Version(version),
			PalletsInfo(pallet_info) => {
				let inner = pallet_info
					.into_iter()
					.map(TryInto::try_into)
					.collect::<result::Result<Vec<_>, _>>()?;
				Self::PalletsInfo(
					BoundedVec::<PalletInfo, MaxPalletsInfo>::try_from(inner).map_err(|_| ())?,
				)
			},
			DispatchResult(maybe_error) => Self::DispatchResult(maybe_error),
		})
	}
}

impl TryFrom<NewResponse> for Response {
	type Error = ();

	fn try_from(new: NewResponse) -> result::Result<Self, Self::Error> {
		use NewResponse::*;
		Ok(match new {
			Null => Self::Null,
			Assets(assets) => Self::Assets(assets.try_into()?),
			ExecutionResult(result) => Self::ExecutionResult(
				result
					.map(|(num, new_error)| (num, new_error.try_into()))
					.map(|(num, result)| result.map(|inner| (num, inner)))
					.transpose()?,
			),
			Version(version) => Self::Version(version),
			PalletsInfo(pallet_info) => {
				let inner = pallet_info
					.into_iter()
					.map(TryInto::try_into)
					.collect::<result::Result<Vec<_>, _>>()?;
				Self::PalletsInfo(
					BoundedVec::<PalletInfo, MaxPalletsInfo>::try_from(inner).map_err(|_| ())?,
				)
			},
			DispatchResult(maybe_error) =>
				Self::DispatchResult(maybe_error.try_into().map_err(|_| ())?),
		})
	}
}

/// Information regarding the composition of a query response.
#[derive(Clone, Eq, PartialEq, Encode, Decode, DecodeWithMemTracking, Debug, TypeInfo)]
pub struct QueryResponseInfo {
	/// The destination to which the query response message should be send.
	pub destination: Location,
	/// The `query_id` field of the `QueryResponse` message.
	#[codec(compact)]
	pub query_id: QueryId,
	/// The `max_weight` field of the `QueryResponse` message.
	pub max_weight: Weight,
}

impl TryFrom<NewQueryResponseInfo> for QueryResponseInfo {
	type Error = ();

	fn try_from(new: NewQueryResponseInfo) -> result::Result<Self, Self::Error> {
		Ok(Self {
			destination: new.destination.try_into()?,
			query_id: new.query_id,
			max_weight: new.max_weight,
		})
	}
}

impl TryFrom<OldQueryResponseInfo> for QueryResponseInfo {
	type Error = ();

	fn try_from(old: OldQueryResponseInfo) -> result::Result<Self, Self::Error> {
		Ok(Self {
			destination: old.destination.try_into()?,
			query_id: old.query_id,
			max_weight: old.max_weight,
		})
	}
}

/// Contextual data pertaining to a specific list of XCM instructions.
#[derive(Clone, Eq, PartialEq, Encode, Decode, Debug)]
pub struct XcmContext {
	/// The current value of the Origin register of the `XCVM`.
	pub origin: Option<Location>,
	/// The identity of the XCM; this may be a hash of its versioned encoding but could also be
	/// a high-level identity set by an appropriate barrier.
	pub message_id: XcmHash,
	/// The current value of the Topic register of the `XCVM`.
	pub topic: Option<[u8; 32]>,
}

impl XcmContext {
	/// Constructor which sets the message ID to the supplied parameter and leaves the origin and
	/// topic unset.
	pub fn with_message_id(message_id: XcmHash) -> XcmContext {
		XcmContext { origin: None, message_id, topic: None }
	}
}

/// Cross-Consensus Message: A message from one consensus system to another.
///
/// Consensus systems that may send and receive messages include blockchains and smart contracts.
///
/// All messages are delivered from a known *origin*, expressed as a `Location`.
///
/// This is the inner XCM format and is version-sensitive. Messages are typically passed using the
/// outer XCM format, known as `VersionedXcm`.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	xcm_procedural::XcmWeightInfoTrait,
	xcm_procedural::Builder,
)]
#[derive_where(Clone, Eq, PartialEq, Debug)]
#[codec(encode_bound())]
#[codec(decode_bound())]
#[codec(decode_with_mem_tracking_bound())]
#[scale_info(bounds(), skip_type_params(Call))]
pub enum Instruction<Call> {
	/// Withdraw asset(s) (`assets`) from the ownership of `origin` and place them into the Holding
	/// Register.
	///
	/// - `assets`: The asset(s) to be withdrawn into holding.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	#[builder(loads_holding)]
	WithdrawAsset(Assets),

	/// Asset(s) (`assets`) have been received into the ownership of this system on the `origin`
	/// system and equivalent derivatives should be placed into the Holding Register.
	///
	/// - `assets`: The asset(s) that are minted into holding.
	///
	/// Safety: `origin` must be trusted to have received and be storing `assets` such that they
	/// may later be withdrawn should this system send a corresponding message.
	///
	/// Kind: *Trusted Indication*.
	///
	/// Errors:
	#[builder(loads_holding)]
	ReserveAssetDeposited(Assets),

	/// Asset(s) (`assets`) have been destroyed on the `origin` system and equivalent assets should
	/// be created and placed into the Holding Register.
	///
	/// - `assets`: The asset(s) that are minted into the Holding Register.
	///
	/// Safety: `origin` must be trusted to have irrevocably destroyed the corresponding `assets`
	/// prior as a consequence of sending this message.
	///
	/// Kind: *Trusted Indication*.
	///
	/// Errors:
	#[builder(loads_holding)]
	ReceiveTeleportedAsset(Assets),

	/// Respond with information that the local system is expecting.
	///
	/// - `query_id`: The identifier of the query that resulted in this message being sent.
	/// - `response`: The message content.
	/// - `max_weight`: The maximum weight that handling this response should take.
	/// - `querier`: The location responsible for the initiation of the response, if there is one.
	///   In general this will tend to be the same location as the receiver of this message. NOTE:
	///   As usual, this is interpreted from the perspective of the receiving consensus system.
	///
	/// Safety: Since this is information only, there are no immediate concerns. However, it should
	/// be remembered that even if the Origin behaves reasonably, it can always be asked to make
	/// a response to a third-party chain who may or may not be expecting the response. Therefore
	/// the `querier` should be checked to match the expected value.
	///
	/// Kind: *Information*.
	///
	/// Errors:
	QueryResponse {
		#[codec(compact)]
		query_id: QueryId,
		response: Response,
		max_weight: Weight,
		querier: Option<Location>,
	},

	/// Withdraw asset(s) (`assets`) from the ownership of `origin` and place equivalent assets
	/// under the ownership of `beneficiary`.
	///
	/// - `assets`: The asset(s) to be withdrawn.
	/// - `beneficiary`: The new owner for the assets.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	TransferAsset { assets: Assets, beneficiary: Location },

	/// Withdraw asset(s) (`assets`) from the ownership of `origin` and place equivalent assets
	/// under the ownership of `dest` within this consensus system (i.e. its sovereign account).
	///
	/// Send an onward XCM message to `dest` of `ReserveAssetDeposited` with the given
	/// `xcm`.
	///
	/// - `assets`: The asset(s) to be withdrawn.
	/// - `dest`: The location whose sovereign account will own the assets and thus the effective
	///   beneficiary for the assets and the notification target for the reserve asset deposit
	///   message.
	/// - `xcm`: The instructions that should follow the `ReserveAssetDeposited` instruction, which
	///   is sent onwards to `dest`.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	TransferReserveAsset { assets: Assets, dest: Location, xcm: Xcm<()> },

	/// Apply the encoded transaction `call`, whose dispatch-origin should be `origin` as expressed
	/// by the kind of origin `origin_kind`.
	///
	/// The Transact Status Register is set according to the result of dispatching the call.
	///
	/// - `origin_kind`: The means of expressing the message origin as a dispatch origin.
	/// - `require_weight_at_most`: The weight of `call`; this should be at least the chain's
	///   calculated weight and will be used in the weight determination arithmetic.
	/// - `call`: The encoded transaction to be applied.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	Transact { origin_kind: OriginKind, require_weight_at_most: Weight, call: DoubleEncoded<Call> },

	/// A message to notify about a new incoming HRMP channel. This message is meant to be sent by
	/// the relay-chain to a para.
	///
	/// - `sender`: The sender in the to-be opened channel. Also, the initiator of the channel
	///   opening.
	/// - `max_message_size`: The maximum size of a message proposed by the sender.
	/// - `max_capacity`: The maximum number of messages that can be queued in the channel.
	///
	/// Safety: The message should originate directly from the relay-chain.
	///
	/// Kind: *System Notification*
	HrmpNewChannelOpenRequest {
		#[codec(compact)]
		sender: u32,
		#[codec(compact)]
		max_message_size: u32,
		#[codec(compact)]
		max_capacity: u32,
	},

	/// A message to notify about that a previously sent open channel request has been accepted by
	/// the recipient. That means that the channel will be opened during the next relay-chain
	/// session change. This message is meant to be sent by the relay-chain to a para.
	///
	/// Safety: The message should originate directly from the relay-chain.
	///
	/// Kind: *System Notification*
	///
	/// Errors:
	HrmpChannelAccepted {
		// NOTE: We keep this as a structured item to a) keep it consistent with the other Hrmp
		// items; and b) because the field's meaning is not obvious/mentioned from the item name.
		#[codec(compact)]
		recipient: u32,
	},

	/// A message to notify that the other party in an open channel decided to close it. In
	/// particular, `initiator` is going to close the channel opened from `sender` to the
	/// `recipient`. The close will be enacted at the next relay-chain session change. This message
	/// is meant to be sent by the relay-chain to a para.
	///
	/// Safety: The message should originate directly from the relay-chain.
	///
	/// Kind: *System Notification*
	///
	/// Errors:
	HrmpChannelClosing {
		#[codec(compact)]
		initiator: u32,
		#[codec(compact)]
		sender: u32,
		#[codec(compact)]
		recipient: u32,
	},

	/// Clear the origin.
	///
	/// This may be used by the XCM author to ensure that later instructions cannot command the
	/// authority of the origin (e.g. if they are being relayed from an untrusted source, as often
	/// the case with `ReserveAssetDeposited`).
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	ClearOrigin,

	/// Mutate the origin to some interior location.
	///
	/// Kind: *Command*
	///
	/// Errors:
	DescendOrigin(InteriorLocation),

	/// Immediately report the contents of the Error Register to the given destination via XCM.
	///
	/// A `QueryResponse` message of type `ExecutionOutcome` is sent to the described destination.
	///
	/// - `response_info`: Information for making the response.
	///
	/// Kind: *Command*
	///
	/// Errors:
	ReportError(QueryResponseInfo),

	/// Remove the asset(s) (`assets`) from the Holding Register and place equivalent assets under
	/// the ownership of `beneficiary` within this consensus system.
	///
	/// - `assets`: The asset(s) to remove from holding.
	/// - `beneficiary`: The new owner for the assets.
	///
	/// Kind: *Command*
	///
	/// Errors:
	DepositAsset { assets: AssetFilter, beneficiary: Location },

	/// Remove the asset(s) (`assets`) from the Holding Register and place equivalent assets under
	/// the ownership of `dest` within this consensus system (i.e. deposit them into its sovereign
	/// account).
	///
	/// Send an onward XCM message to `dest` of `ReserveAssetDeposited` with the given `effects`.
	///
	/// - `assets`: The asset(s) to remove from holding.
	/// - `dest`: The location whose sovereign account will own the assets and thus the effective
	///   beneficiary for the assets and the notification target for the reserve asset deposit
	///   message.
	/// - `xcm`: The orders that should follow the `ReserveAssetDeposited` instruction which is
	///   sent onwards to `dest`.
	///
	/// Kind: *Command*
	///
	/// Errors:
	DepositReserveAsset { assets: AssetFilter, dest: Location, xcm: Xcm<()> },

	/// Remove the asset(s) (`want`) from the Holding Register and replace them with alternative
	/// assets.
	///
	/// The minimum amount of assets to be received into the Holding Register for the order not to
	/// fail may be stated.
	///
	/// - `give`: The maximum amount of assets to remove from holding.
	/// - `want`: The minimum amount of assets which `give` should be exchanged for.
	/// - `maximal`: If `true`, then prefer to give as much as possible up to the limit of `give`
	///   and receive accordingly more. If `false`, then prefer to give as little as possible in
	///   order to receive as little as possible while receiving at least `want`.
	///
	/// Kind: *Command*
	///
	/// Errors:
	ExchangeAsset { give: AssetFilter, want: Assets, maximal: bool },

	/// Remove the asset(s) (`assets`) from holding and send a `WithdrawAsset` XCM message to a
	/// reserve location.
	///
	/// - `assets`: The asset(s) to remove from holding.
	/// - `reserve`: A valid location that acts as a reserve for all asset(s) in `assets`. The
	///   sovereign account of this consensus system *on the reserve location* will have
	///   appropriate assets withdrawn and `effects` will be executed on them. There will typically
	///   be only one valid location on any given asset/chain combination.
	/// - `xcm`: The instructions to execute on the assets once withdrawn *on the reserve
	///   location*.
	///
	/// Kind: *Command*
	///
	/// Errors:
	InitiateReserveWithdraw { assets: AssetFilter, reserve: Location, xcm: Xcm<()> },

	/// Remove the asset(s) (`assets`) from holding and send a `ReceiveTeleportedAsset` XCM message
	/// to a `dest` location.
	///
	/// - `assets`: The asset(s) to remove from holding.
	/// - `dest`: A valid location that respects teleports coming from this location.
	/// - `xcm`: The instructions to execute on the assets once arrived *on the destination
	///   location*.
	///
	/// NOTE: The `dest` location *MUST* respect this origin as a valid teleportation origin for
	/// all `assets`. If it does not, then the assets may be lost.
	///
	/// Kind: *Command*
	///
	/// Errors:
	InitiateTeleport { assets: AssetFilter, dest: Location, xcm: Xcm<()> },

	/// Report to a given destination the contents of the Holding Register.
	///
	/// A `QueryResponse` message of type `Assets` is sent to the described destination.
	///
	/// - `response_info`: Information for making the response.
	/// - `assets`: A filter for the assets that should be reported back. The assets reported back
	///   will be, asset-wise, *the lesser of this value and the holding register*. No wildcards
	///   will be used when reporting assets back.
	///
	/// Kind: *Command*
	///
	/// Errors:
	ReportHolding { response_info: QueryResponseInfo, assets: AssetFilter },

	/// Pay for the execution of some XCM `xcm` and `orders` with up to `weight`
	/// picoseconds of execution time, paying for this with up to `fees` from the Holding Register.
	///
	/// - `fees`: The asset(s) to remove from the Holding Register to pay for fees.
	/// - `weight_limit`: The maximum amount of weight to purchase; this must be at least the
	///   expected maximum weight of the total XCM to be executed for the
	///   `AllowTopLevelPaidExecutionFrom` barrier to allow the XCM be executed.
	///
	/// Kind: *Command*
	///
	/// Errors:
	#[builder(pays_fees)]
	BuyExecution { fees: Asset, weight_limit: WeightLimit },

	/// Refund any surplus weight previously bought with `BuyExecution`.
	///
	/// Kind: *Command*
	///
	/// Errors: None.
	RefundSurplus,

	/// Set the Error Handler Register. This is code that should be called in the case of an error
	/// happening.
	///
	/// An error occurring within execution of this code will _NOT_ result in the error register
	/// being set, nor will an error handler be called due to it. The error handler and appendix
	/// may each still be set.
	///
	/// The apparent weight of this instruction is inclusive of the inner `Xcm`; the executing
	/// weight however includes only the difference between the previous handler and the new
	/// handler, which can reasonably be negative, which would result in a surplus.
	///
	/// Kind: *Command*
	///
	/// Errors: None.
	SetErrorHandler(Xcm<Call>),

	/// Set the Appendix Register. This is code that should be called after code execution
	/// (including the error handler if any) is finished. This will be called regardless of whether
	/// an error occurred.
	///
	/// Any error occurring due to execution of this code will result in the error register being
	/// set, and the error handler (if set) firing.
	///
	/// The apparent weight of this instruction is inclusive of the inner `Xcm`; the executing
	/// weight however includes only the difference between the previous appendix and the new
	/// appendix, which can reasonably be negative, which would result in a surplus.
	///
	/// Kind: *Command*
	///
	/// Errors: None.
	SetAppendix(Xcm<Call>),

	/// Clear the Error Register.
	///
	/// Kind: *Command*
	///
	/// Errors: None.
	ClearError,

	/// Create some assets which are being held on behalf of the origin.
	///
	/// - `assets`: The assets which are to be claimed. This must match exactly with the assets
	///   claimable by the origin of the ticket.
	/// - `ticket`: The ticket of the asset; this is an abstract identifier to help locate the
	///   asset.
	///
	/// Kind: *Command*
	///
	/// Errors:
	#[builder(loads_holding)]
	ClaimAsset { assets: Assets, ticket: Location },

	/// Always throws an error of type `Trap`.
	///
	/// Kind: *Command*
	///
	/// Errors:
	/// - `Trap`: All circumstances, whose inner value is the same as this item's inner value.
	Trap(#[codec(compact)] u64),

	/// Ask the destination system to respond with the most recent version of XCM that they
	/// support in a `QueryResponse` instruction. Any changes to this should also elicit similar
	/// responses when they happen.
	///
	/// - `query_id`: An identifier that will be replicated into the returned XCM message.
	/// - `max_response_weight`: The maximum amount of weight that the `QueryResponse` item which
	///   is sent as a reply may take to execute. NOTE: If this is unexpectedly large then the
	///   response may not execute at all.
	///
	/// Kind: *Command*
	///
	/// Errors: *Fallible*
	SubscribeVersion {
		#[codec(compact)]
		query_id: QueryId,
		max_response_weight: Weight,
	},

	/// Cancel the effect of a previous `SubscribeVersion` instruction.
	///
	/// Kind: *Command*
	///
	/// Errors: *Fallible*
	UnsubscribeVersion,

	/// Reduce Holding by up to the given assets.
	///
	/// Holding is reduced by as much as possible up to the assets in the parameter. It is not an
	/// error if the Holding does not contain the assets (to make this an error, use `ExpectAsset`
	/// prior).
	///
	/// Kind: *Command*
	///
	/// Errors: *Infallible*
	BurnAsset(Assets),

	/// Throw an error if Holding does not contain at least the given assets.
	///
	/// Kind: *Command*
	///
	/// Errors:
	/// - `ExpectationFalse`: If Holding Register does not contain the assets in the parameter.
	ExpectAsset(Assets),

	/// Ensure that the Origin Register equals some given value and throw an error if not.
	///
	/// Kind: *Command*
	///
	/// Errors:
	/// - `ExpectationFalse`: If Origin Register is not equal to the parameter.
	ExpectOrigin(Option<Location>),

	/// Ensure that the Error Register equals some given value and throw an error if not.
	///
	/// Kind: *Command*
	///
	/// Errors:
	/// - `ExpectationFalse`: If the value of the Error Register is not equal to the parameter.
	ExpectError(Option<(u32, Error)>),

	/// Ensure that the Transact Status Register equals some given value and throw an error if
	/// not.
	///
	/// Kind: *Command*
	///
	/// Errors:
	/// - `ExpectationFalse`: If the value of the Transact Status Register is not equal to the
	///   parameter.
	ExpectTransactStatus(MaybeErrorCode),

	/// Query the existence of a particular pallet type.
	///
	/// - `module_name`: The module name of the pallet to query.
	/// - `response_info`: Information for making the response.
	///
	/// Sends a `QueryResponse` to Origin whose data field `PalletsInfo` containing the information
	/// of all pallets on the local chain whose name is equal to `name`. This is empty in the case
	/// that the local chain is not based on Substrate Frame.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*
	///
	/// Errors: *Fallible*.
	QueryPallet { module_name: Vec<u8>, response_info: QueryResponseInfo },

	/// Ensure that a particular pallet with a particular version exists.
	///
	/// - `index: Compact`: The index which identifies the pallet. An error if no pallet exists at
	///   this index.
	/// - `name: Vec<u8>`: Name which must be equal to the name of the pallet.
	/// - `module_name: Vec<u8>`: Module name which must be equal to the name of the module in
	///   which the pallet exists.
	/// - `crate_major: Compact`: Version number which must be equal to the major version of the
	///   crate which implements the pallet.
	/// - `min_crate_minor: Compact`: Version number which must be at most the minor version of the
	///   crate which implements the pallet.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*
	///
	/// Errors:
	/// - `ExpectationFalse`: In case any of the expectations are broken.
	ExpectPallet {
		#[codec(compact)]
		index: u32,
		name: Vec<u8>,
		module_name: Vec<u8>,
		#[codec(compact)]
		crate_major: u32,
		#[codec(compact)]
		min_crate_minor: u32,
	},

	/// Send a `QueryResponse` message containing the value of the Transact Status Register to some
	/// destination.
	///
	/// - `query_response_info`: The information needed for constructing and sending the
	///   `QueryResponse` message.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*
	///
	/// Errors: *Fallible*.
	ReportTransactStatus(QueryResponseInfo),

	/// Set the Transact Status Register to its default, cleared, value.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*
	///
	/// Errors: *Infallible*.
	ClearTransactStatus,

	/// Set the Origin Register to be some child of the Universal Ancestor.
	///
	/// Safety: Should only be usable if the Origin is trusted to represent the Universal Ancestor
	/// child in general. In general, no Origin should be able to represent the Universal Ancestor
	/// child which is the root of the local consensus system since it would by extension
	/// allow it to act as any location within the local consensus.
	///
	/// The `Junction` parameter should generally be a `GlobalConsensus` variant since it is only
	/// these which are children of the Universal Ancestor.
	///
	/// Kind: *Command*
	///
	/// Errors: *Fallible*.
	UniversalOrigin(Junction),

	/// Send a message on to Non-Local Consensus system.
	///
	/// This will tend to utilize some extra-consensus mechanism, the obvious one being a bridge.
	/// A fee may be charged; this may be determined based on the contents of `xcm`. It will be
	/// taken from the Holding register.
	///
	/// - `network`: The remote consensus system to which the message should be exported.
	/// - `destination`: The location relative to the remote consensus system to which the message
	///   should be sent on arrival.
	/// - `xcm`: The message to be exported.
	///
	/// As an example, to export a message for execution on Statemine (parachain #1000 in the
	/// Kusama network), you would call with `network: NetworkId::Kusama` and
	/// `destination: [Parachain(1000)].into()`. Alternatively, to export a message for execution
	/// on Polkadot, you would call with `network: NetworkId:: Polkadot` and `destination: Here`.
	///
	/// Kind: *Command*
	///
	/// Errors: *Fallible*.
	ExportMessage { network: NetworkId, destination: InteriorLocation, xcm: Xcm<()> },

	/// Lock the locally held asset and prevent further transfer or withdrawal.
	///
	/// This restriction may be removed by the `UnlockAsset` instruction being called with an
	/// Origin of `unlocker` and a `target` equal to the current `Origin`.
	///
	/// If the locking is successful, then a `NoteUnlockable` instruction is sent to `unlocker`.
	///
	/// - `asset`: The asset(s) which should be locked.
	/// - `unlocker`: The value which the Origin must be for a corresponding `UnlockAsset`
	///   instruction to work.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	LockAsset { asset: Asset, unlocker: Location },

	/// Remove the lock over `asset` on this chain and (if nothing else is preventing it) allow the
	/// asset to be transferred.
	///
	/// - `asset`: The asset to be unlocked.
	/// - `target`: The owner of the asset on the local chain.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	UnlockAsset { asset: Asset, target: Location },

	/// Asset (`asset`) has been locked on the `origin` system and may not be transferred. It may
	/// only be unlocked with the receipt of the `UnlockAsset` instruction from this chain.
	///
	/// - `asset`: The asset(s) which are now unlockable from this origin.
	/// - `owner`: The owner of the asset on the chain in which it was locked. This may be a
	///   location specific to the origin network.
	///
	/// Safety: `origin` must be trusted to have locked the corresponding `asset`
	/// prior as a consequence of sending this message.
	///
	/// Kind: *Trusted Indication*.
	///
	/// Errors:
	NoteUnlockable { asset: Asset, owner: Location },

	/// Send an `UnlockAsset` instruction to the `locker` for the given `asset`.
	///
	/// This may fail if the local system is making use of the fact that the asset is locked or,
	/// of course, if there is no record that the asset actually is locked.
	///
	/// - `asset`: The asset(s) to be unlocked.
	/// - `locker`: The location from which a previous `NoteUnlockable` was sent and to which an
	///   `UnlockAsset` should be sent.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	RequestUnlock { asset: Asset, locker: Location },

	/// Sets the Fees Mode Register.
	///
	/// - `jit_withdraw`: The fees mode item; if set to `true` then fees for any instructions are
	///   withdrawn as needed using the same mechanism as `WithdrawAssets`.
	///
	/// Kind: *Command*.
	///
	/// Errors:
	SetFeesMode { jit_withdraw: bool },

	/// Set the Topic Register.
	///
	/// The 32-byte array identifier in the parameter is not guaranteed to be
	/// unique; if such a property is desired, it is up to the code author to
	/// enforce uniqueness.
	///
	/// Safety: No concerns.
	///
	/// Kind: *Command*
	///
	/// Errors:
	SetTopic([u8; 32]),

	/// Clear the Topic Register.
	///
	/// Kind: *Command*
	///
	/// Errors: None.
	ClearTopic,

	/// Alter the current Origin to another given origin.
	///
	/// Kind: *Command*
	///
	/// Errors: If the existing state would not allow such a change.
	AliasOrigin(Location),

	/// A directive to indicate that the origin expects free execution of the message.
	///
	/// At execution time, this instruction just does a check on the Origin register.
	/// However, at the barrier stage messages starting with this instruction can be disregarded if
	/// the origin is not acceptable for free execution or the `weight_limit` is `Limited` and
	/// insufficient.
	///
	/// Kind: *Indication*
	///
	/// Errors: If the given origin is `Some` and not equal to the current Origin register.
	UnpaidExecution { weight_limit: WeightLimit, check_origin: Option<Location> },
}

impl<Call> Xcm<Call> {
	pub fn into<C>(self) -> Xcm<C> {
		Xcm::from(self)
	}
	pub fn from<C>(xcm: Xcm<C>) -> Self {
		Self(xcm.0.into_iter().map(Instruction::<Call>::from).collect())
	}
}

impl<Call> Instruction<Call> {
	pub fn into<C>(self) -> Instruction<C> {
		Instruction::from(self)
	}
	pub fn from<C>(xcm: Instruction<C>) -> Self {
		use Instruction::*;
		match xcm {
			WithdrawAsset(assets) => WithdrawAsset(assets),
			ReserveAssetDeposited(assets) => ReserveAssetDeposited(assets),
			ReceiveTeleportedAsset(assets) => ReceiveTeleportedAsset(assets),
			QueryResponse { query_id, response, max_weight, querier } =>
				QueryResponse { query_id, response, max_weight, querier },
			TransferAsset { assets, beneficiary } => TransferAsset { assets, beneficiary },
			TransferReserveAsset { assets, dest, xcm } =>
				TransferReserveAsset { assets, dest, xcm },
			HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity } =>
				HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity },
			HrmpChannelAccepted { recipient } => HrmpChannelAccepted { recipient },
			HrmpChannelClosing { initiator, sender, recipient } =>
				HrmpChannelClosing { initiator, sender, recipient },
			Transact { origin_kind, require_weight_at_most, call } =>
				Transact { origin_kind, require_weight_at_most, call: call.into() },
			ReportError(response_info) => ReportError(response_info),
			DepositAsset { assets, beneficiary } => DepositAsset { assets, beneficiary },
			DepositReserveAsset { assets, dest, xcm } => DepositReserveAsset { assets, dest, xcm },
			ExchangeAsset { give, want, maximal } => ExchangeAsset { give, want, maximal },
			InitiateReserveWithdraw { assets, reserve, xcm } =>
				InitiateReserveWithdraw { assets, reserve, xcm },
			InitiateTeleport { assets, dest, xcm } => InitiateTeleport { assets, dest, xcm },
			ReportHolding { response_info, assets } => ReportHolding { response_info, assets },
			BuyExecution { fees, weight_limit } => BuyExecution { fees, weight_limit },
			ClearOrigin => ClearOrigin,
			DescendOrigin(who) => DescendOrigin(who),
			RefundSurplus => RefundSurplus,
			SetErrorHandler(xcm) => SetErrorHandler(xcm.into()),
			SetAppendix(xcm) => SetAppendix(xcm.into()),
			ClearError => ClearError,
			ClaimAsset { assets, ticket } => ClaimAsset { assets, ticket },
			Trap(code) => Trap(code),
			SubscribeVersion { query_id, max_response_weight } =>
				SubscribeVersion { query_id, max_response_weight },
			UnsubscribeVersion => UnsubscribeVersion,
			BurnAsset(assets) => BurnAsset(assets),
			ExpectAsset(assets) => ExpectAsset(assets),
			ExpectOrigin(origin) => ExpectOrigin(origin),
			ExpectError(error) => ExpectError(error),
			ExpectTransactStatus(transact_status) => ExpectTransactStatus(transact_status),
			QueryPallet { module_name, response_info } =>
				QueryPallet { module_name, response_info },
			ExpectPallet { index, name, module_name, crate_major, min_crate_minor } =>
				ExpectPallet { index, name, module_name, crate_major, min_crate_minor },
			ReportTransactStatus(response_info) => ReportTransactStatus(response_info),
			ClearTransactStatus => ClearTransactStatus,
			UniversalOrigin(j) => UniversalOrigin(j),
			ExportMessage { network, destination, xcm } =>
				ExportMessage { network, destination, xcm },
			LockAsset { asset, unlocker } => LockAsset { asset, unlocker },
			UnlockAsset { asset, target } => UnlockAsset { asset, target },
			NoteUnlockable { asset, owner } => NoteUnlockable { asset, owner },
			RequestUnlock { asset, locker } => RequestUnlock { asset, locker },
			SetFeesMode { jit_withdraw } => SetFeesMode { jit_withdraw },
			SetTopic(topic) => SetTopic(topic),
			ClearTopic => ClearTopic,
			AliasOrigin(location) => AliasOrigin(location),
			UnpaidExecution { weight_limit, check_origin } =>
				UnpaidExecution { weight_limit, check_origin },
		}
	}
}

// TODO: Automate Generation
impl<Call, W: XcmWeightInfo<Call>> GetWeight<W> for Instruction<Call> {
	fn weight(&self) -> Weight {
		use Instruction::*;
		match self {
			WithdrawAsset(assets) => W::withdraw_asset(assets),
			ReserveAssetDeposited(assets) => W::reserve_asset_deposited(assets),
			ReceiveTeleportedAsset(assets) => W::receive_teleported_asset(assets),
			QueryResponse { query_id, response, max_weight, querier } =>
				W::query_response(query_id, response, max_weight, querier),
			TransferAsset { assets, beneficiary } => W::transfer_asset(assets, beneficiary),
			TransferReserveAsset { assets, dest, xcm } =>
				W::transfer_reserve_asset(&assets, dest, xcm),
			Transact { origin_kind, require_weight_at_most, call } =>
				W::transact(origin_kind, require_weight_at_most, call),
			HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity } =>
				W::hrmp_new_channel_open_request(sender, max_message_size, max_capacity),
			HrmpChannelAccepted { recipient } => W::hrmp_channel_accepted(recipient),
			HrmpChannelClosing { initiator, sender, recipient } =>
				W::hrmp_channel_closing(initiator, sender, recipient),
			ClearOrigin => W::clear_origin(),
			DescendOrigin(who) => W::descend_origin(who),
			ReportError(response_info) => W::report_error(&response_info),
			DepositAsset { assets, beneficiary } => W::deposit_asset(assets, beneficiary),
			DepositReserveAsset { assets, dest, xcm } =>
				W::deposit_reserve_asset(assets, dest, xcm),
			ExchangeAsset { give, want, maximal } => W::exchange_asset(give, want, maximal),
			InitiateReserveWithdraw { assets, reserve, xcm } =>
				W::initiate_reserve_withdraw(assets, reserve, xcm),
			InitiateTeleport { assets, dest, xcm } => W::initiate_teleport(assets, dest, xcm),
			ReportHolding { response_info, assets } => W::report_holding(&response_info, &assets),
			BuyExecution { fees, weight_limit } => W::buy_execution(fees, weight_limit),
			RefundSurplus => W::refund_surplus(),
			SetErrorHandler(xcm) => W::set_error_handler(xcm),
			SetAppendix(xcm) => W::set_appendix(xcm),
			ClearError => W::clear_error(),
			ClaimAsset { assets, ticket } => W::claim_asset(assets, ticket),
			Trap(code) => W::trap(code),
			SubscribeVersion { query_id, max_response_weight } =>
				W::subscribe_version(query_id, max_response_weight),
			UnsubscribeVersion => W::unsubscribe_version(),
			BurnAsset(assets) => W::burn_asset(assets),
			ExpectAsset(assets) => W::expect_asset(assets),
			ExpectOrigin(origin) => W::expect_origin(origin),
			ExpectError(error) => W::expect_error(error),
			ExpectTransactStatus(transact_status) => W::expect_transact_status(transact_status),
			QueryPallet { module_name, response_info } =>
				W::query_pallet(module_name, response_info),
			ExpectPallet { index, name, module_name, crate_major, min_crate_minor } =>
				W::expect_pallet(index, name, module_name, crate_major, min_crate_minor),
			ReportTransactStatus(response_info) => W::report_transact_status(response_info),
			ClearTransactStatus => W::clear_transact_status(),
			UniversalOrigin(j) => W::universal_origin(j),
			ExportMessage { network, destination, xcm } =>
				W::export_message(network, destination, xcm),
			LockAsset { asset, unlocker } => W::lock_asset(asset, unlocker),
			UnlockAsset { asset, target } => W::unlock_asset(asset, target),
			NoteUnlockable { asset, owner } => W::note_unlockable(asset, owner),
			RequestUnlock { asset, locker } => W::request_unlock(asset, locker),
			SetFeesMode { jit_withdraw } => W::set_fees_mode(jit_withdraw),
			SetTopic(topic) => W::set_topic(topic),
			ClearTopic => W::clear_topic(),
			AliasOrigin(location) => W::alias_origin(location),
			UnpaidExecution { weight_limit, check_origin } =>
				W::unpaid_execution(weight_limit, check_origin),
		}
	}
}

pub mod opaque {
	/// The basic concrete type of `Xcm`, which doesn't make any assumptions about the
	/// format of a call other than it is pre-encoded.
	pub type Xcm = super::Xcm<()>;

	/// The basic concrete type of `Instruction`, which doesn't make any assumptions about the
	/// format of a call other than it is pre-encoded.
	pub type Instruction = super::Instruction<()>;
}

// Convert from a v3 XCM to a v4 XCM
impl<Call> TryFrom<OldXcm<Call>> for Xcm<Call> {
	type Error = ();
	fn try_from(old_xcm: OldXcm<Call>) -> result::Result<Self, Self::Error> {
		Ok(Xcm(old_xcm.0.into_iter().map(TryInto::try_into).collect::<result::Result<_, _>>()?))
	}
}

// Convert from a v5 XCM to a v4 XCM.
impl<Call: Decode + GetDispatchInfo> TryFrom<NewXcm<Call>> for Xcm<Call> {
	type Error = ();
	fn try_from(new_xcm: NewXcm<Call>) -> result::Result<Self, Self::Error> {
		Ok(Xcm(new_xcm.0.into_iter().map(TryInto::try_into).collect::<result::Result<_, _>>()?))
	}
}

// Convert from a v5 instruction to a v4 instruction.
impl<Call: Decode + GetDispatchInfo> TryFrom<NewInstruction<Call>> for Instruction<Call> {
	type Error = ();
	fn try_from(new_instruction: NewInstruction<Call>) -> result::Result<Self, Self::Error> {
		use NewInstruction::*;
		Ok(match new_instruction {
			WithdrawAsset(assets) => Self::WithdrawAsset(assets.try_into()?),
			ReserveAssetDeposited(assets) => Self::ReserveAssetDeposited(assets.try_into()?),
			ReceiveTeleportedAsset(assets) => Self::ReceiveTeleportedAsset(assets.try_into()?),
			QueryResponse { query_id, response, max_weight, querier: Some(querier) } =>
				Self::QueryResponse {
					query_id,
					querier: querier.try_into()?,
					response: response.try_into()?,
					max_weight,
				},
			QueryResponse { query_id, response, max_weight, querier: None } =>
				Self::QueryResponse {
					query_id,
					querier: None,
					response: response.try_into()?,
					max_weight,
				},
			TransferAsset { assets, beneficiary } => Self::TransferAsset {
				assets: assets.try_into()?,
				beneficiary: beneficiary.try_into()?,
			},
			TransferReserveAsset { assets, dest, xcm } => Self::TransferReserveAsset {
				assets: assets.try_into()?,
				dest: dest.try_into()?,
				xcm: xcm.try_into()?,
			},
			HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity } =>
				Self::HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity },
			HrmpChannelAccepted { recipient } => Self::HrmpChannelAccepted { recipient },
			HrmpChannelClosing { initiator, sender, recipient } =>
				Self::HrmpChannelClosing { initiator, sender, recipient },
			Transact { origin_kind, mut call, fallback_max_weight } => {
				// We first try to decode the call, if we can't, we use the fallback weight,
				// if there's no fallback, we just return `Weight::MAX`.
				let require_weight_at_most = match call.take_decoded() {
					Ok(decoded) => decoded.get_dispatch_info().call_weight,
					Err(error) => {
						let fallback_weight = fallback_max_weight.unwrap_or(Weight::MAX);
						tracing::debug!(
							target: "xcm::versions::v5Tov4",
							?error,
							?fallback_weight,
							"Couldn't decode call in Transact"
						);
						fallback_weight
					},
				};
				Self::Transact { origin_kind, require_weight_at_most, call: call.into() }
			},
			ReportError(response_info) => Self::ReportError(QueryResponseInfo {
				query_id: response_info.query_id,
				destination: response_info.destination.try_into().map_err(|_| ())?,
				max_weight: response_info.max_weight,
			}),
			DepositAsset { assets, beneficiary } => {
				let beneficiary = beneficiary.try_into()?;
				let assets = assets.try_into()?;
				Self::DepositAsset { assets, beneficiary }
			},
			DepositReserveAsset { assets, dest, xcm } => {
				let dest = dest.try_into()?;
				let xcm = xcm.try_into()?;
				let assets = assets.try_into()?;
				Self::DepositReserveAsset { assets, dest, xcm }
			},
			ExchangeAsset { give, want, maximal } => {
				let give = give.try_into()?;
				let want = want.try_into()?;
				Self::ExchangeAsset { give, want, maximal }
			},
			InitiateReserveWithdraw { assets, reserve, xcm } => {
				// No `max_assets` here, so if there's a connt, then we cannot translate.
				let assets = assets.try_into()?;
				let reserve = reserve.try_into()?;
				let xcm = xcm.try_into()?;
				Self::InitiateReserveWithdraw { assets, reserve, xcm }
			},
			InitiateTeleport { assets, dest, xcm } => {
				// No `max_assets` here, so if there's a connt, then we cannot translate.
				let assets = assets.try_into()?;
				let dest = dest.try_into()?;
				let xcm = xcm.try_into()?;
				Self::InitiateTeleport { assets, dest, xcm }
			},
			ReportHolding { response_info, assets } => {
				let response_info = QueryResponseInfo {
					destination: response_info.destination.try_into().map_err(|_| ())?,
					query_id: response_info.query_id,
					max_weight: response_info.max_weight,
				};
				Self::ReportHolding { response_info, assets: assets.try_into()? }
			},
			BuyExecution { fees, weight_limit } => {
				let fees = fees.try_into()?;
				let weight_limit = weight_limit.into();
				Self::BuyExecution { fees, weight_limit }
			},
			ClearOrigin => Self::ClearOrigin,
			DescendOrigin(who) => Self::DescendOrigin(who.try_into()?),
			RefundSurplus => Self::RefundSurplus,
			SetErrorHandler(xcm) => Self::SetErrorHandler(xcm.try_into()?),
			SetAppendix(xcm) => Self::SetAppendix(xcm.try_into()?),
			ClearError => Self::ClearError,
			ClaimAsset { assets, ticket } => {
				let assets = assets.try_into()?;
				let ticket = ticket.try_into()?;
				Self::ClaimAsset { assets, ticket }
			},
			Trap(code) => Self::Trap(code),
			SubscribeVersion { query_id, max_response_weight } =>
				Self::SubscribeVersion { query_id, max_response_weight },
			UnsubscribeVersion => Self::UnsubscribeVersion,
			BurnAsset(assets) => Self::BurnAsset(assets.try_into()?),
			ExpectAsset(assets) => Self::ExpectAsset(assets.try_into()?),
			ExpectOrigin(maybe_origin) =>
				Self::ExpectOrigin(maybe_origin.map(|origin| origin.try_into()).transpose()?),
			ExpectError(maybe_error) => Self::ExpectError(
				maybe_error
					.map(|(num, new_error)| (num, new_error.try_into()))
					.map(|(num, result)| result.map(|inner| (num, inner)))
					.transpose()?,
			),
			ExpectTransactStatus(maybe_error_code) => Self::ExpectTransactStatus(maybe_error_code),
			QueryPallet { module_name, response_info } =>
				Self::QueryPallet { module_name, response_info: response_info.try_into()? },
			ExpectPallet { index, name, module_name, crate_major, min_crate_minor } =>
				Self::ExpectPallet { index, name, module_name, crate_major, min_crate_minor },
			ReportTransactStatus(response_info) =>
				Self::ReportTransactStatus(response_info.try_into()?),
			ClearTransactStatus => Self::ClearTransactStatus,
			UniversalOrigin(junction) => Self::UniversalOrigin(junction.try_into()?),
			ExportMessage { network, destination, xcm } => Self::ExportMessage {
				network: network.into(),
				destination: destination.try_into()?,
				xcm: xcm.try_into()?,
			},
			LockAsset { asset, unlocker } =>
				Self::LockAsset { asset: asset.try_into()?, unlocker: unlocker.try_into()? },
			UnlockAsset { asset, target } =>
				Self::UnlockAsset { asset: asset.try_into()?, target: target.try_into()? },
			NoteUnlockable { asset, owner } =>
				Self::NoteUnlockable { asset: asset.try_into()?, owner: owner.try_into()? },
			RequestUnlock { asset, locker } =>
				Self::RequestUnlock { asset: asset.try_into()?, locker: locker.try_into()? },
			SetFeesMode { jit_withdraw } => Self::SetFeesMode { jit_withdraw },
			SetTopic(topic) => Self::SetTopic(topic),
			ClearTopic => Self::ClearTopic,
			AliasOrigin(location) => Self::AliasOrigin(location.try_into()?),
			UnpaidExecution { weight_limit, check_origin } => Self::UnpaidExecution {
				weight_limit,
				check_origin: check_origin.map(|origin| origin.try_into()).transpose()?,
			},
			InitiateTransfer { .. } |
			PayFees { .. } |
			SetHints { .. } |
			ExecuteWithOrigin { .. } => {
				tracing::debug!(target: "xcm::versions::v5tov4", ?new_instruction, "not supported by v4");
				return Err(());
			},
		})
	}
}

// Convert from a v3 instruction to a v4 instruction
impl<Call> TryFrom<OldInstruction<Call>> for Instruction<Call> {
	type Error = ();
	fn try_from(old_instruction: OldInstruction<Call>) -> result::Result<Self, Self::Error> {
		use OldInstruction::*;
		Ok(match old_instruction {
			WithdrawAsset(assets) => Self::WithdrawAsset(assets.try_into()?),
			ReserveAssetDeposited(assets) => Self::ReserveAssetDeposited(assets.try_into()?),
			ReceiveTeleportedAsset(assets) => Self::ReceiveTeleportedAsset(assets.try_into()?),
			QueryResponse { query_id, response, max_weight, querier: Some(querier) } =>
				Self::QueryResponse {
					query_id,
					querier: querier.try_into()?,
					response: response.try_into()?,
					max_weight,
				},
			QueryResponse { query_id, response, max_weight, querier: None } =>
				Self::QueryResponse {
					query_id,
					querier: None,
					response: response.try_into()?,
					max_weight,
				},
			TransferAsset { assets, beneficiary } => Self::TransferAsset {
				assets: assets.try_into()?,
				beneficiary: beneficiary.try_into()?,
			},
			TransferReserveAsset { assets, dest, xcm } => Self::TransferReserveAsset {
				assets: assets.try_into()?,
				dest: dest.try_into()?,
				xcm: xcm.try_into()?,
			},
			HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity } =>
				Self::HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity },
			HrmpChannelAccepted { recipient } => Self::HrmpChannelAccepted { recipient },
			HrmpChannelClosing { initiator, sender, recipient } =>
				Self::HrmpChannelClosing { initiator, sender, recipient },
			Transact { origin_kind, require_weight_at_most, call } =>
				Self::Transact { origin_kind, require_weight_at_most, call: call.into() },
			ReportError(response_info) => Self::ReportError(QueryResponseInfo {
				query_id: response_info.query_id,
				destination: response_info.destination.try_into().map_err(|_| ())?,
				max_weight: response_info.max_weight,
			}),
			DepositAsset { assets, beneficiary } => {
				let beneficiary = beneficiary.try_into()?;
				let assets = assets.try_into()?;
				Self::DepositAsset { assets, beneficiary }
			},
			DepositReserveAsset { assets, dest, xcm } => {
				let dest = dest.try_into()?;
				let xcm = xcm.try_into()?;
				let assets = assets.try_into()?;
				Self::DepositReserveAsset { assets, dest, xcm }
			},
			ExchangeAsset { give, want, maximal } => {
				let give = give.try_into()?;
				let want = want.try_into()?;
				Self::ExchangeAsset { give, want, maximal }
			},
			InitiateReserveWithdraw { assets, reserve, xcm } => {
				let assets = assets.try_into()?;
				let reserve = reserve.try_into()?;
				let xcm = xcm.try_into()?;
				Self::InitiateReserveWithdraw { assets, reserve, xcm }
			},
			InitiateTeleport { assets, dest, xcm } => {
				let assets = assets.try_into()?;
				let dest = dest.try_into()?;
				let xcm = xcm.try_into()?;
				Self::InitiateTeleport { assets, dest, xcm }
			},
			ReportHolding { response_info, assets } => {
				let response_info = QueryResponseInfo {
					destination: response_info.destination.try_into().map_err(|_| ())?,
					query_id: response_info.query_id,
					max_weight: response_info.max_weight,
				};
				Self::ReportHolding { response_info, assets: assets.try_into()? }
			},
			BuyExecution { fees, weight_limit } => {
				let fees = fees.try_into()?;
				let weight_limit = weight_limit.into();
				Self::BuyExecution { fees, weight_limit }
			},
			ClearOrigin => Self::ClearOrigin,
			DescendOrigin(who) => Self::DescendOrigin(who.try_into()?),
			RefundSurplus => Self::RefundSurplus,
			SetErrorHandler(xcm) => Self::SetErrorHandler(xcm.try_into()?),
			SetAppendix(xcm) => Self::SetAppendix(xcm.try_into()?),
			ClearError => Self::ClearError,
			ClaimAsset { assets, ticket } => {
				let assets = assets.try_into()?;
				let ticket = ticket.try_into()?;
				Self::ClaimAsset { assets, ticket }
			},
			Trap(code) => Self::Trap(code),
			SubscribeVersion { query_id, max_response_weight } =>
				Self::SubscribeVersion { query_id, max_response_weight },
			UnsubscribeVersion => Self::UnsubscribeVersion,
			BurnAsset(assets) => Self::BurnAsset(assets.try_into()?),
			ExpectAsset(assets) => Self::ExpectAsset(assets.try_into()?),
			ExpectOrigin(maybe_location) => Self::ExpectOrigin(
				maybe_location.map(|location| location.try_into()).transpose().map_err(|_| ())?,
			),
			ExpectError(maybe_error) => Self::ExpectError(
				maybe_error.map(|error| error.try_into()).transpose().map_err(|_| ())?,
			),
			ExpectTransactStatus(maybe_error_code) => Self::ExpectTransactStatus(maybe_error_code),
			QueryPallet { module_name, response_info } => Self::QueryPallet {
				module_name,
				response_info: response_info.try_into().map_err(|_| ())?,
			},
			ExpectPallet { index, name, module_name, crate_major, min_crate_minor } =>
				Self::ExpectPallet { index, name, module_name, crate_major, min_crate_minor },
			ReportTransactStatus(response_info) =>
				Self::ReportTransactStatus(response_info.try_into().map_err(|_| ())?),
			ClearTransactStatus => Self::ClearTransactStatus,
			UniversalOrigin(junction) =>
				Self::UniversalOrigin(junction.try_into().map_err(|_| ())?),
			ExportMessage { network, destination, xcm } => Self::ExportMessage {
				network: network.into(),
				destination: destination.try_into().map_err(|_| ())?,
				xcm: xcm.try_into().map_err(|_| ())?,
			},
			LockAsset { asset, unlocker } => Self::LockAsset {
				asset: asset.try_into().map_err(|_| ())?,
				unlocker: unlocker.try_into().map_err(|_| ())?,
			},
			UnlockAsset { asset, target } => Self::UnlockAsset {
				asset: asset.try_into().map_err(|_| ())?,
				target: target.try_into().map_err(|_| ())?,
			},
			NoteUnlockable { asset, owner } => Self::NoteUnlockable {
				asset: asset.try_into().map_err(|_| ())?,
				owner: owner.try_into().map_err(|_| ())?,
			},
			RequestUnlock { asset, locker } => Self::RequestUnlock {
				asset: asset.try_into().map_err(|_| ())?,
				locker: locker.try_into().map_err(|_| ())?,
			},
			SetFeesMode { jit_withdraw } => Self::SetFeesMode { jit_withdraw },
			SetTopic(topic) => Self::SetTopic(topic),
			ClearTopic => Self::ClearTopic,
			AliasOrigin(location) => Self::AliasOrigin(location.try_into().map_err(|_| ())?),
			UnpaidExecution { weight_limit, check_origin } => Self::UnpaidExecution {
				weight_limit,
				check_origin: check_origin
					.map(|location| location.try_into())
					.transpose()
					.map_err(|_| ())?,
			},
		})
	}
}

#[cfg(test)]
mod tests {
	use super::{prelude::*, *};
	use crate::{
		v3::{
			Junctions::Here as OldHere, MultiAssetFilter as OldMultiAssetFilter,
			WildMultiAsset as OldWildMultiAsset,
		},
		MAX_INSTRUCTIONS_TO_DECODE,
	};

	#[test]
	fn basic_roundtrip_works() {
		let xcm = Xcm::<()>(vec![TransferAsset {
			assets: (Here, 1u128).into(),
			beneficiary: Here.into(),
		}]);
		let old_xcm = OldXcm::<()>(vec![OldInstruction::TransferAsset {
			assets: (OldHere, 1u128).into(),
			beneficiary: OldHere.into(),
		}]);
		assert_eq!(old_xcm, OldXcm::<()>::try_from(xcm.clone()).unwrap());
		let new_xcm: Xcm<()> = old_xcm.try_into().unwrap();
		assert_eq!(new_xcm, xcm);
	}

	#[test]
	fn teleport_roundtrip_works() {
		let xcm = Xcm::<()>(vec![
			ReceiveTeleportedAsset((Here, 1u128).into()),
			ClearOrigin,
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary: Here.into() },
		]);
		let old_xcm: OldXcm<()> = OldXcm::<()>(vec![
			OldInstruction::ReceiveTeleportedAsset((OldHere, 1u128).into()),
			OldInstruction::ClearOrigin,
			OldInstruction::DepositAsset {
				assets: crate::v3::MultiAssetFilter::Wild(crate::v3::WildMultiAsset::AllCounted(1)),
				beneficiary: OldHere.into(),
			},
		]);
		assert_eq!(old_xcm, OldXcm::<()>::try_from(xcm.clone()).unwrap());
		let new_xcm: Xcm<()> = old_xcm.try_into().unwrap();
		assert_eq!(new_xcm, xcm);
	}

	#[test]
	fn reserve_deposit_roundtrip_works() {
		let xcm = Xcm::<()>(vec![
			ReserveAssetDeposited((Here, 1u128).into()),
			ClearOrigin,
			BuyExecution {
				fees: (Here, 1u128).into(),
				weight_limit: Some(Weight::from_parts(1, 1)).into(),
			},
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary: Here.into() },
		]);
		let old_xcm = OldXcm::<()>(vec![
			OldInstruction::ReserveAssetDeposited((OldHere, 1u128).into()),
			OldInstruction::ClearOrigin,
			OldInstruction::BuyExecution {
				fees: (OldHere, 1u128).into(),
				weight_limit: WeightLimit::Limited(Weight::from_parts(1, 1)),
			},
			OldInstruction::DepositAsset {
				assets: crate::v3::MultiAssetFilter::Wild(crate::v3::WildMultiAsset::AllCounted(1)),
				beneficiary: OldHere.into(),
			},
		]);
		assert_eq!(old_xcm, OldXcm::<()>::try_from(xcm.clone()).unwrap());
		let new_xcm: Xcm<()> = old_xcm.try_into().unwrap();
		assert_eq!(new_xcm, xcm);
	}

	#[test]
	fn deposit_asset_roundtrip_works() {
		let xcm = Xcm::<()>(vec![
			WithdrawAsset((Here, 1u128).into()),
			DepositAsset { assets: Wild(AllCounted(1)), beneficiary: Here.into() },
		]);
		let old_xcm = OldXcm::<()>(vec![
			OldInstruction::WithdrawAsset((OldHere, 1u128).into()),
			OldInstruction::DepositAsset {
				assets: OldMultiAssetFilter::Wild(OldWildMultiAsset::AllCounted(1)),
				beneficiary: OldHere.into(),
			},
		]);
		assert_eq!(old_xcm, OldXcm::<()>::try_from(xcm.clone()).unwrap());
		let new_xcm: Xcm<()> = old_xcm.try_into().unwrap();
		assert_eq!(new_xcm, xcm);
	}

	#[test]
	fn deposit_reserve_asset_roundtrip_works() {
		let xcm = Xcm::<()>(vec![
			WithdrawAsset((Here, 1u128).into()),
			DepositReserveAsset {
				assets: Wild(AllCounted(1)),
				dest: Here.into(),
				xcm: Xcm::<()>(vec![]),
			},
		]);
		let old_xcm = OldXcm::<()>(vec![
			OldInstruction::WithdrawAsset((OldHere, 1u128).into()),
			OldInstruction::DepositReserveAsset {
				assets: OldMultiAssetFilter::Wild(OldWildMultiAsset::AllCounted(1)),
				dest: OldHere.into(),
				xcm: OldXcm::<()>(vec![]),
			},
		]);
		assert_eq!(old_xcm, OldXcm::<()>::try_from(xcm.clone()).unwrap());
		let new_xcm: Xcm<()> = old_xcm.try_into().unwrap();
		assert_eq!(new_xcm, xcm);
	}

	#[test]
	fn decoding_respects_limit() {
		let max_xcm = Xcm::<()>(vec![ClearOrigin; MAX_INSTRUCTIONS_TO_DECODE as usize]);
		let encoded = max_xcm.encode();
		assert!(Xcm::<()>::decode(&mut &encoded[..]).is_ok());

		let big_xcm = Xcm::<()>(vec![ClearOrigin; MAX_INSTRUCTIONS_TO_DECODE as usize + 1]);
		let encoded = big_xcm.encode();
		assert!(Xcm::<()>::decode(&mut &encoded[..]).is_err());

		let nested_xcm = Xcm::<()>(vec![
			DepositReserveAsset {
				assets: All.into(),
				dest: Here.into(),
				xcm: max_xcm,
			};
			(MAX_INSTRUCTIONS_TO_DECODE / 2) as usize
		]);
		let encoded = nested_xcm.encode();
		assert!(Xcm::<()>::decode(&mut &encoded[..]).is_err());

		let even_more_nested_xcm = Xcm::<()>(vec![SetAppendix(nested_xcm); 64]);
		let encoded = even_more_nested_xcm.encode();
		assert_eq!(encoded.len(), 342530);
		// This should not decode since the limit is 100
		assert_eq!(MAX_INSTRUCTIONS_TO_DECODE, 100, "precondition");
		assert!(Xcm::<()>::decode(&mut &encoded[..]).is_err());
	}
}
