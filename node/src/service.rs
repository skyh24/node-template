//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::sync::Arc;
use std::time::Duration;
use sc_client_api::{ExecutorProvider, RemoteBackend};
use node_template_runtime::{self, opaque::Block, RuntimeApi};
use sc_service::{error::Error as ServiceError, Configuration, TaskManager};
use sp_inherents::InherentDataProviders;
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_finality_grandpa::SharedVoterState;
use sc_keystore::LocalKeystore;

// Our native executor instance.
native_executor_instance!(
	pub Executor,
	node_template_runtime::api::dispatch,
	node_template_runtime::native_version,
	frame_benchmarking::benchmarking::HostFunctions,
);

type FullClient = sc_service::TFullClient<Block, RuntimeApi, Executor>;
type FullBackend = sc_service::TFullBackend<Block>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;

pub fn new_partial(config: &Configuration) -> Result<sc_service::PartialComponents<
	FullClient, FullBackend, FullSelectChain,
	sp_consensus::DefaultImportQueue<Block, FullClient>,
	sc_transaction_pool::FullPool<Block, FullClient>,
	(
		sc_consensus_babe::BabeBlockImport<
			Block,
			FullClient,
			sc_finality_grandpa::GrandpaBlockImport<FullBackend, Block, FullClient, FullSelectChain>,
		>,
		sc_finality_grandpa::LinkHalf<Block, FullClient, FullSelectChain>,
		sc_consensus_babe::BabeLink<Block>,
	)
>, ServiceError> {
	// if config.keystore_remote.is_some() {
	// 	return Err(ServiceError::Other(
	// 		format!("Remote Keystores are not supported.")))
	// }

	// if !test {
	// 	// If we're using prometheus, use a registry with a prefix of `acala`.
	// 	if let Some(PrometheusConfig { registry, .. }) = config.prometheus_config.as_mut() {
	// 		*registry = Registry::new_custom(Some("node-template".into()), None)?;
	// 	}
	// }

	let (client, backend, keystore_container, task_manager) =
		sc_service::new_full_parts::<Block, RuntimeApi, Executor>(&config)?;
	let client = Arc::new(client);

	let select_chain = sc_consensus::LongestChain::new(backend.clone());
	let inherent_data_providers = InherentDataProviders::new();

	let transaction_pool = sc_transaction_pool::BasicPool::new_full(
		config.transaction_pool.clone(),
		config.role.is_authority().into(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
	);

	let (grandpa_block_import, grandpa_link) = sc_finality_grandpa::block_import(
		client.clone(), &(client.clone() as Arc<_>), select_chain.clone(),
	)?;

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get_or_compute(&*client)?,
		grandpa_block_import.clone(),
		client.clone(),
	)?;

	let justification_import = grandpa_block_import.clone();

	// let import_queue = if instant_sealing {
	// 	inherent_data_providers
	// 		.register_provider(mock_timestamp_data_provider::MockTimestampInherentDataProvider)
	// 		.map_err(Into::into)
	// 		.map_err(sp_consensus::error::Error::InherentData)?;
	//
	// 	sc_consensus_manual_seal::import_queue(
	// 		Box::new(client.clone()),
	// 		&task_manager.spawn_handle(),
	// 		config.prometheus_registry(),
	// 	)
	// } else {
	let import_queue = sc_consensus_babe::import_queue(
		babe_link.clone(),
		block_import.clone(),
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
		sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone()),
	)?;

	// let justification_stream = grandpa_link.justification_stream();
	// let shared_authority_set = grandpa_link.shared_authority_set().clone();
	// let shared_voter_state = sc_finality_grandpa::SharedVoterState::empty();

	// let babe_config = babe_link.config().clone();
	// let shared_epoch_changes = babe_link.epoch_changes().clone();
	// let finality_proof_provider = GrandpaFinalityProofProvider::new_for_service(backend.clone(), client.clone());
	//
	// let rpc_extensions_builder = {
	// 	let client = client.clone();
	// 	let keystore = keystore_container.sync_keystore();
	// 	let transaction_pool = transaction_pool.clone();
	// 	let select_chain = select_chain.clone();
	//
	// 	move |deny_unsafe, subscription_executor| -> acala_rpc::RpcExtension {
	// 		let deps = acala_rpc::FullDeps {
	// 			client: client.clone(),
	// 			pool: transaction_pool.clone(),
	// 			select_chain: select_chain.clone(),
	// 			deny_unsafe,
	// 			babe: acala_rpc::BabeDeps {
	// 				babe_config: babe_config.clone(),
	// 				shared_epoch_changes: shared_epoch_changes.clone(),
	// 				keystore: keystore.clone(),
	// 			},
	// 			grandpa: acala_rpc::GrandpaDeps {
	// 				shared_voter_state: shared_voter_state.clone(),
	// 				shared_authority_set: shared_authority_set.clone(),
	// 				justification_stream: justification_stream.clone(),
	// 				subscription_executor,
	// 				finality_provider: finality_proof_provider.clone(),
	// 			},
	// 		};
	//
	// 		acala_rpc::create_full(deps)
	// 	}
	// };

	let import_setup = (block_import, grandpa_link, babe_link.clone());
	// let rpc_setup = shared_voter_state.clone();

	Ok(sc_service::PartialComponents {
		client,
		backend,
		task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		inherent_data_providers,
		other: import_setup,
	})
}

// fn remote_keystore(_url: &String) -> Result<Arc<LocalKeystore>, &'static str> {
// 	// FIXME: here would the concrete keystore be built,
// 	//        must return a concrete type (NOT `LocalKeystore`) that
// 	//        implements `CryptoStore` and `SyncCryptoStore`
// 	Err("Remote Keystore not supported.")
// }

/// Builds a new service for a full client.
pub fn new_full(mut config: Configuration) -> Result<TaskManager, ServiceError> {

	let sc_service::PartialComponents {
		client,
		backend,
		mut task_manager,
		keystore_container,
		select_chain,
		import_queue,
		transaction_pool,
		inherent_data_providers,
		other: import_setup,
	} = new_partial(&config)?;

	// if let Some(url) = &config.keystore_remote {
	// 	match remote_keystore(url) {
	// 		Ok(k) => keystore_container.set_remote_keystore(k),
	// 		Err(e) => {
	// 			return Err(ServiceError::Other(
	// 				format!("Error hooking up remote keystore for {}: {}", url, e)))
	// 		}
	// 	};
	// }

	let role = config.role.clone();
	let force_authoring = config.force_authoring;
	let backoff_authoring_blocks: Option<()> = None; //Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
	let name = config.network.node_name.clone();
	let enable_grandpa = !config.disable_grandpa;
	let prometheus_registry = config.prometheus_registry().cloned();
	// let telemetry_connection_sinks = sc_service::TelemetryConnectionSinks::default();

	let (block_import, grandpa_link, babe_link) = import_setup;
	// let shared_voter_state = rpc_setup;

	config.network.extra_sets.push(sc_finality_grandpa::grandpa_peers_set_config());

	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: None,
			block_announce_validator_builder: None,
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config, backend.clone(),
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	let rpc_extensions_builder = {
		let client = client.clone();
		let pool = transaction_pool.clone();

		// TODO my_rpc
		Box::new(move |deny_unsafe, _| {
			let deps = crate::rpc::FullDeps {
				client: client.clone(),
				pool: pool.clone(),
				deny_unsafe,
			};

			crate::rpc::create_full(deps)
		})
	};

	let (_rpc_handlers, telemetry_connection_notifier) = sc_service::spawn_tasks(
		sc_service::SpawnTasksParams {
			network: network.clone(),
			client: client.clone(),
			keystore: keystore_container.sync_keystore(),
			task_manager: &mut task_manager,
			transaction_pool: transaction_pool.clone(),
			rpc_extensions_builder,
			on_demand: None,
			remote_blockchain: None,
			backend,
			network_status_sinks,
			system_rpc_tx,
			config,
		},
	)?;

	// if instant_sealing { // sc_consensus_manual_seal
	if role.is_authority() {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			task_manager.spawn_handle(),
			client.clone(),
			transaction_pool,
			prometheus_registry.as_ref(),
		);

		let can_author_with =
			sp_consensus::CanAuthorWithNativeVersion::new(client.executor().clone());

		let babe_config = sc_consensus_babe::BabeParams {
			keystore: keystore_container.sync_keystore(),
			client: client.clone(),
			select_chain,
			env: proposer,
			block_import,
			sync_oracle: network.clone(),
			inherent_data_providers: inherent_data_providers.clone(),
			force_authoring,
			backoff_authoring_blocks,
			babe_link,
			can_author_with,
		};

		let babe = sc_consensus_babe::start_babe(babe_config)?;
		task_manager
			.spawn_essential_handle()
			.spawn_blocking("babe-proposer", babe);
	}

	// if the node isn't actively participating in consensus then it doesn't
	// need a keystore, regardless of which protocol we use below.
	let keystore = if role.is_authority() {
		Some(keystore_container.sync_keystore())
	} else {
		None
	};

	let grandpa_config = sc_finality_grandpa::Config {
		// FIXME #1578 make this available through chainspec
		gossip_duration: Duration::from_millis(333),
		justification_period: 512,
		name: Some(name),
		observer_enabled: false,
		keystore,
		is_authority: role.is_network_authority(),
	};

	if enable_grandpa {
		let grandpa_config = sc_finality_grandpa::GrandpaParams {
			config: grandpa_config,
			link: grandpa_link,
			network,
			telemetry_on_connect: telemetry_connection_notifier.map(|x| x.on_connect_stream()),
			voting_rule: sc_finality_grandpa::VotingRulesBuilder::default().build(),
			prometheus_registry,
			shared_voter_state: SharedVoterState::empty(),
		};

		// the GRANDPA voter task is considered infallible, i.e.
		// if it fails we take down the service with it.
		task_manager.spawn_essential_handle().spawn_blocking(
			"grandpa-voter",
			sc_finality_grandpa::run_grandpa_voter(grandpa_config)?
		);
	}

	network_starter.start_network();
	Ok(task_manager)
}

/// Builds a new service for a light client.
pub fn new_light(mut config: Configuration) -> Result<TaskManager, ServiceError> {
	let (client, backend, keystore_container, mut task_manager, on_demand) =
		sc_service::new_light_parts::<Block, RuntimeApi, Executor>(&config)?;

	config.network.extra_sets.push(sc_finality_grandpa::grandpa_peers_set_config());

	let select_chain = sc_consensus::LongestChain::new(backend.clone());

	let transaction_pool = Arc::new(sc_transaction_pool::BasicPool::new_light(
		config.transaction_pool.clone(),
		config.prometheus_registry(),
		task_manager.spawn_handle(),
		client.clone(),
		on_demand.clone(),
	));

	let (grandpa_block_import, _) = sc_finality_grandpa::block_import(
		client.clone(),
		&(client.clone() as Arc<_>),
		select_chain.clone(),
	)?;

	let inherent_data_providers = sp_inherents::InherentDataProviders::new();
	let justification_import = grandpa_block_import.clone();

	let (block_import, babe_link) = sc_consensus_babe::block_import(
		sc_consensus_babe::Config::get_or_compute(&*client)?,
		grandpa_block_import.clone(),
		client.clone(),
	)?;

	let import_queue = sc_consensus_babe::import_queue(
		babe_link.clone(),
		block_import.clone(),
		Some(Box::new(justification_import)),
		client.clone(),
		select_chain.clone(),
		inherent_data_providers.clone(),
		&task_manager.spawn_handle(),
		config.prometheus_registry(),
		sp_consensus::NeverCanAuthor,
	)?;


	let (network, network_status_sinks, system_rpc_tx, network_starter) =
		sc_service::build_network(sc_service::BuildNetworkParams {
			config: &config,
			client: client.clone(),
			transaction_pool: transaction_pool.clone(),
			spawn_handle: task_manager.spawn_handle(),
			import_queue,
			on_demand: Some(on_demand.clone()),
			block_announce_validator_builder: None,
		})?;

	if config.offchain_worker.enabled {
		sc_service::build_offchain_workers(
			&config,
			backend.clone(),
			task_manager.spawn_handle(),
			client.clone(),
			network.clone(),
		);
	}

	sc_service::spawn_tasks(sc_service::SpawnTasksParams {
		remote_blockchain: Some(backend.remote_blockchain()),
		transaction_pool,
		task_manager: &mut task_manager,
		on_demand: Some(on_demand),
		rpc_extensions_builder: Box::new(|_, _| ()),
		config,
		client,
		keystore: keystore_container.sync_keystore(),
		backend,
		network,
		network_status_sinks,
		system_rpc_tx,
	})?;

	network_starter.start_network();
	Ok(task_manager)
}
