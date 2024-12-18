use std::{num::NonZeroUsize, sync::Arc};

// Re-export to initialize the layer without having to depend on the crate directly.
pub use zksync_node_storage_init::SnapshotRecoveryConfig;
use zksync_node_storage_init::{
    external_node::{ExternalNodeGenesis, ExternalNodeReverter, NodeRecovery},
    InitializeStorage, NodeInitializationStrategy, RevertStorage,
};
use zksync_types::{Address, L2ChainId};

use super::NodeInitializationStrategyResource;
use crate::{
    implementations::resources::{
        blob_client::BlobClientResource,
        eth_interface::EthInterfaceResource,
        healthcheck::AppHealthCheckResource,
        main_node_client::MainNodeClientResource,
        pools::{MasterPool, PoolResource},
        reverter::BlockReverterResource,
    },
    wiring_layer::{WiringError, WiringLayer},
    FromContext, IntoContext,
};

/// Wiring layer for external node initialization strategy.
#[derive(Debug)]
pub struct ExternalNodeInitStrategyLayer {
    pub l2_chain_id: L2ChainId,
    pub max_postgres_concurrency: NonZeroUsize,
    pub snapshot_recovery_config: Option<SnapshotRecoveryConfig>,
    pub diamond_proxy_addr: Address,
}

#[derive(Debug, FromContext)]
#[context(crate = crate)]
pub struct Input {
    pub l1_client: EthInterfaceResource,
    pub master_pool: PoolResource<MasterPool>,
    pub main_node_client: MainNodeClientResource,
    pub block_reverter: Option<BlockReverterResource>,
    #[context(default)]
    pub app_health: AppHealthCheckResource,
    pub blob_client: Option<BlobClientResource>,
}

#[derive(Debug, IntoContext)]
#[context(crate = crate)]
pub struct Output {
    pub strategy: NodeInitializationStrategyResource,
}

#[async_trait::async_trait]
impl WiringLayer for ExternalNodeInitStrategyLayer {
    type Input = Input;
    type Output = Output;

    fn layer_name(&self) -> &'static str {
        "external_node_role_layer"
    }

    async fn wire(self, input: Self::Input) -> Result<Self::Output, WiringError> {
        let pool = input.master_pool.get().await?;
        let MainNodeClientResource(client) = input.main_node_client;
        let AppHealthCheckResource(app_health) = input.app_health;
        let block_reverter = match input.block_reverter {
            Some(reverter) => {
                // If reverter was provided, we intend to be its sole consumer.
                // We don't want multiple components to attempt reverting blocks.
                let reverter = reverter.0.take().ok_or(WiringError::Configuration(
                    "BlockReverterResource is taken".into(),
                ))?;
                Some(reverter)
            }
            None => None,
        };

        let genesis = Arc::new(ExternalNodeGenesis {
            l2_chain_id: self.l2_chain_id,
            client: client.clone(),
            pool: pool.clone(),
        });
        let snapshot_recovery = match self.snapshot_recovery_config {
            Some(recovery_config) => {
                // Add a connection for checking whether the storage is initialized.
                let recovery_pool = input
                    .master_pool
                    .get_custom(self.max_postgres_concurrency.get() as u32 + 1)
                    .await?;
                let recovery: Arc<dyn InitializeStorage> = Arc::new(NodeRecovery {
                    main_node_client: Some(client.clone()),
                    l1_client: input.l1_client.0.clone(),
                    pool: recovery_pool,
                    max_concurrency: self.max_postgres_concurrency,
                    recovery_config,
                    app_health,
                    diamond_proxy_addr: self.diamond_proxy_addr,
                    blob_client: input.blob_client.clone().map(|x| x.0),
                });
                Some(recovery)
            }
            None => None,
        };
        // We always want to detect reorgs, even if we can't roll them back.
        let block_reverter = Some(Arc::new(ExternalNodeReverter {
            client,
            pool: pool.clone(),
            reverter: block_reverter,
        }) as Arc<dyn RevertStorage>);
        let strategy = NodeInitializationStrategy {
            genesis,
            snapshot_recovery,
            block_reverter,
        };

        Ok(Output {
            strategy: strategy.into(),
        })
    }
}
