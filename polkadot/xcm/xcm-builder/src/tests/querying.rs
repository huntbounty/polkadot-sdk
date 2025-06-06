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

use super::*;

#[test]
fn pallet_query_should_work() {
	AllowUnpaidFrom::set(vec![[Parachain(1)].into()]);
	// They want to transfer 100 of our native asset from sovereign account of parachain #1 into #2
	// and let them know to hand it to account #3.
	let message = Xcm(vec![QueryPallet {
		module_name: "Error".into(),
		response_info: QueryResponseInfo {
			destination: Parachain(1).into(),
			query_id: 1,
			max_weight: Weight::from_parts(50, 50),
		},
	}]);
	let mut hash = fake_message_hash(&message);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Parachain(1),
		message,
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(r, Outcome::Complete { used: Weight::from_parts(10, 10) });

	let expected_msg = Xcm::<()>(vec![
		QueryResponse {
			query_id: 1,
			max_weight: Weight::from_parts(50, 50),
			response: Response::PalletsInfo(Default::default()),
			querier: Some(Here.into()),
		},
		SetTopic(hash),
	]);
	assert_eq!(sent_xcm(), vec![(Parachain(1).into(), expected_msg, hash)]);
}

#[test]
fn pallet_query_with_results_should_work() {
	AllowUnpaidFrom::set(vec![[Parachain(1)].into()]);
	// They want to transfer 100 of our native asset from sovereign account of parachain #1 into #2
	// and let them know to hand it to account #3.
	let message = Xcm(vec![QueryPallet {
		module_name: "pallet_balances".into(),
		response_info: QueryResponseInfo {
			destination: Parachain(1).into(),
			query_id: 1,
			max_weight: Weight::from_parts(50, 50),
		},
	}]);
	let mut hash = fake_message_hash(&message);
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Parachain(1),
		message,
		&mut hash,
		Weight::from_parts(50, 50),
		Weight::zero(),
	);
	assert_eq!(r, Outcome::Complete { used: Weight::from_parts(10, 10) });

	let expected_msg = Xcm::<()>(vec![
		QueryResponse {
			query_id: 1,
			max_weight: Weight::from_parts(50, 50),
			response: Response::PalletsInfo(
				vec![PalletInfo::new(
					1,
					b"Balances".as_ref().into(),
					b"pallet_balances".as_ref().into(),
					1,
					42,
					69,
				)
				.unwrap()]
				.try_into()
				.unwrap(),
			),
			querier: Some(Here.into()),
		},
		SetTopic(hash),
	]);
	assert_eq!(sent_xcm(), vec![(Parachain(1).into(), expected_msg, hash)]);
}

#[test]
fn prepaid_result_of_query_should_get_free_execution() {
	let query_id = 33;
	// We put this in manually here, but normally this would be done at the point of crafting the
	// message.
	expect_response(query_id, Parent.into());

	let the_response = Response::Assets((Parent, 100u128).into());
	let message = Xcm::<TestCall>(vec![QueryResponse {
		query_id,
		response: the_response.clone(),
		max_weight: Weight::from_parts(10, 10),
		querier: Some(Here.into()),
	}]);
	let mut hash = fake_message_hash(&message);
	let weight_limit = Weight::from_parts(10, 10);

	// First time the response gets through since we're expecting it...
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Parent,
		message.clone(),
		&mut hash,
		weight_limit,
		Weight::zero(),
	);
	assert_eq!(r, Outcome::Complete { used: Weight::from_parts(10, 10) });
	assert_eq!(response(query_id).unwrap(), the_response);

	// Second time it doesn't, since we're not.
	let r = XcmExecutor::<TestConfig>::prepare_and_execute(
		Parent,
		message.clone(),
		&mut hash,
		weight_limit,
		Weight::zero(),
	);
	assert_eq!(
		r,
		Outcome::Incomplete {
			used: Weight::from_parts(10, 10),
			error: InstructionError { index: 0, error: XcmError::Barrier },
		}
	);
}
