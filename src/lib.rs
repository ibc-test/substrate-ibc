#![cfg_attr(not(feature = "std"), no_std)]

//! # Overview
//!
//! The goal of this pallet is to allow the blockchains built on Substrate to gain the ability to
//! interact with other chains in a trustees way via IBC protocol
//!
//! The pallet implements the chain specific logic of [ICS spec](https://github.com/cosmos/ibc/tree/ee71d0640c23ec4e05e924f52f557b5e06c1d82f),  
//! and is integrated with [ibc-rs](https://github.com/informalsystems/ibc-rs),
//! which implements the generic cross-chain logic in [ICS spec](https://github.com/cosmos/ibc/tree/ee71d0640c23ec4e05e924f52f557b5e06c1d82f).
extern crate alloc;
extern crate core;

pub use pallet::*;

use alloc::{
	format,
	string::{String, ToString},
};
use codec::{Decode, Encode};
use core::{marker::PhantomData, str::FromStr};
use scale_info::{prelude::vec, TypeInfo};

use frame_support::{sp_std::fmt::Debug, traits::Currency};
use frame_system::ensure_signed;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

use ibc::{
	clients::ics10_grandpa::help::Commitment,
	core::ics24_host::identifier::ChannelId as IbcChannelId,
};
use tendermint_proto::Protobuf;

pub mod context;
pub mod events;
pub mod module;
pub mod traits;
pub mod utils;

use crate::{context::Context, traits::AssetIdAndNameProvider};

use crate::module::core::ics24_host::{
	ChannelId, ClientId, ClientType, ConnectionId, Height, Packet, PortId,
};

pub const LOG_TARGET: &str = "runtime::pallet-ibc";
pub const REVISION_NUMBER: u64 = 0;

type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

/// A struct corresponds to `Any` in crate "prost-types", used in ibc-rs.
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct Any {
	pub type_url: Vec<u8>,
	pub value: Vec<u8>,
}

impl From<ibc_proto::google::protobuf::Any> for Any {
	fn from(any: ibc_proto::google::protobuf::Any) -> Self {
		Self { type_url: any.type_url.as_bytes().to_vec(), value: any.value }
	}
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

mod type_define {
	use alloc::vec::Vec;

	pub type OctopusClientStatePath = Vec<u8>;
	pub type OctopusClientState = Vec<u8>;
	pub type OctopusClientId = Vec<u8>;
	pub type OctopusIbcHeight = Vec<u8>;
	pub type OctopusTimeStamp = Vec<u8>;
	pub type OctopusIbcHostHeight = Vec<u8>;
	pub type OctopusClientConsensusStatePath = Vec<u8>;
	pub type OctopusConsensusState = Vec<u8>;
	pub type OctopusConnectionsPath = Vec<u8>;
	pub type OctopusConnectionEnd = Vec<u8>;
	pub type OctopusChannelEndPath = Vec<u8>;
	pub type OctopusChannelEnd = Vec<u8>;
	pub type OctopusSeqSendsPath = Vec<u8>;
	pub type OctopusSeqRecvsPath = Vec<u8>;
	pub type OctopusSeqAcksPath = Vec<u8>;
	pub type OctopusAcksPath = Vec<u8>;
	pub type OctopusAcksHash = Vec<u8>;
	pub type OctopusClientTypePath = Vec<u8>;
	pub type OctopusClientType = Vec<u8>;
	pub type OctopusClientConnectionsPath = Vec<u8>;
	pub type OctopusConnectionId = Vec<u8>;
	pub type OctopusRecipientsPath = Vec<u8>;
	pub type OctopusRecipient = Vec<u8>;
	pub type OctopusCommitmentsPath = Vec<u8>;
	pub type OctopusCommitmentHash = Vec<u8>;
	pub type OctopusPortId = Vec<u8>;
	pub type OctopusChannelId = Vec<u8>;
	pub type OctopusSequence = u64;
	pub type OctopusWriteAckEvent = Vec<u8>;
	pub type PreviousHostHeight = u64;
	pub type AssetName = Vec<u8>;
}

#[frame_support::pallet]
pub mod pallet {
	use super::{type_define::*, *};
	use crate::module::{
		clients::ics10_grandpa::ClientState as EventClientState, core::ics24_host::Height,
	};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungibles::{Mutate, Transfer},
			tokens::{AssetId, Balance as AssetBalance},
			UnixTime,
		},
	};
	use frame_system::pallet_prelude::*;
	use ibc::{events::IbcEvent, signer::Signer};
	use sp_runtime::traits::IdentifyAccount;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + Sync + Send + Debug {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// The provider providing timestamp of host chain
		type TimeProvider: UnixTime;

		/// The currency type of the runtime
		type Currency: Currency<Self::AccountId>;

		/// Identifier for the class of asset.
		type AssetId: AssetId + MaybeSerializeDeserialize + Default;

		/// The units in which we record balances.
		type AssetBalance: AssetBalance + From<u128> + Into<u128>;

		/// Expose customizable associated type of asset transfer, lock and unlock
		type Fungibles: Transfer<Self::AccountId, AssetId = Self::AssetId, Balance = Self::AssetBalance>
			+ Mutate<Self::AccountId, AssetId = Self::AssetId, Balance = Self::AssetBalance>;

		/// Map of cross-chain asset ID & name
		type AssetIdByName: AssetIdAndNameProvider<Self::AssetId>;

		/// Account Id Conversion from SS58 string or hex string
		type AccountIdConversion: TryFrom<Signer>
			+ IdentifyAccount<AccountId = Self::AccountId>
			+ Clone
			+ PartialEq
			+ Debug;

		// The native token name
		const NATIVE_TOKEN_NAME: &'static [u8];
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	/// ClientStatePath(client_id) => ClientState
	pub type ClientStates<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusClientStatePath, OctopusClientState, ValueQuery>;

	#[pallet::storage]
	/// (client_id, height) => timestamp
	pub type ClientProcessedTimes<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		OctopusClientId,
		Blake2_128Concat,
		OctopusIbcHeight,
		OctopusTimeStamp,
		ValueQuery,
	>;


	#[pallet::storage]
	/// (client_id, height) => host_height
	pub type ClientUpdateHeight<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		OctopusClientId,
		Blake2_128Concat,
		OctopusIbcHeight,
		OctopusIbcHostHeight,
		ValueQuery,
	>;

	#[pallet::storage]
	/// ClientConsensusStatePath(client_id, Height) => ConsensusState
	pub type ConsensusStates<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		OctopusClientConsensusStatePath,
		OctopusConsensusState,
		ValueQuery,
	>;

	#[pallet::storage]
	/// ConnectionsPath(connection_id) => ConnectionEnd
	pub type Connections<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusConnectionsPath, OctopusConnectionEnd, ValueQuery>;

	#[pallet::storage]
	/// ChannelEndPath(port_id, channel_id) => ChannelEnd
	pub type Channels<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusChannelEndPath, OctopusChannelEnd, ValueQuery>;

	#[pallet::storage]
	/// ConnectionsPath(connection_id) => Vec<ChannelEndPath(port_id, channel_id)>
	pub type ChannelsConnection<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		OctopusConnectionsPath,
		Vec<OctopusChannelEndPath>,
		ValueQuery,
	>;

	#[pallet::storage]
	/// SeqSendsPath(port_id, channel_id) => sequence
	pub type NextSequenceSend<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusSeqSendsPath, OctopusSequence, ValueQuery>;

	#[pallet::storage]
	/// SeqRecvsPath(port_id, channel_id) => sequence
	pub type NextSequenceRecv<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusSeqRecvsPath, OctopusSequence, ValueQuery>;

	#[pallet::storage]
	/// SeqAcksPath(port_id, channel_id) => sequence
	pub type NextSequenceAck<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusSeqAcksPath, OctopusSequence, ValueQuery>;

	#[pallet::storage]
	/// AcksPath(port_id, channel_id, sequence) => hash of acknowledgement
	pub type Acknowledgements<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusAcksPath, OctopusAcksHash, ValueQuery>;

	#[pallet::storage]
	/// ClientTypePath(client_id) => client_type
	pub type Clients<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusClientTypePath, OctopusClientType, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn client_counter)]
	/// client counter
	pub type ClientCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn connection_counter)]
	/// connection counter
	pub type ConnectionCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	/// channel counter
	pub type ChannelCounter<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	/// ClientConnectionsPath(client_id) => connection_id
	pub type ConnectionClient<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		OctopusClientConnectionsPath,
		OctopusConnectionId,
		ValueQuery,
	>;

	#[pallet::storage]
	/// ReceiptsPath(port_id, channel_id, sequence) => receipt
	pub type PacketReceipt<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusRecipientsPath, OctopusRecipient, ValueQuery>;

	#[pallet::storage]
	/// CommitmentsPath(port_id, channel_id, sequence) => hash of (timestamp, height, packet)
	pub type PacketCommitment<T: Config> =
		StorageMap<_, Blake2_128Concat, OctopusCommitmentsPath, OctopusCommitmentHash, ValueQuery>;

	// TODO
	#[pallet::storage]
	/// (port_id, channel_id, sequence) => writ ack event
	pub type WriteAckPacketEvent<T: Config> = StorageNMap<
		_,
		(
			NMapKey<Blake2_128Concat, OctopusPortId>,
			NMapKey<Blake2_128Concat, OctopusChannelId>,
			NMapKey<Blake2_128Concat, OctopusSequence>,
		),
		OctopusWriteAckEvent,
		ValueQuery,
	>;

	#[pallet::storage]
	/// Previous host block height
	pub type OldHeight<T: Config> = StorageValue<_, PreviousHostHeight, ValueQuery>;

	#[pallet::storage]
	/// (asset name) => asset id
	pub type AssetIdByName<T: Config> =
		StorageMap<_, Twox64Concat, AssetName, T::AssetId, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub asset_id_by_name: Vec<(String, T::AssetId)>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { asset_id_by_name: Vec::new() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			for (token_id, id) in self.asset_id_by_name.iter() {
				<AssetIdByName<T>>::insert(token_id.as_bytes(), id);
			}
		}
	}
	/// Substrate IBC event list
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		IbcEvent {
			event: events::IbcEvent,
		},
		/// Emit update client state event
		UpdateClientState(Height, EventClientState),
		/// Transfer native token  event
		TransferNativeToken(T::AccountIdConversion, T::AccountIdConversion, BalanceOf<T>),
		/// Transfer non-native token event
		TransferNoNativeToken(
			T::AccountIdConversion,
			T::AccountIdConversion,
			<T as Config>::AssetBalance,
		),
		/// Burn cross chain token event
		BurnToken(T::AssetId, T::AccountIdConversion, T::AssetBalance),
		/// Mint chairperson token event
		MintToken(T::AssetId, T::AccountIdConversion, T::AssetBalance),
	}

	/// Errors in MMR verification informing users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Update the beefy light client failure!
		UpdateBeefyLightClientFailure,
		/// Receive mmr root block number less than client_state.latest_commitment.block_number
		ReceiveMmrRootBlockNumberLessThanClientStateLatestCommitmentBlockNumber,
		/// Client id not found
		ClientIdNotFound,
		/// Encode error
		InvalidEncode,
		/// Decode Error
		InvalidDecode,
		/// FromUtf8Error
		InvalidFromUtf8,
		/// Invalid signed_commitment
		InvalidSignedCommitment,
		/// Empty latest_commitment
		EmptyLatestCommitment,
		/// Invalid token id
		InvalidTokenId,
		/// Wrong assert id
		WrongAssetId,
	}

	/// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	/// These functions materialize as "extrinsic", which are often compared to transactions.
	/// Dispatch able functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// This function acts as an entry for most of the IBC request.
		/// I.e., create clients, update clients, handshakes to create channels, ...etc
		///
		/// The origin must be Signed and the sender must have sufficient funds fee.
		///
		/// Parameters:
		/// - `messages`: The arbitrary ICS message's representation in Substrate, which contains an
		///   URL and
		///  a serialized protocol buffer message. The URL name that uniquely identifies the type of
		/// the serialized protocol buffer message.
		///
		/// The relevant events are emitted when successful.
		#[pallet::weight(0)]
		pub fn deliver(origin: OriginFor<T>, messages: Vec<Any>) -> DispatchResultWithPostInfo {
			sp_tracing::within_span!(
			sp_tracing::Level::TRACE, "deliver";
			{
				let _sender = ensure_signed(origin)?;
				let mut ctx = Context::<T>::default();

				let messages: Vec<ibc_proto::google::protobuf::Any> = messages
					.into_iter()
					.map(|message| ibc_proto::google::protobuf::Any {
						type_url: String::from_utf8(message.type_url.clone()).unwrap(),
						value: message.value,
					})
					.collect();

				for (_, message) in messages.into_iter().enumerate() {

					match ibc::core::ics26_routing::handler::deliver(&mut ctx, message.clone()) {
						Ok(ibc::core::ics26_routing::handler::MsgReceipt { events, log: _log}) => {
							log::trace!(target: LOG_TARGET, "deliver events  : {:?} ", events);
							// deposit events about send packet event and ics20 transfer event
							for event in events {
								match event {
									IbcEvent::WriteAcknowledgement(ref write_ack) => {
										store_write_ack::<T>(write_ack);
										Self::deposit_event(event.clone().into());
									}
									_ => {
										log::trace!(target: LOG_TARGET, "raw_transfer event : {:?} ", event);
										Self::deposit_event(event.clone().into());
									}
								}
							}
						}
						Err(error) => {
							log::trace!(
								target: LOG_TARGET,
								"deliver error  : {:?} ",
								error
							);
						}
					};
				}

				Ok(().into())
			})
		}
	}
}

fn store_write_ack<T: Config>(
	write_ack_event: &ibc::core::ics04_channel::events::WriteAcknowledgement,
) {
	// store ack
	let port_id = write_ack_event.packet.source_port.as_bytes().to_vec();
	let channel_id = write_ack_event.packet.source_channel.clone().to_string().as_bytes().to_vec();
	let sequence = u64::from(write_ack_event.packet.sequence);
	let write_ack = write_ack_event.encode_vec().unwrap();

	// store.Set((portID, channelID, sequence), WriteAckEvent)
	<WriteAckPacketEvent<T>>::insert((port_id, channel_id, sequence), write_ack);
}

impl<T: Config> AssetIdAndNameProvider<T::AssetId> for Pallet<T> {
	type Err = Error<T>;

	fn try_get_asset_id(name: impl AsRef<[u8]>) -> Result<<T as Config>::AssetId, Self::Err> {
		let asset_id = <AssetIdByName<T>>::try_get(name.as_ref().to_vec());
		match asset_id {
			Ok(id) => Ok(id),
			_ => Err(Error::<T>::InvalidTokenId),
		}
	}

	fn try_get_asset_name(asset_id: T::AssetId) -> Result<Vec<u8>, Self::Err> {
		let token_id = <AssetIdByName<T>>::iter().find(|p| p.1 == asset_id).map(|p| p.0);
		match token_id {
			Some(id) => Ok(id),
			_ => Err(Error::<T>::WrongAssetId),
		}
	}
}

