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

//! The crate's tests.

use super::*;
use crate::{self as pallet_referenda, types::Track};
use alloc::borrow::Cow;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	assert_ok, derive_impl, ord_parameter_types, parameter_types,
	traits::{
		ConstU32, ConstU64, Contains, EqualPrivilegeOnly, OnInitialize, OriginTrait, Polling,
	},
	weights::Weight,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use sp_runtime::{
	str_array as s,
	traits::{BlakeTwo256, Hash},
	BuildStorage, DispatchResult, Perbill,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Preimage: pallet_preimage,
		Scheduler: pallet_scheduler,
		Referenda: pallet_referenda,
	}
);

// Test that a fitlered call can be dispatched.
pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(call: &RuntimeCall) -> bool {
		!matches!(call, &RuntimeCall::Balances(pallet_balances::Call::force_set_balance { .. }))
	}
}

parameter_types! {
	pub MaxWeight: Weight = Weight::from_parts(2_000_000_000_000, u64::MAX);
}
#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = BaseFilter;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
}
impl pallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<u64>;
	type Consideration = ();
}
impl pallet_scheduler::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaxWeight;
	type ScheduleOrigin = EnsureRoot<u64>;
	type MaxScheduledPerBlock = ConstU32<100>;
	type WeightInfo = ();
	type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
	type BlockNumberProvider = frame_system::Pallet<Test>;
}
#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}
parameter_types! {
	pub static AlarmInterval: u64 = 1;
}
ord_parameter_types! {
	pub const One: u64 = 1;
	pub const Two: u64 = 2;
	pub const Three: u64 = 3;
	pub const Four: u64 = 4;
	pub const Five: u64 = 5;
	pub const Six: u64 = 6;
}

pub struct TestTracksInfo;
impl TracksInfo<u64, u64> for TestTracksInfo {
	type Id = u8;
	type RuntimeOrigin = <RuntimeOrigin as OriginTrait>::PalletsOrigin;

	fn tracks() -> impl Iterator<Item = Cow<'static, Track<Self::Id, u64, u64>>> {
		static DATA: [Track<u8, u64, u64>; 3] = [
			Track {
				id: 0u8,
				info: TrackInfo {
					name: s("root"),
					max_deciding: 1,
					decision_deposit: 10,
					prepare_period: 4,
					decision_period: 4,
					confirm_period: 2,
					min_enactment_period: 4,
					min_approval: Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(50),
						ceil: Perbill::from_percent(100),
					},
					min_support: Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(0),
						ceil: Perbill::from_percent(100),
					},
				},
			},
			Track {
				id: 1u8,
				info: TrackInfo {
					name: s("none"),
					max_deciding: 3,
					decision_deposit: 1,
					prepare_period: 2,
					decision_period: 2,
					confirm_period: 1,
					min_enactment_period: 2,
					min_approval: Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(95),
						ceil: Perbill::from_percent(100),
					},
					min_support: Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(90),
						ceil: Perbill::from_percent(100),
					},
				},
			},
			Track {
				id: 2u8,
				info: TrackInfo {
					name: s("none"),
					max_deciding: 3,
					decision_deposit: 1,
					prepare_period: 2,
					decision_period: 2,
					confirm_period: 1,
					min_enactment_period: 0,
					min_approval: Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(95),
						ceil: Perbill::from_percent(100),
					},
					min_support: Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(90),
						ceil: Perbill::from_percent(100),
					},
				},
			},
		];
		DATA.iter().map(Cow::Borrowed)
	}
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		if let Ok(system_origin) = frame_system::RawOrigin::try_from(id.clone()) {
			match system_origin {
				frame_system::RawOrigin::Root => Ok(0),
				frame_system::RawOrigin::None => Ok(1),
				frame_system::RawOrigin::Signed(1) => Ok(2),
				_ => Err(()),
			}
		} else {
			Err(())
		}
	}
}

impl Config for Test {
	type WeightInfo = ();
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = pallet_balances::Pallet<Self>;
	type SubmitOrigin = frame_system::EnsureSigned<u64>;
	type CancelOrigin = EnsureSignedBy<Four, u64>;
	type KillOrigin = EnsureRoot<u64>;
	type Slash = ();
	type Votes = u32;
	type Tally = Tally;
	type SubmissionDeposit = ConstU64<2>;
	type MaxQueued = ConstU32<3>;
	type UndecidingTimeout = ConstU64<20>;
	type AlarmInterval = AlarmInterval;
	type Tracks = TestTracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}
pub struct ExtBuilder {}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self {}
	}
}

impl ExtBuilder {
	pub fn build(self) -> sp_io::TestExternalities {
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		let balances = vec![(1, 100), (2, 100), (3, 100), (4, 100), (5, 100), (6, 100)];
		pallet_balances::GenesisConfig::<Test> { balances, ..Default::default() }
			.assimilate_storage(&mut t)
			.unwrap();
		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}

	pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
		self.build().execute_with(|| {
			test();
			Referenda::do_try_state().unwrap();
		})
	}
}

#[derive(Encode, Debug, Decode, TypeInfo, Eq, PartialEq, Clone, MaxEncodedLen)]
pub struct Tally {
	pub ayes: u32,
	pub nays: u32,
}

impl<Class> VoteTally<u32, Class> for Tally {
	fn new(_: Class) -> Self {
		Self { ayes: 0, nays: 0 }
	}

	fn ayes(&self, _: Class) -> u32 {
		self.ayes
	}

	fn support(&self, _: Class) -> Perbill {
		Perbill::from_percent(self.ayes)
	}

	fn approval(&self, _: Class) -> Perbill {
		if self.ayes + self.nays > 0 {
			Perbill::from_rational(self.ayes, self.ayes + self.nays)
		} else {
			Perbill::zero()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn unanimity(_: Class) -> Self {
		Self { ayes: 100, nays: 0 }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn rejection(_: Class) -> Self {
		Self { ayes: 0, nays: 100 }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn from_requirements(support: Perbill, approval: Perbill, _: Class) -> Self {
		let ayes = support.mul_ceil(100u32);
		let nays = ((ayes as u64) * 1_000_000_000u64 / approval.deconstruct() as u64) as u32 - ayes;
		Self { ayes, nays }
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn setup(_: Class, _: Perbill) {}
}

pub fn set_balance_proposal(value: u64) -> Vec<u8> {
	RuntimeCall::Balances(pallet_balances::Call::force_set_balance { who: 42, new_free: value })
		.encode()
}

pub fn set_balance_proposal_bounded(value: u64) -> BoundedCallOf<Test, ()> {
	let c = RuntimeCall::Balances(pallet_balances::Call::force_set_balance {
		who: 42,
		new_free: value,
	});
	<Preimage as StorePreimage>::bound(c).unwrap()
}

#[allow(dead_code)]
pub fn propose_set_balance(who: u64, value: u64, delay: u64) -> DispatchResult {
	Referenda::submit(
		RuntimeOrigin::signed(who),
		Box::new(frame_system::RawOrigin::Root.into()),
		set_balance_proposal_bounded(value),
		DispatchTime::After(delay),
	)
}

pub fn next_block() {
	System::set_block_number(System::block_number() + 1);
	Scheduler::on_initialize(System::block_number());
}

pub fn run_to(n: u64) {
	while System::block_number() < n {
		next_block();
	}
}

#[allow(dead_code)]
pub fn begin_referendum() -> ReferendumIndex {
	System::set_block_number(0);
	assert_ok!(propose_set_balance(1, 2, 1));
	run_to(2);
	0
}

#[allow(dead_code)]
pub fn tally(r: ReferendumIndex) -> Tally {
	Referenda::ensure_ongoing(r).unwrap().tally
}

pub fn set_tally(index: ReferendumIndex, ayes: u32, nays: u32) {
	<Referenda as Polling<Tally>>::access_poll(index, |status| {
		let tally = status.ensure_ongoing().unwrap().0;
		tally.ayes = ayes;
		tally.nays = nays;
	});
}

pub fn waiting_since(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::Ongoing(ReferendumStatus { submitted, deciding: None, .. }) => submitted,
		_ => panic!("Not waiting"),
	}
}

pub fn deciding_since(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::Ongoing(ReferendumStatus {
			deciding: Some(DecidingStatus { since, .. }),
			..
		}) => since,
		_ => panic!("Not deciding"),
	}
}

pub fn deciding_and_failing_since(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::Ongoing(ReferendumStatus {
			deciding: Some(DecidingStatus { since, confirming: None, .. }),
			..
		}) => since,
		_ => panic!("Not deciding"),
	}
}

pub fn confirming_until(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::Ongoing(ReferendumStatus {
			deciding: Some(DecidingStatus { confirming: Some(until), .. }),
			..
		}) => until,
		_ => panic!("Not confirming"),
	}
}

pub fn approved_since(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::Approved(since, ..) => since,
		_ => panic!("Not approved"),
	}
}

pub fn rejected_since(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::Rejected(since, ..) => since,
		_ => panic!("Not rejected"),
	}
}

pub fn cancelled_since(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::Cancelled(since, ..) => since,
		_ => panic!("Not cancelled"),
	}
}

pub fn killed_since(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::Killed(since, ..) => since,
		_ => panic!("Not killed"),
	}
}

pub fn timed_out_since(i: ReferendumIndex) -> u64 {
	match ReferendumInfoFor::<Test>::get(i).unwrap() {
		ReferendumInfo::TimedOut(since, ..) => since,
		_ => panic!("Not timed out"),
	}
}

fn is_deciding(i: ReferendumIndex) -> bool {
	matches!(
		ReferendumInfoFor::<Test>::get(i),
		Some(ReferendumInfo::Ongoing(ReferendumStatus { deciding: Some(_), .. }))
	)
}

#[derive(Clone, Copy)]
pub enum RefState {
	Failing,
	Passing,
	Confirming { immediate: bool },
}

impl RefState {
	pub fn create(self) -> ReferendumIndex {
		assert_ok!(Referenda::submit(
			RuntimeOrigin::signed(1),
			Box::new(frame_support::dispatch::RawOrigin::Root.into()),
			set_balance_proposal_bounded(1),
			DispatchTime::At(10),
		));
		assert_ok!(Referenda::place_decision_deposit(RuntimeOrigin::signed(2), 0));
		if matches!(self, RefState::Confirming { immediate: true }) {
			set_tally(0, 100, 0);
		}
		let index = ReferendumCount::<Test>::get() - 1;
		while !is_deciding(index) {
			run_to(System::block_number() + 1);
		}
		if matches!(self, RefState::Confirming { immediate: false }) {
			set_tally(0, 100, 0);
			run_to(System::block_number() + 1);
		}
		if matches!(self, RefState::Confirming { .. }) {
			assert_eq!(confirming_until(index), System::block_number() + 2);
		}
		if matches!(self, RefState::Passing) {
			set_tally(0, 100, 99);
			run_to(System::block_number() + 1);
		}
		index
	}
}

/// note a new preimage without registering.
pub fn note_preimage(who: u64) -> <Test as frame_system::Config>::Hash {
	use std::sync::atomic::{AtomicU8, Ordering};
	// note a new preimage on every function invoke.
	static COUNTER: AtomicU8 = AtomicU8::new(0);
	let data = vec![COUNTER.fetch_add(1, Ordering::Relaxed)];
	assert_ok!(Preimage::note_preimage(RuntimeOrigin::signed(who), data.clone()));
	let hash = BlakeTwo256::hash(&data);
	assert!(!Preimage::is_requested(&hash));
	hash
}
