// This file is part of Substrate.

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

//! The Substrate runtime. This can be compiled with `#[no_std]`, ready for Wasm.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "std")]
pub mod extrinsic;
#[cfg(feature = "std")]
pub mod genesismap;
pub mod substrate_test_pallet;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{
	construct_runtime, derive_impl,
	dispatch::DispatchClass,
	genesis_builder_helper::{build_state, get_preset},
	parameter_types,
	traits::{ConstU32, ConstU64},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, WEIGHT_REF_TIME_PER_SECOND},
		Weight,
	},
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	CheckNonce, CheckWeight,
};
use scale_info::TypeInfo;
use sp_application_crypto::Ss58Codec;
use sp_keyring::Sr25519Keyring;

use sp_application_crypto::{ecdsa, ed25519, sr25519, RuntimeAppPublic};

#[cfg(feature = "bls-experimental")]
use sp_application_crypto::{bls381, ecdsa_bls381};

use sp_core::{OpaqueMetadata, RuntimeDebug};
use sp_trie::{
	trie_types::{TrieDBBuilder, TrieDBMutBuilderV1},
	PrefixedMemoryDB, StorageProof,
};
use trie_db::{Trie, TrieMut};

use serde_json::json;
use sp_api::{decl_runtime_apis, impl_runtime_apis};
pub use sp_core::hash::H256;
use sp_genesis_builder::PresetId;
use sp_inherents::{CheckInherentsResult, InherentData};
use sp_runtime::{
	impl_opaque_keys, impl_tx_ext_default,
	traits::{BlakeTwo256, Block as BlockT, DispatchInfoOf, Dispatchable, NumberFor, Verify},
	transaction_validity::{
		TransactionSource, TransactionValidity, TransactionValidityError, ValidTransaction,
	},
	ApplyExtrinsicResult, ExtrinsicInclusionMode, Perbill,
};
#[cfg(any(feature = "std", test))]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

pub use sp_consensus_babe::{AllowedSlots, BabeEpochConfiguration, Slot};

pub use pallet_balances::Call as BalancesCall;

pub type AuraId = sp_consensus_aura::sr25519::AuthorityId;
#[cfg(feature = "std")]
pub use extrinsic::{ExtrinsicBuilder, Transfer};

const LOG_TARGET: &str = "substrate-test-runtime";

// Include the WASM binary
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

#[cfg(feature = "std")]
pub mod wasm_binary_logging_disabled {
	include!(concat!(env!("OUT_DIR"), "/wasm_binary_logging_disabled.rs"));
}

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_unwrap() -> &'static [u8] {
	WASM_BINARY.expect(
		"Development wasm binary is not available. Testing is only supported with the flag
		 disabled.",
	)
}

/// Wasm binary unwrapped. If built with `SKIP_WASM_BUILD`, the function panics.
#[cfg(feature = "std")]
pub fn wasm_binary_logging_disabled_unwrap() -> &'static [u8] {
	wasm_binary_logging_disabled::WASM_BINARY.expect(
		"Development wasm binary is not available. Testing is only supported with the flag
		 disabled.",
	)
}

/// Test runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: alloc::borrow::Cow::Borrowed("test"),
	impl_name: alloc::borrow::Cow::Borrowed("parity-test"),
	authoring_version: 1,
	spec_version: 2,
	impl_version: 2,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	system_version: 1,
};

fn version() -> RuntimeVersion {
	VERSION
}

/// Native version.
#[cfg(any(feature = "std", test))]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// Transfer data extracted from Extrinsic containing `Balances::transfer_allow_death`.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo)]
pub struct TransferData {
	pub from: AccountId,
	pub to: AccountId,
	pub amount: Balance,
	pub nonce: Nonce,
}

/// The address format for describing accounts.
pub type Address = sp_core::sr25519::Public;
pub type Signature = sr25519::Signature;
#[cfg(feature = "std")]
pub type Pair = sp_core::sr25519::Pair;

// TODO: Remove after the Checks are migrated to TxExtension.
/// The extension to the basic transaction logic.
pub type TxExtension = (
	(CheckNonce<Runtime>, CheckWeight<Runtime>),
	CheckSubstrateCall,
	frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
	frame_system::WeightReclaim<Runtime>,
);
/// The payload being signed in transactions.
pub type SignedPayload = sp_runtime::generic::SignedPayload<RuntimeCall, TxExtension>;
/// Unchecked extrinsic type as expected by this runtime.
pub type Extrinsic =
	sp_runtime::generic::UncheckedExtrinsic<Address, RuntimeCall, Signature, TxExtension>;

/// An identifier for an account on this system.
pub type AccountId = <Signature as Verify>::Signer;
/// A simple hash type for all our hashing.
pub type Hash = H256;
/// The hashing algorithm used.
pub type Hashing = BlakeTwo256;
/// The block number type used in this runtime.
pub type BlockNumber = u64;
/// Index of a transaction.
pub type Nonce = u64;
/// The item of a block digest.
pub type DigestItem = sp_runtime::generic::DigestItem;
/// The digest of a block.
pub type Digest = sp_runtime::generic::Digest;
/// A test block.
pub type Block = sp_runtime::generic::Block<Header, Extrinsic>;
/// A test block's header.
pub type Header = sp_runtime::generic::Header<BlockNumber, Hashing>;
/// Balance of an account.
pub type Balance = u64;

#[cfg(feature = "bls-experimental")]
mod bls {
	use sp_application_crypto::{bls381, ecdsa_bls381};
	pub type Bls381Public = bls381::AppPublic;
	pub type Bls381Pop = bls381::AppSignature;
	pub type EcdsaBls381Public = ecdsa_bls381::AppPublic;
	pub type EcdsaBls381Pop = ecdsa_bls381::AppSignature;
}
#[cfg(not(feature = "bls-experimental"))]
mod bls {
	pub type Bls381Public = ();
	pub type Bls381Pop = ();
	pub type EcdsaBls381Public = ();
	pub type EcdsaBls381Pop = ();
}
pub use bls::*;

pub type EcdsaPop = ecdsa::AppSignature;
pub type Sr25519Pop = sr25519::AppSignature;
pub type Ed25519Pop = ed25519::AppSignature;

decl_runtime_apis! {
	#[api_version(2)]
	pub trait TestAPI {
		/// Return the balance of the given account id.
		fn balance_of(id: AccountId) -> u64;
		/// A benchmark function that adds one to the given value and returns the result.
		fn benchmark_add_one(val: &u64) -> u64;
		/// A benchmark function that adds one to each value in the given vector and returns the
		/// result.
		fn benchmark_vector_add_one(vec: &Vec<u64>) -> Vec<u64>;
		/// A function for that the signature changed in version `2`.
		#[changed_in(2)]
		fn function_signature_changed() -> Vec<u64>;
		/// The new signature.
		fn function_signature_changed() -> u64;
		/// trie no_std testing
		fn use_trie() -> u64;
		/// Calls function in the loop using never-inlined function pointer
		fn benchmark_indirect_call() -> u64;
		/// Calls function in the loop
		fn benchmark_direct_call() -> u64;
		/// Allocates vector with given capacity.
		fn vec_with_capacity(size: u32) -> Vec<u8>;
		/// Returns the initialized block number.
		fn get_block_number() -> u64;
		/// Test that `ed25519` crypto works in the runtime.
		///
		/// Returns the signature generated for the message `ed25519` both the public key and proof of possession.
		fn test_ed25519_crypto() -> (ed25519::AppSignature, ed25519::AppPublic, Ed25519Pop);
		/// Test that `sr25519` crypto works in the runtime.
		///
		/// Returns the signature generated for the message `sr25519` both the public key and proof of possession.
		fn test_sr25519_crypto() -> (sr25519::AppSignature, sr25519::AppPublic, Sr25519Pop);
		/// Test that `ecdsa` crypto works in the runtime.
		///
		/// Returns the signature generated for the message `ecdsa` both the public key and proof of possession.
		fn test_ecdsa_crypto() -> (ecdsa::AppSignature, ecdsa::AppPublic, EcdsaPop);
		/// Test that `bls381` crypto works in the runtime
		///
		/// Returns both the proof of possession and public key.
		fn test_bls381_crypto() -> (Bls381Pop, Bls381Public);
		/// Test that `ecdsa_bls381_crypto` works in the runtime
		///
		/// Returns both the proof of possession and public key.
		fn test_ecdsa_bls381_crypto() -> (EcdsaBls381Pop, EcdsaBls381Public);
		/// Run various tests against storage.
		fn test_storage();
		/// Check a witness.
		fn test_witness(proof: StorageProof, root: crate::Hash);
		/// Test that ensures that we can call a function that takes multiple
		/// arguments.
		fn test_multiple_arguments(data: Vec<u8>, other: Vec<u8>, num: u32);
		/// Traces log "Hey I'm runtime."
		fn do_trace_log();
		/// Verify the given signature, public & message bundle.
		fn verify_ed25519(sig: ed25519::Signature, public: ed25519::Public, message: Vec<u8>) -> bool;
		/// Write the given `value` under the given `key` into the storage and then optional panic.
		fn write_key_value(key: Vec<u8>, value: Vec<u8>, panic: bool);
	}
}

pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPalletsWithSystem,
>;

#[derive(
	Copy, Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, RuntimeDebug, TypeInfo,
)]
pub struct CheckSubstrateCall;

impl sp_runtime::traits::Printable for CheckSubstrateCall {
	fn print(&self) {
		"CheckSubstrateCall".print()
	}
}

impl sp_runtime::traits::RefundWeight for CheckSubstrateCall {
	fn refund(&mut self, _weight: frame_support::weights::Weight) {}
}
impl sp_runtime::traits::ExtensionPostDispatchWeightHandler<CheckSubstrateCall>
	for CheckSubstrateCall
{
	fn set_extension_weight(&mut self, _info: &CheckSubstrateCall) {}
}

impl sp_runtime::traits::Dispatchable for CheckSubstrateCall {
	type RuntimeOrigin = RuntimeOrigin;
	type Config = CheckSubstrateCall;
	type Info = CheckSubstrateCall;
	type PostInfo = CheckSubstrateCall;

	fn dispatch(
		self,
		_origin: Self::RuntimeOrigin,
	) -> sp_runtime::DispatchResultWithInfo<Self::PostInfo> {
		panic!("This implementation should not be used for actual dispatch.");
	}
}

impl sp_runtime::traits::TransactionExtension<RuntimeCall> for CheckSubstrateCall {
	const IDENTIFIER: &'static str = "CheckSubstrateCall";
	type Implicit = ();
	type Pre = ();
	type Val = ();
	impl_tx_ext_default!(RuntimeCall; weight prepare);

	fn validate(
		&self,
		origin: <RuntimeCall as Dispatchable>::RuntimeOrigin,
		call: &RuntimeCall,
		_info: &DispatchInfoOf<RuntimeCall>,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Encode,
		_source: TransactionSource,
	) -> Result<
		(ValidTransaction, Self::Val, <RuntimeCall as Dispatchable>::RuntimeOrigin),
		TransactionValidityError,
	> {
		log::trace!(target: LOG_TARGET, "validate");
		let v = match call {
			RuntimeCall::SubstrateTest(ref substrate_test_call) =>
				substrate_test_pallet::validate_runtime_call(substrate_test_call)?,
			_ => Default::default(),
		};
		Ok((v, (), origin))
	}
}

construct_runtime!(
	pub enum Runtime
	{
		System: frame_system,
		Babe: pallet_babe,
		SubstrateTest: substrate_test_pallet::pallet,
		Balances: pallet_balances,
	}
);

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// Max weight, actual value does not matter for test runtime.
const MAXIMUM_BLOCK_WEIGHT: Weight =
	Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2), u64::MAX);

parameter_types! {
	pub const BlockHashCount: BlockNumber = 2400;
	pub const Version: RuntimeVersion = VERSION;

	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);

	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::pallet::Config for Runtime {
	type BlockWeights = RuntimeBlockWeights;
	type Nonce = Nonce;
	type AccountId = AccountId;
	type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

pub mod currency {
	use crate::Balance;
	const MILLICENTS: Balance = 1_000_000_000;
	const CENTS: Balance = 1_000 * MILLICENTS; // assume this is worth about a cent.
	pub const DOLLARS: Balance = 100 * CENTS;
}

parameter_types! {
	pub const ExistentialDeposit: Balance = 1 * currency::DOLLARS;
	// For weight estimation, we assume that the most locks on an individual account will be 50.
	// This number may need to be adjusted in the future if this assumption no longer holds true.
	pub const MaxLocks: u32 = 50;
	pub const MaxReserves: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = MaxLocks;
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = [u8; 8];
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

impl substrate_test_pallet::Config for Runtime {}

// Required for `pallet_babe::Config`.
impl pallet_timestamp::Config for Runtime {
	type Moment = u64;
	type OnTimestampSet = Babe;
	type MinimumPeriod = ConstU64<500>;
	type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const EpochDuration: u64 = 6;
}

impl pallet_babe::Config for Runtime {
	type EpochDuration = EpochDuration;
	type ExpectedBlockTime = ConstU64<10_000>;
	type EpochChangeTrigger = pallet_babe::SameAuthoritiesForever;
	type DisabledValidators = ();
	type KeyOwnerProof = sp_core::Void;
	type EquivocationReportSystem = ();
	type WeightInfo = ();
	type MaxAuthorities = ConstU32<10>;
	type MaxNominators = ConstU32<100>;
}

/// Adds one to the given input and returns the final result.
#[inline(never)]
fn benchmark_add_one(i: u64) -> u64 {
	i + 1
}

fn code_using_trie() -> u64 {
	let pairs = [
		(b"0103000000000000000464".to_vec(), b"0400000000".to_vec()),
		(b"0103000000000000000469".to_vec(), b"0401000000".to_vec()),
	]
	.to_vec();

	let mut mdb = PrefixedMemoryDB::default();
	let mut root = core::default::Default::default();
	{
		let mut t = TrieDBMutBuilderV1::<Hashing>::new(&mut mdb, &mut root).build();
		for (key, value) in &pairs {
			if t.insert(key, value).is_err() {
				return 101
			}
		}
	}

	let trie = TrieDBBuilder::<Hashing>::new(&mdb, &root).build();
	let res = if let Ok(iter) = trie.iter() { iter.flatten().count() as u64 } else { 102 };

	res
}

impl_opaque_keys! {
	pub struct SessionKeys {
		pub ed25519: ed25519::AppPublic,
		pub sr25519: sr25519::AppPublic,
		pub ecdsa: ecdsa::AppPublic,
	}
}

pub const TEST_RUNTIME_BABE_EPOCH_CONFIGURATION: BabeEpochConfiguration = BabeEpochConfiguration {
	c: (3, 10),
	allowed_slots: AllowedSlots::PrimaryAndSecondaryPlainSlots,
};

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			version()
		}

		fn execute_block(block: Block) {
			log::trace!(target: LOG_TARGET, "execute_block: {block:#?}");
			Executive::execute_block(block);
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> ExtrinsicInclusionMode {
			log::trace!(target: LOG_TARGET, "initialize_block: {header:#?}");
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}
		fn metadata_versions() -> alloc::vec::Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			utx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			let validity = Executive::validate_transaction(source, utx.clone(), block_hash);
			log::trace!(target: LOG_TARGET, "validate_transaction {:?} {:?}", utx, validity);
			validity
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			log::trace!(target: LOG_TARGET, "finalize_block");
			Executive::finalize_block()
		}

		fn inherent_extrinsics(_data: InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			vec![]
		}

		fn check_inherents(_block: Block, _data: InherentData) -> CheckInherentsResult {
			CheckInherentsResult::new()
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl self::TestAPI<Block> for Runtime {
		fn balance_of(id: AccountId) -> u64 {
			Balances::free_balance(id)
		}

		fn benchmark_add_one(val: &u64) -> u64 {
			val + 1
		}

		fn benchmark_vector_add_one(vec: &Vec<u64>) -> Vec<u64> {
			let mut vec = vec.clone();
			vec.iter_mut().for_each(|v| *v += 1);
			vec
		}

		fn function_signature_changed() -> u64 {
			1
		}

		fn use_trie() -> u64 {
			code_using_trie()
		}

		fn benchmark_indirect_call() -> u64 {
			let function = benchmark_add_one;
			(0..1000).fold(0, |p, i| p + function(i))
		}
		fn benchmark_direct_call() -> u64 {
			(0..1000).fold(0, |p, i| p + benchmark_add_one(i))
		}

		fn vec_with_capacity(size: u32) -> Vec<u8> {
			Vec::with_capacity(size as usize)
		}

		fn get_block_number() -> u64 {
			System::block_number()
		}

		fn test_ed25519_crypto() -> (ed25519::AppSignature, ed25519::AppPublic, Ed25519Pop) {
			test_ed25519_crypto()
		}

		fn test_sr25519_crypto() -> (sr25519::AppSignature, sr25519::AppPublic, Sr25519Pop) {
			test_sr25519_crypto()
		}

		fn test_ecdsa_crypto() -> (ecdsa::AppSignature, ecdsa::AppPublic, EcdsaPop) {
			test_ecdsa_crypto()
		}

		#[cfg(feature = "bls-experimental")]
		fn test_bls381_crypto() -> (Bls381Pop, Bls381Public) {
			test_bls381_crypto()
		}

		#[cfg(feature = "bls-experimental")]
		fn test_ecdsa_bls381_crypto() -> (EcdsaBls381Pop, EcdsaBls381Public) {
			test_ecdsa_bls381_crypto()
		}

		#[cfg(not(feature = "bls-experimental"))]
		fn test_bls381_crypto() -> (Bls381Pop, Bls381Public) {
			((),())
		}

		#[cfg(not(feature = "bls-experimental"))]
		fn test_ecdsa_bls381_crypto() -> (EcdsaBls381Pop, EcdsaBls381Public) {
			((), ())
		}

		fn test_storage() {
			test_read_storage();
			test_read_child_storage();
		}

		fn test_witness(proof: StorageProof, root: crate::Hash) {
			test_witness(proof, root);
		}

		fn test_multiple_arguments(data: Vec<u8>, other: Vec<u8>, num: u32) {
			assert_eq!(&data[..], &other[..]);
			assert_eq!(data.len(), num as usize);
		}

		fn do_trace_log() {
			log::trace!(target: "test", "Hey I'm runtime");

			let data = "THIS IS TRACING";

			tracing::trace!(target: "test", %data, "Hey, I'm tracing");
		}

		fn verify_ed25519(sig: ed25519::Signature, public: ed25519::Public, message: Vec<u8>) -> bool {
			sp_io::crypto::ed25519_verify(&sig, &message, &public)
		}

		fn write_key_value(key: Vec<u8>, value: Vec<u8>, panic: bool) {
			sp_io::storage::set(&key, &value);

			if panic {
				panic!("I'm just following my master");
			}
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(1000)
		}

		fn authorities() -> Vec<AuraId> {
			SubstrateTest::authorities().into_iter().map(|auth| AuraId::from(auth)).collect()
		}
	}

	impl sp_consensus_babe::BabeApi<Block> for Runtime {
		fn configuration() -> sp_consensus_babe::BabeConfiguration {
			let epoch_config = Babe::epoch_config().unwrap_or(TEST_RUNTIME_BABE_EPOCH_CONFIGURATION);
			sp_consensus_babe::BabeConfiguration {
				slot_duration: Babe::slot_duration(),
				epoch_length: EpochDuration::get(),
				c: epoch_config.c,
				authorities: Babe::authorities().to_vec(),
				randomness: Babe::randomness(),
				allowed_slots: epoch_config.allowed_slots,
			}
		}

		fn current_epoch_start() -> Slot {
			Babe::current_epoch_start()
		}

		fn current_epoch() -> sp_consensus_babe::Epoch {
			Babe::current_epoch()
		}

		fn next_epoch() -> sp_consensus_babe::Epoch {
			Babe::next_epoch()
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_babe::EquivocationProof<
			<Block as BlockT>::Header,
			>,
			_key_owner_proof: sp_consensus_babe::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_slot: sp_consensus_babe::Slot,
			_authority_id: sp_consensus_babe::AuthorityId,
		) -> Option<sp_consensus_babe::OpaqueKeyOwnershipProof> {
			None
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			let ext = Extrinsic::new_bare(
				substrate_test_pallet::pallet::Call::storage_change{
					key:b"some_key".encode(),
					value:Some(header.number.encode())
				}.into(),
			);
			sp_io::offchain::submit_transaction(ext.encode()).unwrap();
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(_: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(None)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_consensus_grandpa::AuthorityList {
			Vec::new()
		}

		fn current_set_id() -> sp_consensus_grandpa::SetId {
			0
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_consensus_grandpa::EquivocationProof<
			<Block as BlockT>::Hash,
			NumberFor<Block>,
			>,
			_key_owner_proof: sp_consensus_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_consensus_grandpa::SetId,
			_authority_id: sp_consensus_grandpa::AuthorityId,
		) -> Option<sp_consensus_grandpa::OpaqueKeyOwnershipProof> {
			None
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(name: &Option<PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(name, |name| {
				 let patch = match name.as_ref() {
					"staging" => {
						let endowed_accounts: Vec<AccountId> = vec![
							Sr25519Keyring::Bob.public().into(),
							Sr25519Keyring::Charlie.public().into(),
						];

						json!({
							"balances": {
								"balances": endowed_accounts.into_iter().map(|k| (k, 10 * currency::DOLLARS)).collect::<Vec<_>>(),
							},
							"substrateTest": {
								"authorities": [
									Sr25519Keyring::Alice.public().to_ss58check(),
									Sr25519Keyring::Ferdie.public().to_ss58check()
								],
							}
						})
					},
					"foobar" => json!({"foo":"bar"}),
					_ => return None,
				};
				Some(serde_json::to_string(&patch)
					.expect("serialization to json is expected to work. qed.")
					.into_bytes())
			})
		}

		fn preset_names() -> Vec<PresetId> {
			vec![PresetId::from("foobar"), PresetId::from("staging")]
		}
	}
}

fn test_ed25519_crypto() -> (ed25519::AppSignature, ed25519::AppPublic, Ed25519Pop) {
	let mut public0 = ed25519::AppPublic::generate_pair(None);
	let public1 = ed25519::AppPublic::generate_pair(None);
	let public2 = ed25519::AppPublic::generate_pair(None);

	let all = ed25519::AppPublic::all();
	assert!(all.contains(&public0));
	assert!(all.contains(&public1));
	assert!(all.contains(&public2));

	let proof_of_possession = public0
		.generate_proof_of_possession()
		.expect("Cant generate proof_of_possession for ed25519");
	assert!(public0.verify_proof_of_possession(&proof_of_possession));

	let signature = public0.sign(&"ed25519").expect("Generates a valid `ed25519` signature.");
	assert!(public0.verify(&"ed25519", &signature));
	(signature, public0, proof_of_possession)
}

fn test_sr25519_crypto() -> (sr25519::AppSignature, sr25519::AppPublic, Sr25519Pop) {
	let mut public0 = sr25519::AppPublic::generate_pair(None);
	let public1 = sr25519::AppPublic::generate_pair(None);
	let public2 = sr25519::AppPublic::generate_pair(None);

	let all = sr25519::AppPublic::all();
	assert!(all.contains(&public0));
	assert!(all.contains(&public1));
	assert!(all.contains(&public2));

	let proof_of_possession = public0
		.generate_proof_of_possession()
		.expect("Cant generate proof_of_possession for sr25519");
	assert!(public0.verify_proof_of_possession(&proof_of_possession));

	let signature = public0.sign(&"sr25519").expect("Generates a valid `sr25519` signature.");
	assert!(public0.verify(&"sr25519", &signature));
	(signature, public0, proof_of_possession)
}

fn test_ecdsa_crypto() -> (ecdsa::AppSignature, ecdsa::AppPublic, EcdsaPop) {
	let mut public0 = ecdsa::AppPublic::generate_pair(None);
	let public1 = ecdsa::AppPublic::generate_pair(None);
	let public2 = ecdsa::AppPublic::generate_pair(None);

	let all = ecdsa::AppPublic::all();
	assert!(all.contains(&public0));
	assert!(all.contains(&public1));
	assert!(all.contains(&public2));

	let proof_of_possession = public0
		.generate_proof_of_possession()
		.expect("Cant generate proof_of_possession for ecdsa");
	assert!(public0.verify_proof_of_possession(&proof_of_possession));

	let signature = public0.sign(&"ecdsa").expect("Generates a valid `ecdsa` signature.");

	assert!(public0.verify(&"ecdsa", &signature));
	(signature, public0, proof_of_possession)
}

#[cfg(feature = "bls-experimental")]
fn test_bls381_crypto() -> (Bls381Pop, Bls381Public) {
	let mut public0 = bls381::AppPublic::generate_pair(None);

	let proof_of_possession = public0
		.generate_proof_of_possession()
		.expect("Cant generate proof_of_possession for bls381");
	assert!(public0.verify_proof_of_possession(&proof_of_possession));

	(proof_of_possession, public0)
}

#[cfg(feature = "bls-experimental")]
fn test_ecdsa_bls381_crypto() -> (EcdsaBls381Pop, EcdsaBls381Public) {
	let mut public0 = ecdsa_bls381::AppPublic::generate_pair(None);

	let proof_of_possession = public0
		.generate_proof_of_possession()
		.expect("Cant Generate proof_of_possession for ecdsa_bls381");
	assert!(public0.verify_proof_of_possession(&proof_of_possession));

	(proof_of_possession, public0)
}

fn test_read_storage() {
	const KEY: &[u8] = b":read_storage";
	sp_io::storage::set(KEY, b"test");

	let mut v = [0u8; 4];
	let r = sp_io::storage::read(KEY, &mut v, 0);
	assert_eq!(r, Some(4));
	assert_eq!(&v, b"test");

	let mut v = [0u8; 4];
	let r = sp_io::storage::read(KEY, &mut v, 4);
	assert_eq!(r, Some(0));
	assert_eq!(&v, &[0, 0, 0, 0]);
}

fn test_read_child_storage() {
	const STORAGE_KEY: &[u8] = b"unique_id_1";
	const KEY: &[u8] = b":read_child_storage";
	sp_io::default_child_storage::set(STORAGE_KEY, KEY, b"test");

	let mut v = [0u8; 4];
	let r = sp_io::default_child_storage::read(STORAGE_KEY, KEY, &mut v, 0);
	assert_eq!(r, Some(4));
	assert_eq!(&v, b"test");

	let mut v = [0u8; 4];
	let r = sp_io::default_child_storage::read(STORAGE_KEY, KEY, &mut v, 8);
	assert_eq!(r, Some(0));
	assert_eq!(&v, &[0, 0, 0, 0]);
}

fn test_witness(proof: StorageProof, root: crate::Hash) {
	use sp_externalities::Externalities;
	let db: sp_trie::MemoryDB<crate::Hashing> = proof.into_memory_db();
	let backend = sp_state_machine::TrieBackendBuilder::<_, crate::Hashing>::new(db, root).build();
	let mut overlay = sp_state_machine::OverlayedChanges::default();
	let mut ext = sp_state_machine::Ext::new(
		&mut overlay,
		&backend,
		#[cfg(feature = "std")]
		None,
	);
	assert!(ext.storage(b"value3").is_some());
	assert!(ext.storage_root(Default::default()).as_slice() == &root[..]);
	ext.place_storage(vec![0], Some(vec![1]));
	assert!(ext.storage_root(Default::default()).as_slice() != &root[..]);
}

/// Some tests require the hashed keys of the storage. As the values of hashed keys are not trivial
/// to guess, this small module provides the values of the keys, and the code which is required to
/// generate the keys.
#[cfg(feature = "std")]
pub mod storage_key_generator {
	use super::*;
	use sp_core::Pair;

	/// Generate hex string without prefix
	pub(super) fn hex<T>(x: T) -> String
	where
		T: array_bytes::Hex,
	{
		x.hex(Default::default())
	}

	fn concat_hashes(input: &Vec<&[u8]>) -> String {
		input.iter().map(|s| sp_crypto_hashing::twox_128(s)).map(hex).collect()
	}

	fn twox_64_concat(x: &[u8]) -> Vec<u8> {
		sp_crypto_hashing::twox_64(x).iter().chain(x.iter()).cloned().collect()
	}

	/// Generate the hashed storage keys from the raw literals. These keys are expected to be in
	/// storage with given substrate-test runtime.
	pub fn generate_expected_storage_hashed_keys(custom_heap_pages: bool) -> Vec<String> {
		let mut literals: Vec<&[u8]> = vec![b":code", b":extrinsic_index"];

		if custom_heap_pages {
			literals.push(b":heappages");
		}

		let keys: Vec<Vec<&[u8]>> = vec![
			vec![b"Babe", b":__STORAGE_VERSION__:"],
			vec![b"Babe", b"Authorities"],
			vec![b"Babe", b"EpochConfig"],
			vec![b"Babe", b"NextAuthorities"],
			vec![b"Babe", b"SegmentIndex"],
			vec![b"Balances", b":__STORAGE_VERSION__:"],
			vec![b"Balances", b"TotalIssuance"],
			vec![b"SubstrateTest", b":__STORAGE_VERSION__:"],
			vec![b"SubstrateTest", b"Authorities"],
			vec![b"System", b":__STORAGE_VERSION__:"],
			vec![b"System", b"LastRuntimeUpgrade"],
			vec![b"System", b"ParentHash"],
			vec![b"System", b"UpgradedToTripleRefCount"],
			vec![b"System", b"UpgradedToU32RefCount"],
		];

		let mut expected_keys = keys.iter().map(concat_hashes).collect::<Vec<String>>();
		expected_keys.extend(literals.into_iter().map(hex));

		let balances_map_keys = (0..16_usize)
			.into_iter()
			.map(|i| Sr25519Keyring::numeric(i).public().to_vec())
			.chain(vec![
				Sr25519Keyring::Alice.public().to_vec(),
				Sr25519Keyring::Bob.public().to_vec(),
				Sr25519Keyring::Charlie.public().to_vec(),
			])
			.map(|pubkey| {
				sp_crypto_hashing::blake2_128(&pubkey)
					.iter()
					.chain(pubkey.iter())
					.cloned()
					.collect::<Vec<u8>>()
			})
			.map(|hash_pubkey| {
				[concat_hashes(&vec![b"System", b"Account"]), hex(hash_pubkey)].concat()
			});

		expected_keys.extend(balances_map_keys);

		expected_keys.push(
			[
				concat_hashes(&vec![b"System", b"BlockHash"]),
				hex(0u64.using_encoded(twox_64_concat)),
			]
			.concat(),
		);

		expected_keys.sort();
		expected_keys
	}

	/// Provides the commented list of hashed keys. This contains a hard-coded list of hashed keys
	/// that would be generated by `generate_expected_storage_hashed_keys`. This list is provided
	/// for the debugging convenience only. Value of each hex-string is documented with the literal
	/// origin.
	///
	/// `custom_heap_pages`: Should be set to `true` when the state contains the `:heap_pages` key
	/// aka when overriding the heap pages to be used by the executor.
	pub fn get_expected_storage_hashed_keys(custom_heap_pages: bool) -> Vec<&'static str> {
		let mut res = vec![
			//SubstrateTest|:__STORAGE_VERSION__:
			"00771836bebdd29870ff246d305c578c4e7b9012096b41c4eb3aaf947f6ea429",
			//SubstrateTest|Authorities
			"00771836bebdd29870ff246d305c578c5e0621c4869aa60c02be9adcc98a0d1d",
			//Babe|:__STORAGE_VERSION__:
			"1cb6f36e027abb2091cfb5110ab5087f4e7b9012096b41c4eb3aaf947f6ea429",
			//Babe|Authorities
			"1cb6f36e027abb2091cfb5110ab5087f5e0621c4869aa60c02be9adcc98a0d1d",
			//Babe|SegmentIndex
			"1cb6f36e027abb2091cfb5110ab5087f66e8f035c8adbe7f1547b43c51e6f8a4",
			//Babe|NextAuthorities
			"1cb6f36e027abb2091cfb5110ab5087faacf00b9b41fda7a9268821c2a2b3e4c",
			//Babe|EpochConfig
			"1cb6f36e027abb2091cfb5110ab5087fdc6b171b77304263c292cc3ea5ed31ef",
			//System|:__STORAGE_VERSION__:
			"26aa394eea5630e07c48ae0c9558cef74e7b9012096b41c4eb3aaf947f6ea429",
			//System|UpgradedToU32RefCount
			"26aa394eea5630e07c48ae0c9558cef75684a022a34dd8bfa2baaf44f172b710",
			//System|ParentHash
			"26aa394eea5630e07c48ae0c9558cef78a42f33323cb5ced3b44dd825fda9fcc",
			//System::BlockHash|0
			"26aa394eea5630e07c48ae0c9558cef7a44704b568d21667356a5a050c118746bb1bdbcacd6ac9340000000000000000",
			//System|UpgradedToTripleRefCount
			"26aa394eea5630e07c48ae0c9558cef7a7fd6c28836b9a28522dc924110cf439",

			// System|Account|blake2_128Concat("//11")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da901cae4e3edfbb32c91ed3f01ab964f4eeeab50338d8e5176d3141802d7b010a55dadcd5f23cf8aaafa724627e967e90e",
			// System|Account|blake2_128Concat("//4")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da91b614bd4a126f2d5d294e9a8af9da25248d7e931307afb4b68d8d565d4c66e00d856c6d65f5fed6bb82dcfb60e936c67",
			// System|Account|blake2_128Concat("//7")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da94b21aff9fe1e8b2fc4b0775b8cbeff28ba8e2c7594dd74730f3ca835e95455d199261897edc9735d602ea29615e2b10b",
			// System|Account|blake2_128Concat("//Bob")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da94f9aea1afa791265fae359272badc1cf8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48",
			// System|Account|blake2_128Concat("//3")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da95786a2916fcb81e1bd5dcd81e0d2452884617f575372edb5a36d85c04cdf2e4699f96fe33eb5f94a28c041b88e398d0c",
			// System|Account|blake2_128Concat("//14")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da95b8542d9672c7b7e779cc7c1e6b605691c2115d06120ea2bee32dd601d02f36367564e7ddf84ae2717ca3f097459652e",
			// System|Account|blake2_128Concat("//6")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da996c30bdbfab640838e6b6d3c33ab4adb4211b79e34ee8072eab506edd4b93a7b85a14c9a05e5cdd056d98e7dbca87730",
			// System|Account|blake2_128Concat("//9")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da99dc65b1339ec388fbf2ca0cdef51253512c6cfd663203ea16968594f24690338befd906856c4d2f4ef32dad578dba20c",
			// System|Account|blake2_128Concat("//8")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da99e6eb5abd62f5fd54793da91a47e6af6125d57171ff9241f07acaa1bb6a6103517965cf2cd00e643b27e7599ebccba70",
			// System|Account|blake2_128Concat("//Charlie")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9b0edae20838083f2cde1c4080db8cf8090b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22",
			// System|Account|blake2_128Concat("//10")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9d0052993b6f3bd0544fd1f5e4125b9fbde3e789ecd53431fe5c06c12b72137153496dace35c695b5f4d7b41f7ed5763b",
			// System|Account|blake2_128Concat("//1")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9d6b7e9a5f12bc571053265dade10d3b4b606fc73f57f03cdb4c932d475ab426043e429cecc2ffff0d2672b0df8398c48",
			// System|Account|blake2_128Concat("//Alice")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9de1e86a9a8c739864cf3cc5ec2bea59fd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d",
			// System|Account|blake2_128Concat("//2")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9e1a35f56ee295d39287cbffcfc60c4b346f136b564e1fad55031404dd84e5cd3fa76bfe7cc7599b39d38fd06663bbc0a",
			// System|Account|blake2_128Concat("//5")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9e2c1dc507e2035edbbd8776c440d870460c57f0008067cc01c5ff9eb2e2f9b3a94299a915a91198bd1021a6c55596f57",
			// System|Account|blake2_128Concat("//0")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9eca0e653a94f4080f6311b4e7b6934eb2afba9278e30ccf6a6ceb3a8b6e336b70068f045c666f2e7f4f9cc5f47db8972",
			// System|Account|blake2_128Concat("//13")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9ee8bf7ef90fc56a8aa3b90b344c599550c29b161e27ff8ba45bf6bad4711f326fc506a8803453a4d7e3158e993495f10",
			// System|Account|blake2_128Concat("//12")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9f5d6f1c082fe63eec7a71fcad00f4a892e3d43b7b0d04e776e69e7be35247cecdac65504c579195731eaf64b7940966e",
			// System|Account|blake2_128Concat("//15")
			"26aa394eea5630e07c48ae0c9558cef7b99d880ec681799c0cf30e8886371da9fbf0818841edf110e05228a6379763c4fc3c37459d9bdc61f58a5ebc01e9e2305a19d390c0543dc733861ec3cf1de01f",
			// System|LastRuntimeUpgrade
			"26aa394eea5630e07c48ae0c9558cef7f9cce9c888469bb1a0dceaa129672ef8",
			// :code
			"3a636f6465",
			// :extrinsic_index
			"3a65787472696e7369635f696e646578",
			// Balances|:__STORAGE_VERSION__:
			"c2261276cc9d1f8598ea4b6a74b15c2f4e7b9012096b41c4eb3aaf947f6ea429",
			// Balances|TotalIssuance
			"c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80",
		];

		if custom_heap_pages {
			// :heappages
			res.push("3a686561707061676573");
		}

		res
	}

	#[test]
	fn expected_keys_vec_are_matching() {
		assert_eq!(
			storage_key_generator::get_expected_storage_hashed_keys(false),
			storage_key_generator::generate_expected_storage_hashed_keys(false),
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;
	use frame_support::dispatch::DispatchInfo;
	use sc_block_builder::BlockBuilderBuilder;
	use sp_api::{ApiExt, ProvideRuntimeApi};
	use sp_consensus::BlockOrigin;
	use sp_core::{storage::well_known_keys::HEAP_PAGES, traits::CallContext};
	use sp_runtime::{
		traits::{DispatchTransaction, Hash as _},
		transaction_validity::{InvalidTransaction, TransactionSource::External, ValidTransaction},
	};
	use substrate_test_runtime_client::{
		prelude::*, runtime::TestAPI, DefaultTestClientBuilderExt, TestClientBuilder,
	};

	#[test]
	fn heap_pages_is_respected() {
		// This tests that the on-chain `HEAP_PAGES` parameter is respected.

		// Create a client devoting only 8 pages of wasm memory. This gives us ~512k of heap memory.
		let client = TestClientBuilder::new().set_heap_pages(8).build();
		let best_hash = client.chain_info().best_hash;

		// Try to allocate 1024k of memory on heap. This is going to fail since it is twice larger
		// than the heap.
		let mut runtime_api = client.runtime_api();
		// This is currently required to allocate the 1024k of memory as configured above.
		runtime_api.set_call_context(CallContext::Onchain);
		let ret = runtime_api.vec_with_capacity(best_hash, 1048576);
		assert!(ret.is_err());

		// Create a block that sets the `:heap_pages` to 32 pages of memory which corresponds to
		// ~2048k of heap memory.
		let (new_at_hash, block) = {
			let mut builder = BlockBuilderBuilder::new(&client)
				.on_parent_block(best_hash)
				.with_parent_block_number(0)
				.build()
				.unwrap();
			builder.push_storage_change(HEAP_PAGES.to_vec(), Some(32u64.encode())).unwrap();
			let block = builder.build().unwrap().block;
			let hash = block.header.hash();
			(hash, block)
		};

		futures::executor::block_on(client.import(BlockOrigin::Own, block)).unwrap();

		// Allocation of 1024k while having ~2048k should succeed.
		let ret = client.runtime_api().vec_with_capacity(new_at_hash, 1048576);
		assert!(ret.is_ok());
	}

	#[test]
	fn test_storage() {
		let client = TestClientBuilder::new().build();
		let runtime_api = client.runtime_api();
		let best_hash = client.chain_info().best_hash;

		runtime_api.test_storage(best_hash).unwrap();
	}

	fn witness_backend() -> (sp_trie::MemoryDB<crate::Hashing>, crate::Hash) {
		let mut root = crate::Hash::default();
		let mut mdb = sp_trie::MemoryDB::<crate::Hashing>::default();
		{
			let mut trie =
				sp_trie::trie_types::TrieDBMutBuilderV1::new(&mut mdb, &mut root).build();
			trie.insert(b"value3", &[142]).expect("insert failed");
			trie.insert(b"value4", &[124]).expect("insert failed");
		};
		(mdb, root)
	}

	#[test]
	fn witness_backend_works() {
		let (db, root) = witness_backend();
		let backend =
			sp_state_machine::TrieBackendBuilder::<_, crate::Hashing>::new(db, root).build();
		let proof = sp_state_machine::prove_read(backend, vec![b"value3"]).unwrap();
		let client = TestClientBuilder::new().build();
		let runtime_api = client.runtime_api();
		let best_hash = client.chain_info().best_hash;

		runtime_api.test_witness(best_hash, proof, root).unwrap();
	}

	pub fn new_test_ext() -> sp_io::TestExternalities {
		genesismap::GenesisStorageBuilder::new(
			vec![Sr25519Keyring::One.public().into(), Sr25519Keyring::Two.public().into()],
			vec![Sr25519Keyring::One.into(), Sr25519Keyring::Two.into()],
			1000 * currency::DOLLARS,
		)
		.build()
		.into()
	}

	#[test]
	fn validate_storage_keys() {
		assert_eq!(
			genesismap::GenesisStorageBuilder::default()
				.build()
				.top
				.keys()
				.cloned()
				.map(storage_key_generator::hex)
				.collect::<Vec<_>>(),
			storage_key_generator::get_expected_storage_hashed_keys(false)
		);
	}

	#[test]
	fn validate_unsigned_works() {
		sp_tracing::try_init_simple();
		new_test_ext().execute_with(|| {
			let failing_calls = vec![
				substrate_test_pallet::Call::bench_call { transfer: Default::default() },
				substrate_test_pallet::Call::include_data { data: vec![] },
				substrate_test_pallet::Call::fill_block { ratio: Perbill::from_percent(50) },
			];
			let succeeding_calls = vec![
				substrate_test_pallet::Call::deposit_log_digest_item {
					log: DigestItem::Other(vec![]),
				},
				substrate_test_pallet::Call::storage_change { key: vec![], value: None },
				substrate_test_pallet::Call::read { count: 0 },
				substrate_test_pallet::Call::read_and_panic { count: 0 },
			];

			for call in failing_calls {
				assert_eq!(
					<SubstrateTest as sp_runtime::traits::ValidateUnsigned>::validate_unsigned(
						TransactionSource::External,
						&call,
					),
					InvalidTransaction::Call.into(),
				);
			}

			for call in succeeding_calls {
				assert_eq!(
					<SubstrateTest as sp_runtime::traits::ValidateUnsigned>::validate_unsigned(
						TransactionSource::External,
						&call,
					),
					Ok(ValidTransaction {
						provides: vec![BlakeTwo256::hash_of(&call).encode()],
						..Default::default()
					})
				);
			}
		});
	}

	#[test]
	fn check_substrate_check_signed_extension_works() {
		sp_tracing::try_init_simple();
		new_test_ext().execute_with(|| {
			let x = Sr25519Keyring::Alice.into();
			let info = DispatchInfo::default();
			let len = 0_usize;
			assert_eq!(
				CheckSubstrateCall {}
					.validate_only(
						Some(x).into(),
						&ExtrinsicBuilder::new_call_with_priority(16).build().function,
						&info,
						len,
						External,
						0,
					)
					.unwrap()
					.0
					.priority,
				16
			);

			assert_eq!(
				CheckSubstrateCall {}
					.validate_only(
						Some(x).into(),
						&ExtrinsicBuilder::new_call_do_not_propagate().build().function,
						&info,
						len,
						External,
						0,
					)
					.unwrap()
					.0
					.propagate,
				false
			);
		})
	}

	mod genesis_builder_tests {
		use super::*;
		use crate::genesismap::GenesisStorageBuilder;
		use sc_executor::{error::Result, WasmExecutor};
		use sc_executor_common::runtime_blob::RuntimeBlob;
		use serde_json::json;
		use sp_application_crypto::Ss58Codec;
		use sp_core::traits::Externalities;
		use sp_genesis_builder::Result as BuildResult;
		use sp_state_machine::BasicExternalities;
		use std::{fs, io::Write};
		use storage_key_generator::hex;

		pub fn executor_call(
			ext: &mut dyn Externalities,
			method: &str,
			data: &[u8],
		) -> Result<Vec<u8>> {
			let executor = WasmExecutor::<sp_io::SubstrateHostFunctions>::builder().build();
			executor.uncached_call(
				RuntimeBlob::uncompress_if_needed(wasm_binary_unwrap()).unwrap(),
				ext,
				true,
				method,
				data,
			)
		}

		#[test]
		fn build_minimal_genesis_config_works() {
			sp_tracing::try_init_simple();
			let default_minimal_json = r#"{"system":{},"babe":{"authorities":[],"epochConfig":{"c": [ 3, 10 ],"allowed_slots":"PrimaryAndSecondaryPlainSlots"}},"substrateTest":{"authorities":[]},"balances":{"balances":[]}}"#;
			let mut t = BasicExternalities::new_empty();

			executor_call(&mut t, "GenesisBuilder_build_state", &default_minimal_json.encode())
				.unwrap();

			let mut keys = t.into_storages().top.keys().cloned().map(hex).collect::<Vec<String>>();
			keys.sort();

			let mut expected = [
				//SubstrateTest|Authorities
				"00771836bebdd29870ff246d305c578c5e0621c4869aa60c02be9adcc98a0d1d",
				//Babe|SegmentIndex
				"1cb6f36e027abb2091cfb5110ab5087f66e8f035c8adbe7f1547b43c51e6f8a4",
				//Babe|EpochConfig
				"1cb6f36e027abb2091cfb5110ab5087fdc6b171b77304263c292cc3ea5ed31ef",
				//System|UpgradedToU32RefCount
				"26aa394eea5630e07c48ae0c9558cef75684a022a34dd8bfa2baaf44f172b710",
				//System|ParentHash
				"26aa394eea5630e07c48ae0c9558cef78a42f33323cb5ced3b44dd825fda9fcc",
				//System::BlockHash|0
				"26aa394eea5630e07c48ae0c9558cef7a44704b568d21667356a5a050c118746bb1bdbcacd6ac9340000000000000000",
				//System|UpgradedToTripleRefCount
				"26aa394eea5630e07c48ae0c9558cef7a7fd6c28836b9a28522dc924110cf439",

				// System|LastRuntimeUpgrade
				"26aa394eea5630e07c48ae0c9558cef7f9cce9c888469bb1a0dceaa129672ef8",
				// :extrinsic_index
				"3a65787472696e7369635f696e646578",
				// Balances|TotalIssuance
				"c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80",

				// added by on_genesis:
				// Balances|:__STORAGE_VERSION__:
				"c2261276cc9d1f8598ea4b6a74b15c2f4e7b9012096b41c4eb3aaf947f6ea429",
				//System|:__STORAGE_VERSION__:
				"26aa394eea5630e07c48ae0c9558cef74e7b9012096b41c4eb3aaf947f6ea429",
				//Babe|:__STORAGE_VERSION__:
				"1cb6f36e027abb2091cfb5110ab5087f4e7b9012096b41c4eb3aaf947f6ea429",
				//SubstrateTest|:__STORAGE_VERSION__:
				"00771836bebdd29870ff246d305c578c4e7b9012096b41c4eb3aaf947f6ea429",
				].into_iter().map(String::from).collect::<Vec<_>>();
			expected.sort();

			assert_eq!(keys, expected);
		}

		#[test]
		fn default_config_as_json_works() {
			sp_tracing::try_init_simple();
			let mut t = BasicExternalities::new_empty();
			let r = executor_call(&mut t, "GenesisBuilder_get_preset", &None::<&PresetId>.encode())
				.unwrap();
			let r = Option::<Vec<u8>>::decode(&mut &r[..])
				.unwrap()
				.expect("default config is there");
			let json = String::from_utf8(r.into()).expect("returned value is json. qed.");

			let expected = r#"{"system":{},"babe":{"authorities":[],"epochConfig":{"c":[1,4],"allowed_slots":"PrimaryAndSecondaryVRFSlots"}},"substrateTest":{"authorities":[]},"balances":{"balances":[],"devAccounts":null}}"#;
			assert_eq!(expected.to_string(), json);
		}

		#[test]
		fn preset_names_listing_works() {
			sp_tracing::try_init_simple();
			let mut t = BasicExternalities::new_empty();
			let r = executor_call(&mut t, "GenesisBuilder_preset_names", &vec![]).unwrap();
			let r = Vec::<PresetId>::decode(&mut &r[..]).unwrap();
			assert_eq!(r, vec![PresetId::from("foobar"), PresetId::from("staging"),]);
			log::info!("r: {:#?}", r);
		}

		#[test]
		fn named_config_works() {
			sp_tracing::try_init_simple();
			let f = |cfg_name: &str, expected: &str| {
				let mut t = BasicExternalities::new_empty();
				let name = cfg_name.to_string();
				let r = executor_call(
					&mut t,
					"GenesisBuilder_get_preset",
					&Some(name.as_bytes()).encode(),
				)
				.unwrap();
				let r = Option::<Vec<u8>>::decode(&mut &r[..]).unwrap();
				let json =
					String::from_utf8(r.unwrap().into()).expect("returned value is json. qed.");
				log::info!("json: {:#?}", json);
				assert_eq!(expected.to_string(), json);
			};

			f("foobar", r#"{"foo":"bar"}"#);
			f(
				"staging",
				r#"{"balances":{"balances":[["5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",1000000000000000],["5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y",1000000000000000]]},"substrateTest":{"authorities":["5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY","5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL"]}}"#,
			);
		}

		#[test]
		fn build_config_from_json_works() {
			sp_tracing::try_init_simple();
			let j = include_str!("../res/default_genesis_config.json");

			let mut t = BasicExternalities::new_empty();
			let r = executor_call(&mut t, "GenesisBuilder_build_state", &j.encode()).unwrap();
			let r = BuildResult::decode(&mut &r[..]);
			assert!(r.is_ok());

			let mut keys = t.into_storages().top.keys().cloned().map(hex).collect::<Vec<String>>();

			// following keys are not placed during `<RuntimeGenesisConfig as GenesisBuild>::build`
			// process, add them `keys` to assert against known keys.
			keys.push(hex(b":code"));
			keys.sort();

			assert_eq!(keys, storage_key_generator::get_expected_storage_hashed_keys(false));
		}

		#[test]
		fn build_config_from_invalid_json_fails() {
			sp_tracing::try_init_simple();
			let j = include_str!("../res/default_genesis_config_invalid.json");
			let mut t = BasicExternalities::new_empty();
			let r = executor_call(&mut t, "GenesisBuilder_build_state", &j.encode()).unwrap();
			let r = BuildResult::decode(&mut &r[..]).unwrap();
			log::info!("result: {:#?}", r);
			assert_eq!(r, Err(
				"Invalid JSON blob: unknown field `renamed_authorities`, expected `authorities` or `epochConfig` at line 4 column 25".to_string(),
			));
		}

		#[test]
		fn build_config_from_invalid_json_fails_2() {
			sp_tracing::try_init_simple();
			let j = include_str!("../res/default_genesis_config_invalid_2.json");
			let mut t = BasicExternalities::new_empty();
			let r = executor_call(&mut t, "GenesisBuilder_build_state", &j.encode()).unwrap();
			let r = BuildResult::decode(&mut &r[..]).unwrap();
			assert_eq!(r, Err(
				"Invalid JSON blob: unknown field `babex`, expected one of `system`, `babe`, `substrateTest`, `balances` at line 3 column 9".to_string(),
			));
		}

		#[test]
		fn build_config_from_incomplete_json_fails() {
			sp_tracing::try_init_simple();
			let j = include_str!("../res/default_genesis_config_incomplete.json");

			let mut t = BasicExternalities::new_empty();
			let r = executor_call(&mut t, "GenesisBuilder_build_state", &j.encode()).unwrap();
			let r = core::result::Result::<(), String>::decode(&mut &r[..]).unwrap();
			assert_eq!(
				r,
				Err("Invalid JSON blob: missing field `authorities` at line 11 column 3"
					.to_string())
			);
		}

		#[test]
		fn write_default_config_to_tmp_file() {
			if std::env::var("WRITE_DEFAULT_JSON_FOR_STR_GC").is_ok() {
				sp_tracing::try_init_simple();
				let mut file = fs::OpenOptions::new()
					.create(true)
					.write(true)
					.open("/tmp/default_genesis_config.json")
					.unwrap();

				let j = serde_json::to_string(&GenesisStorageBuilder::default().genesis_config())
					.unwrap()
					.into_bytes();
				file.write_all(&j).unwrap();
			}
		}

		#[test]
		fn build_genesis_config_with_patch_json_works() {
			//this tests shows how to do patching on native side
			sp_tracing::try_init_simple();

			let mut t = BasicExternalities::new_empty();
			let r = executor_call(&mut t, "GenesisBuilder_get_preset", &None::<&PresetId>.encode())
				.unwrap();
			let r = Option::<Vec<u8>>::decode(&mut &r[..])
				.unwrap()
				.expect("default config is there");
			let mut default_config: serde_json::Value =
				serde_json::from_slice(&r[..]).expect("returned value is json. qed.");

			// Patch default json with some custom values:
			let patch = json!({
				"babe": {
					"epochConfig": {
						"c": [
							7,
							10
						],
						"allowed_slots": "PrimaryAndSecondaryPlainSlots"
					}
				},
				"substrateTest": {
					"authorities": [
						Sr25519Keyring::Ferdie.public().to_ss58check(),
						Sr25519Keyring::Alice.public().to_ss58check()
					],
				}
			});

			sc_chain_spec::json_merge(&mut default_config, patch);

			// Build genesis config using custom json:
			let mut t = BasicExternalities::new_empty();
			executor_call(
				&mut t,
				"GenesisBuilder_build_state",
				&default_config.to_string().encode(),
			)
			.unwrap();

			// Ensure that custom values are in the genesis storage:
			let storage = t.into_storages();
			let get_from_storage = |key: &str| -> Vec<u8> {
				storage.top.get(&array_bytes::hex2bytes(key).unwrap()).unwrap().clone()
			};

			//SubstrateTest|Authorities
			let value: Vec<u8> = get_from_storage(
				"00771836bebdd29870ff246d305c578c5e0621c4869aa60c02be9adcc98a0d1d",
			);
			let authority_key_vec =
				Vec::<sp_core::sr25519::Public>::decode(&mut &value[..]).unwrap();
			assert_eq!(authority_key_vec.len(), 2);
			assert_eq!(authority_key_vec[0], Sr25519Keyring::Ferdie.public());
			assert_eq!(authority_key_vec[1], Sr25519Keyring::Alice.public());

			//Babe|Authorities
			let value: Vec<u8> = get_from_storage(
				"1cb6f36e027abb2091cfb5110ab5087fdc6b171b77304263c292cc3ea5ed31ef",
			);
			assert_eq!(
				BabeEpochConfiguration::decode(&mut &value[..]).unwrap(),
				BabeEpochConfiguration {
					c: (7, 10),
					allowed_slots: AllowedSlots::PrimaryAndSecondaryPlainSlots
				}
			);

			// Ensure that some values are default ones:
			// Balances|TotalIssuance
			let value: Vec<u8> = get_from_storage(
				"c2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c80",
			);
			assert_eq!(u64::decode(&mut &value[..]).unwrap(), 0);

			//System|ParentHash
			let value: Vec<u8> = get_from_storage(
				"26aa394eea5630e07c48ae0c9558cef78a42f33323cb5ced3b44dd825fda9fcc",
			);
			assert_eq!(H256::decode(&mut &value[..]).unwrap(), [69u8; 32].into());
		}
	}
}
