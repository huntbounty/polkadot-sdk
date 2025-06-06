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

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use futures::{future, StreamExt};
use kitchensink_runtime::{constants::currency::*, BalancesCall, SudoCall};
use node_cli::service::{create_extrinsic, fetch_nonce, FullClient, TransactionPool};
use node_primitives::AccountId;
use polkadot_sdk::{
	sc_service::config::{ExecutorConfiguration, RpcConfiguration},
	sc_transaction_pool_api::TransactionPool as _,
	*,
};
use sc_service::{
	config::{
		BlocksPruning, DatabaseSource, KeystoreConfig, NetworkConfiguration, OffchainWorkerConfig,
		PruningMode, RpcBatchRequestConfig, TransactionPoolOptions,
	},
	BasePath, Configuration, Role,
};
use sc_transaction_pool_api::{TransactionSource, TransactionStatus};
use sp_core::{crypto::Pair, sr25519};
use sp_keyring::Sr25519Keyring;
use sp_runtime::OpaqueExtrinsic;
use staging_node_cli as node_cli;
use tokio::runtime::Handle;

fn new_node(tokio_handle: Handle) -> node_cli::service::NewFullBase {
	let base_path = BasePath::new_temp_dir().expect("Creates base path");
	let root = base_path.path().to_path_buf();

	let network_config = NetworkConfiguration::new(
		Sr25519Keyring::Alice.to_seed(),
		"network/test/0.1",
		Default::default(),
		None,
	);

	let spec = Box::new(node_cli::chain_spec::development_config());

	let config = Configuration {
		impl_name: "BenchmarkImpl".into(),
		impl_version: "1.0".into(),
		role: Role::Authority,
		tokio_handle: tokio_handle.clone(),
		transaction_pool: TransactionPoolOptions::new_for_benchmarks(),
		network: network_config,
		keystore: KeystoreConfig::InMemory,
		database: DatabaseSource::RocksDb { path: root.join("db"), cache_size: 128 },
		trie_cache_maximum_size: Some(64 * 1024 * 1024),
		warm_up_trie_cache: None,
		state_pruning: Some(PruningMode::ArchiveAll),
		blocks_pruning: BlocksPruning::KeepAll,
		chain_spec: spec,
		executor: ExecutorConfiguration::default(),
		rpc: RpcConfiguration {
			addr: None,
			max_connections: Default::default(),
			cors: None,
			methods: Default::default(),
			max_request_size: Default::default(),
			max_response_size: Default::default(),
			id_provider: Default::default(),
			max_subs_per_conn: Default::default(),
			port: 9944,
			message_buffer_capacity: Default::default(),
			batch_config: RpcBatchRequestConfig::Unlimited,
			rate_limit: None,
			rate_limit_whitelisted_ips: Default::default(),
			rate_limit_trust_proxy_headers: Default::default(),
		},
		prometheus_config: None,
		telemetry_endpoints: None,
		offchain_worker: OffchainWorkerConfig { enabled: true, indexing_enabled: false },
		force_authoring: false,
		disable_grandpa: false,
		dev_key_seed: Some(Sr25519Keyring::Alice.to_seed()),
		tracing_targets: None,
		tracing_receiver: Default::default(),
		announce_block: true,
		data_path: base_path.path().into(),
		base_path,
		wasm_runtime_overrides: None,
	};

	tokio_handle.block_on(async move {
		node_cli::service::new_full_base::<sc_network::NetworkWorker<_, _>>(
			config,
			None,
			false,
			|_, _| (),
		)
		.expect("Creates node")
	})
}

fn create_accounts(num: usize) -> Vec<sr25519::Pair> {
	(0..num)
		.map(|i| {
			Pair::from_string(&format!("{}/{}", Sr25519Keyring::Alice.to_seed(), i), None)
				.expect("Creates account pair")
		})
		.collect()
}

/// Create the extrinsics that will initialize the accounts from the sudo account (Alice).
///
/// `start_nonce` is the current nonce of Alice.
fn create_account_extrinsics(
	client: &FullClient,
	accounts: &[sr25519::Pair],
) -> Vec<OpaqueExtrinsic> {
	let start_nonce = fetch_nonce(client, Sr25519Keyring::Alice.pair());

	accounts
		.iter()
		.enumerate()
		.flat_map(|(i, a)| {
			vec![
				// Reset the nonce by removing any funds
				create_extrinsic(
					client,
					Sr25519Keyring::Alice.pair(),
					SudoCall::sudo {
						call: Box::new(
							BalancesCall::force_set_balance {
								who: AccountId::from(a.public()).into(),
								new_free: 0,
							}
							.into(),
						),
					},
					Some(start_nonce + (i as u32) * 2),
				),
				// Give back funds
				create_extrinsic(
					client,
					Sr25519Keyring::Alice.pair(),
					SudoCall::sudo {
						call: Box::new(
							BalancesCall::force_set_balance {
								who: AccountId::from(a.public()).into(),
								new_free: 1_000_000 * DOLLARS,
							}
							.into(),
						),
					},
					Some(start_nonce + (i as u32) * 2 + 1),
				),
			]
		})
		.map(OpaqueExtrinsic::from)
		.collect()
}

fn create_benchmark_extrinsics(
	client: &FullClient,
	accounts: &[sr25519::Pair],
	extrinsics_per_account: usize,
) -> Vec<OpaqueExtrinsic> {
	accounts
		.iter()
		.flat_map(|account| {
			(0..extrinsics_per_account).map(move |nonce| {
				create_extrinsic(
					client,
					account.clone(),
					BalancesCall::transfer_allow_death {
						dest: Sr25519Keyring::Bob.to_account_id().into(),
						value: 1 * DOLLARS,
					},
					Some(nonce as u32),
				)
			})
		})
		.map(OpaqueExtrinsic::from)
		.collect()
}

async fn submit_tx_and_wait_for_inclusion(
	tx_pool: &TransactionPool,
	tx: OpaqueExtrinsic,
	client: &FullClient,
	wait_for_finalized: bool,
) {
	let best_hash = client.chain_info().best_hash;

	let mut watch = tx_pool
		.submit_and_watch(best_hash, TransactionSource::External, tx.clone())
		.await
		.expect("Submits tx to pool")
		.fuse();

	loop {
		match watch.select_next_some().await {
			TransactionStatus::Finalized(_) => break,
			TransactionStatus::InBlock(_) if !wait_for_finalized => break,
			_ => {},
		}
	}
}

fn transaction_pool_benchmarks(c: &mut Criterion) {
	sp_tracing::try_init_simple();

	let runtime = tokio::runtime::Runtime::new().expect("Creates tokio runtime");
	let tokio_handle = runtime.handle().clone();

	let node = new_node(tokio_handle.clone());

	let account_num = 10;
	let extrinsics_per_account = 2000;
	let accounts = create_accounts(account_num);

	let mut group = c.benchmark_group("Transaction pool");

	group.sample_size(10);
	group.throughput(Throughput::Elements(account_num as u64 * extrinsics_per_account as u64));

	let mut counter = 1;
	group.bench_function(
		format!("{} transfers from {} accounts", account_num * extrinsics_per_account, account_num),
		move |b| {
			b.iter_batched(
				|| {
					let prepare_extrinsics = create_account_extrinsics(&node.client, &accounts);

					runtime.block_on(future::join_all(prepare_extrinsics.into_iter().map(|tx| {
						submit_tx_and_wait_for_inclusion(
							&node.transaction_pool,
							tx,
							&node.client,
							true,
						)
					})));

					create_benchmark_extrinsics(&node.client, &accounts, extrinsics_per_account)
				},
				|extrinsics| {
					runtime.block_on(future::join_all(extrinsics.into_iter().map(|tx| {
						submit_tx_and_wait_for_inclusion(
							&node.transaction_pool,
							tx,
							&node.client,
							false,
						)
					})));

					println!("Finished {}", counter);
					counter += 1;
				},
				BatchSize::SmallInput,
			)
		},
	);
}

criterion_group!(benches, transaction_pool_benchmarks);
criterion_main!(benches);
