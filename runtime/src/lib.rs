#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit="256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_std::prelude::*;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata};
use sp_runtime::{
	ApplyExtrinsicResult,
	create_runtime_str, impl_opaque_keys,
	generic, curve::PiecewiseLinear,
	transaction_validity::{TransactionValidity, TransactionSource, TransactionPriority},
};
use sp_runtime::traits::{
	BlakeTwo256, Block as BlockT, AccountIdLookup, OpaqueKeys, Verify, IdentifyAccount, NumberFor,
};
use sp_api::impl_runtime_apis;
use pallet_grandpa::{AuthorityId as GrandpaId, AuthorityList as GrandpaAuthorityList};
use pallet_grandpa::fg_primitives;
use sp_version::RuntimeVersion;
#[cfg(feature = "std")]
use sp_version::NativeVersion;

// A few exports that help ease life for downstream crates.
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use pallet_staking::StakerStatus;
pub use pallet_timestamp::Call as TimestampCall;
pub use pallet_balances::Call as BalancesCall;
pub use sp_runtime::{Permill, Perbill};
pub use frame_support::{
	construct_runtime, parameter_types, StorageValue,
	traits::{KeyOwnerProofSystem, Randomness, U128CurrencyToVote},
	weights::{
		DispatchClass, Weight, IdentityFee,
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
	},
};
use pallet_transaction_payment::CurrencyAdapter;
use pallet_session::historical as pallet_session_historical;

pub use template;

pub mod common;
pub use common::*;
pub use common::opaque;
pub mod constants;
pub use constants::{currency::*, time::*, fee::*};

pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("node-template"),
	impl_name: create_runtime_str!("node-template"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub babe: Babe,
		pub grandpa: Grandpa,
	}
}

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;
	pub const BlockHashCount: BlockNumber = 2400;

	/// We allow for 2 seconds of compute with a 6 second average block time.
	pub BlockWeights: frame_system::limits::BlockWeights = frame_system::limits::BlockWeights
		::with_sensible_defaults(2 * WEIGHT_PER_SECOND, NORMAL_DISPATCH_RATIO);
	pub BlockLength: frame_system::limits::BlockLength = frame_system::limits::BlockLength
		::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);

	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type Origin = Origin;

	type Version = Version;
	type BaseCallFilter = ();
	type BlockWeights = BlockWeights;
	type BlockLength = BlockLength;
	type BlockNumber = BlockNumber; // u32
	type BlockHashCount = BlockHashCount;

	type Hash = Hash; // H256
	type Hashing = BlakeTwo256;
	type Index = Index; // u32
	type DbWeight = RocksDbWeight;

	type AccountId = AccountId;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	type PalletInfo = PalletInfo;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type AccountData = pallet_balances::AccountData<Balance>;
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
}

parameter_types! {
	pub const EpochDuration: u64 = EPOCH_DURATION_IN_BLOCKS as u64;
	pub const ExpectedBlockTime: Moment = MILLISECS_PER_BLOCK;
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ExpectedBlockTime;
	type EpochChangeTrigger = pallet_babe::ExternalTrigger;
	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::Proof;
	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, pallet_babe::AuthorityId)>>::IdentificationTuple;
	type HandleEquivocation = pallet_babe::EquivocationHandler<Self::KeyOwnerIdentification, ()>; // Offences
	type WeightInfo = ();
}

impl pallet_grandpa::Config for Runtime {
	type Event = Event;
	type Call = Call;

	type KeyOwnerProofSystem = Historical;
	type KeyOwnerProof = <Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId)>>::Proof;
	type KeyOwnerIdentification =
		<Self::KeyOwnerProofSystem as KeyOwnerProofSystem<(KeyTypeId, GrandpaId, )>>::IdentificationTuple;
	type HandleEquivocation = pallet_grandpa::EquivocationHandler<Self::KeyOwnerIdentification, ()>; // Offences

	type WeightInfo = ();
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(17);
}

impl pallet_session::Config for Runtime {
	type Event = Event;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_staking::StashOf<Self>;

	type Keys = SessionKeys;
	type ShouldEndSession = Babe;
	type NextSessionRotation = Babe;
	type SessionManager = pallet_session::historical::NoteHistoricalRoot<Self, Staking>;
	type SessionHandler = <SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type WeightInfo = ();
}

impl pallet_session::historical::Config for Runtime {
	type FullIdentification = pallet_staking::Exposure<AccountId, Balance>;
	type FullIdentificationOf = pallet_staking::ExposureOf<Runtime>;
}

pallet_staking_reward_curve::build! {
	const REWARD_CURVE: PiecewiseLinear<'static> = curve!(
		min_inflation: 0_025_000,
		max_inflation: 0_100_000,
		ideal_stake: 0_500_000,
		falloff: 0_050_000,
		max_piece_count: 40,
		test_precision: 0_005_000,
	);
}

parameter_types! {
	pub const SessionsPerEra: sp_staking::SessionIndex = 3; // 3 hours
	pub const BondingDuration: pallet_staking::EraIndex = 4; // 12 hours
	pub const SlashDeferDuration: pallet_staking::EraIndex = 2; // 6 hours
	pub const RewardCurve: &'static PiecewiseLinear<'static> = &REWARD_CURVE;
	pub const ElectionLookahead: BlockNumber = EPOCH_DURATION_IN_BLOCKS / 4;

	pub const MaxNominatorRewardedPerValidator: u32 = 64;
	pub const MaxIterations: u32 = 5;
	// 0.05%. The higher the value, the more strict solution acceptance becomes.
	pub MinSolutionScoreBump: Perbill = Perbill::from_rational_approximation(5u32, 10_000);
	pub StakingUnsignedPriority: TransactionPriority =
		Perbill::from_percent(90) * TransactionPriority::max_value();
}

parameter_types! {
	/// A limit for off-chain phragmen unsigned solution submission.
	///
	/// We want to keep it as high as possible, but can't risk having it reject,
	/// so we always subtract the base block execution weight.
	pub OffchainSolutionWeightLimit: Weight = BlockWeights::get()
		.get(DispatchClass::Normal)
		.max_extrinsic
		.expect("Normal extrinsics have weight limit configured by default; qed")
		.saturating_sub(BlockExecutionWeight::get());
}

// TODO: Kusama
// type SlashCancelOrigin = EnsureOneOf<
// 	AccountId,
// 	EnsureRoot<AccountId>,
// 	pallet_collective::EnsureProportionAtLeast<_1, _2, AccountId, CouncilCollective>
// >;

impl pallet_staking::Config for Runtime {
	type Call = Call;
	type Event = Event;
	type UnixTime = Timestamp;

	type Currency = Balances;
	type CurrencyToVote = U128CurrencyToVote;
	type RewardRemainder = (); // TODO: Treasury
	type Slash = (); // TODO: Treasury
	type Reward = (); // rewards are minted from the voi

	type SessionInterface = Self;
	type NextNewSession = Session;
	type SessionsPerEra = SessionsPerEra;
	type BondingDuration = BondingDuration;
	type SlashDeferDuration = SlashDeferDuration;
	/// A super-majority of the council can cancel the slash.
	type SlashCancelOrigin = frame_system::EnsureRoot<Self::AccountId>; // TODO: SlashCancelOrigin
	type RewardCurve = RewardCurve;
	type ElectionLookahead = ElectionLookahead;

	type MinSolutionScoreBump = MinSolutionScoreBump;
	type MaxIterations = MaxIterations;
	type MaxNominatorRewardedPerValidator = MaxNominatorRewardedPerValidator;
	type UnsignedPriority = StakingUnsignedPriority; // Kusama
	type OffchainSolutionWeightLimit = OffchainSolutionWeightLimit;
	type WeightInfo = ();
}

// impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
// 	where
// 		Call: From<LocalCall>,
// {
// 	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
// 		call: Call,
// 		public: <Signature as sp_runtime::traits::Verify>::Signer,
// 		account: AccountId,
// 		nonce: Nonce,
// 	) -> Option<(
// 		Call,
// 		<UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload,
// 	)> {
// 		// take the biggest period possible.
// 		let period = BlockHashCount::get()
// 			.checked_next_power_of_two()
// 			.map(|c| c / 2)
// 			.unwrap_or(2) as u64;
// 		let current_block = System::block_number()
// 			.saturated_into::<u64>()
// 			// The `System::block_number` is initialized with `n+1`,
// 			// so the actual block number is `n`.
// 			.saturating_sub(1);
// 		let tip = 0;
// 		let extra: SignedExtra = (
// 			frame_system::CheckSpecVersion::<Runtime>::new(),
// 			frame_system::CheckTxVersion::<Runtime>::new(),
// 			frame_system::CheckGenesis::<Runtime>::new(),
// 			frame_system::CheckEra::<Runtime>::from(generic::Era::mortal(period, current_block)),
// 			frame_system::CheckNonce::<Runtime>::from(nonce),
// 			frame_system::CheckWeight::<Runtime>::new(),
// 			module_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
// 		);
// 		let raw_payload = SignedPayload::new(call, extra)
// 			.map_err(|e| {
// 				debug::warn!("Unable to create signed payload: {:?}", e);
// 			})
// 			.ok()?;
// 		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
// 		let address = Indices::unlookup(account);
// 		let (call, extra, _) = raw_payload.deconstruct();
// 		Some((call, (address, signature, extra)))
// 	}
// }
// impl frame_system::offchain::SigningTypes for Runtime {
// 	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
// 	type Signature = Signature;
// }

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
	where
		Call: From<C>,
{
	type OverarchingCall = Call;
	type Extrinsic = UncheckedExtrinsic;
}


parameter_types! {
	pub const UncleGenerations: BlockNumber = 5;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Babe>;
	type UncleGenerations = UncleGenerations; // 5
	type FilterUncle = ();
	type EventHandler = (Staking, ()); // ImOnline
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type Event = Event;
	type Balance = Balance;
	type AccountStore = System;

	type ExistentialDeposit = ExistentialDeposit; // 500?
	type MaxLocks = MaxLocks;
	type DustRemoval = ();

	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const IndexDeposit: Balance = DOLLARS;
}

impl pallet_indices::Config for Runtime {
	type Event = Event;
	type Currency = Balances;
	type Deposit = IndexDeposit;
	type AccountIndex = AccountIndex;
	type WeightInfo = ();
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = Moment;
	type OnTimestampSet = Babe;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	pub const TransactionByteFee: Balance = 1;
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Config for Runtime {
	type Event = Event;
	type Call = Call;
}

/// Configure the pallet template in pallets/template.
impl template::Config for Runtime {
	type Event = Event;
}

// TODO: This
// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{

		Sudo: pallet_sudo::{Module, Call, Config<T>, Storage, Event<T>},
		System: frame_system::{Module, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Module, Call, Storage, Config<T>, Event<T>},
		Indices: pallet_indices::{Module, Call, Storage, Config<T>, Event<T>},

		Timestamp: pallet_timestamp::{Module, Call, Storage, Inherent},
		TransactionPayment: pallet_transaction_payment::{Module, Storage},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Module, Call, Storage},

		Babe: pallet_babe::{Module, Call, Storage, Config, Inherent, ValidateUnsigned},
		Grandpa: pallet_grandpa::{Module, Call, Storage, Config, Event, ValidateUnsigned},
		Historical: pallet_session_historical::{Module},
		Authorship: pallet_authorship::{Module, Call, Storage, Inherent},
		Session: pallet_session::{Module, Call, Storage, Event, Config<T>},
		Staking: pallet_staking::{Module, Call, Config<T>, Storage, Event<T>},

		// ImOnline: pallet_im_online::{Module, Call, Storage, Event<T>, ValidateUnsigned, Config<T>}
		// AuthorityDiscovery: pallet_authority_discovery::{Module, Call, Config}
		// Offences: pallet_offences::{Module, Call, Storage, Event}

		// Democracy: pallet_democracy::{Module, Call, Storage, Config, Event<T>},
		// Council: pallet_collective::<Instance1>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		// TechnicalCommittee: pallet_collective::<Instance2>::{Module, Call, Storage, Origin<T>, Event<T>, Config<T>},
		// ElectionsPhragmen: pallet_elections_phragmen::{Module, Call, Storage, Event<T>, Config<T>},
		// TechnicalMembership: pallet_membership::<Instance1>::{Module, Call, Storage, Event<T>, Config<T>},
		// Treasury: pallet_treasury::{Module, Call, Storage, Config, Event<T>},

		// Include the custom logic from the template pallet in the runtime.
		TemplateModule: template::{Module, Call, Storage, Event<T>},
	}
);


pub type SignedExtra = (
	frame_system::CheckSpecVersion<Runtime>,
	frame_system::CheckTxVersion<Runtime>,
	frame_system::CheckGenesis<Runtime>,
	frame_system::CheckEra<Runtime>,
	frame_system::CheckNonce<Runtime>,
	frame_system::CheckWeight<Runtime>,
	pallet_transaction_payment::ChargeTransactionPayment<Runtime>
);

pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllModules,
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}

		fn random_seed() -> <Block as BlockT>::Hash {
			RandomnessCollectiveFlip::random_seed()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeGenesisConfiguration {
			sp_consensus_babe::BabeGenesisConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: PRIMARY_PROBABILITY,
				genesis_authorities: Babe::authorities(),
				randomness: Babe::randomness(),
				allowed_slots: sp_consensus_babe::AllowedSlots::PrimaryAndSecondaryPlainSlots,
			}
		}

		fn current_epoch_start() -> sp_consensus_babe::Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			use codec::Encode;

			Historical::prove((sp_consensus_babe::KEY_TYPE, authority_id))
				.map(|p| p.encode())
				.map(sp_consensus_babe::OpaqueKeyOwnershipProof::new)
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			equivocation_proof: sp_consensus_babe::EquivocationProof<<Block as BlockT>::Header>,
			key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
			) -> Option<()> {
			let key_owner_proof = key_owner_proof.decode()?;

			Babe::submit_unsigned_equivocation_report(
				equivocation_proof,
				key_owner_proof,
				)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl fg_primitives::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> GrandpaAuthorityList {
			Grandpa::grandpa_authorities()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: fg_primitives::EquivocationProof<
				<Block as BlockT>::Hash,
				NumberFor<Block>,
			>,
			_key_owner_proof: fg_primitives::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: fg_primitives::SetId,
			_authority_id: GrandpaId,
		) -> Option<fg_primitives::OpaqueKeyOwnershipProof> {
			// NOTE: this is the only implementation possible since we've
			// defined our key owner proof type as a bottom type (i.e. a type
			// with no values).
			None
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Index> for Runtime {
		fn account_nonce(account: AccountId) -> Index {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
			use frame_benchmarking::{Benchmarking, BenchmarkBatch, add_benchmark, TrackedStorageKey};

			use frame_system_benchmarking::Module as SystemBench;
			impl frame_system_benchmarking::Config for Runtime {}

			let whitelist: Vec<TrackedStorageKey> = vec![
				// Block Number
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef702a5c1b19ab7a04f536c519aca4983ac").to_vec().into(),
				// Total Issuance
				hex_literal::hex!("c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80").to_vec().into(),
				// Execution Phase
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef7ff553b5a9862a516939d82b3d3d8661a").to_vec().into(),
				// Event Count
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef70a98fdbe9ce6c55837576c60c7af3850").to_vec().into(),
				// System Events
				hex_literal::hex!("26aa394eea5630e07c48ae0c9558cef780d41e5e16056765bc8461851072c9d7").to_vec().into(),
			];

			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);

			add_benchmark!(params, batches, frame_system, SystemBench::<Runtime>);
			add_benchmark!(params, batches, pallet_balances, Balances);
			add_benchmark!(params, batches, pallet_timestamp, Timestamp);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}
}
