use super::*;
use crate as pallet_ics20_transfer;
use codec::Encode;
pub use frame_support::{
	construct_runtime, parameter_types,
	traits::{
		AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32, ConstU8, KeyOwnerProofSystem,
		Randomness, StorageInfo,
	},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight},
		IdentityFee, Weight,
	},
	StorageValue,
};
use frame_system as system;
use frame_system::EnsureRoot;
use ibc_support::module::Router;
use pallet_assets::AssetsCallback;
use sp_io::storage;
use sp_runtime::{
	generic,
	traits::{AccountIdLookup, BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature,
};

pub type Signature = MultiSignature;
pub(crate) type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Assets: pallet_assets::<Instance1>,
		Balances: pallet_balances,
		Timestamp: pallet_timestamp,
		Ibc: pallet_ibc,
		Ics20Transfer: pallet_ics20_transfer,
	}
);

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub const SS58Prefix: u8 = 42;
}

/// Index of a transaction in the chain.
pub type Index = u32;
/// An index to a block.
pub type BlockNumber = u32;

impl frame_system::Config for Test {
	/// The basic call filter to use in dispatchable.
	type BaseCallFilter = frame_support::traits::Everything;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = ();
	/// The maximum length of a block (in bytes).
	type BlockLength = ();
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type RuntimeCall = RuntimeCall;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	/// The ubiquitous origin type.
	type RuntimeOrigin = RuntimeOrigin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = ();
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = ();
	/// Version of the runtime.
	type Version = ();
	/// Converts a module to the index of the module in `construct_runtime!`.
	///
	/// This type is being generated by `construct_runtime!`.
	type PalletInfo = PalletInfo;
	/// What to do if a new account is created.
	type OnNewAccount = ();
	/// What to do if an account is fully reaped from the system.
	type OnKilledAccount = ();
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// Weight information for the extrinsics of this pallet.
	type SystemWeightInfo = ();
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = ConstU16<42>;
	/// The set code logic, just the default since we're not a parachain.
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub type Balance = u128;
/// Type used for expressing timestamp.
pub type Moment = u64;

pub const MILLICENTS: Balance = 10_000_000_000_000;
pub const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
pub const DOLLARS: Balance = 100 * CENTS;

parameter_types! {
	pub const AssetDeposit: Balance = 100 * DOLLARS;
	pub const ApprovalDeposit: Balance = 1 * DOLLARS;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10 * DOLLARS;
	pub const MetadataDepositPerByte: Balance = 1 * DOLLARS;
}

pub struct AssetsCallbackHandle;
impl AssetsCallback<AssetId, AccountId> for AssetsCallbackHandle {
	fn created(_id: &AssetId, _owner: &AccountId) -> Result<(), ()> {
		storage::set(b"asset_created", &().encode());
		Ok(())
	}

	fn destroyed(_id: &AssetId) -> Result<(), ()> {
		storage::set(b"asset_destroyed", &().encode());
		Ok(())
	}
}

impl pallet_assets::Config<pallet_assets::Instance1> for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = AssetBalance;
	type AssetId = AssetId;
	type AssetIdParameter = u32;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<DOLLARS>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type RemoveItemsLimit = ConstU32<5>;
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Test>;
	type CallbackHandle = AssetsCallbackHandle;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * DOLLARS;
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = frame_system::Pallet<Test>;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Test>;
}

parameter_types! {
	pub const MinimumPeriod: Moment = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Test {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const MaxAuthorities: u32 = 100;
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

pub const MILLISECS_PER_BLOCK: Moment = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: Moment = MILLISECS_PER_BLOCK;

use ibc::applications::transfer::MODULE_ID_STR;

pub struct IbcModule;

impl ibc_support::module::AddModule for IbcModule {
	fn add_module(router: Router) -> Router {
		match router.clone().add_route(
			MODULE_ID_STR.parse().expect("never failed"),
			pallet_ics20_transfer::callback::IbcTransferModule::<Test>(
				std::marker::PhantomData::<Test>,
			),
		) {
			Ok(ret) => ret,
			Err(e) => panic!("add module failed by {}", e),
		}
	}
}

impl pallet_ics20_transfer::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type AssetId = AssetId;
	type AssetBalance = AssetBalance;
	type Fungibles = Assets;
	type AssetIdByName = Ics20Transfer;
	type IbcContext = pallet_ibc::context::Context<Test>;
	type AccountIdConversion = pallet_ics20_transfer::r#impl::IbcAccount;
	const NATIVE_TOKEN_NAME: &'static [u8] = b"DEMO";
}

pub type AssetBalance = u128;
pub type AssetId = u32;

parameter_types! {
	pub const ExpectedBlockTime: u64 = 6;
	pub const ChainVersion: u64 = 0;
}

impl pallet_ibc::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type TimeProvider = pallet_timestamp::Pallet<Test>;
	type ExpectedBlockTime = ExpectedBlockTime;
	const IBC_COMMITMENT_PREFIX: &'static [u8] = b"Ibc";
	type ChainVersion = ChainVersion;
	type IbcModule = IbcModule;
	type WeightInfo = ();
}

#[allow(dead_code)]
// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
}
