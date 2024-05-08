use std::{sync::Arc, time::Instant};

use anyhow::Context;
use async_trait::async_trait;
use multivm::interface::{L2BlockEnv, VmInterface};
use tokio::{runtime::Handle, task::JoinHandle};
use vm_utils::{create_vm, execute_tx};
use zksync_dal::{
    basic_witness_input_producer_dal::JOB_MAX_ATTEMPT, ConnectionPool, Core, CoreDal,
};
use zksync_object_store::{ObjectStore, ObjectStoreFactory};
use zksync_queued_job_processor::JobProcessor;
use zksync_types::{witness_block_state::WitnessBlockState, L1BatchNumber, L2ChainId};

use self::metrics::METRICS;

mod metrics;
/// Component that extracts all data (from DB) necessary to run a Basic Witness Generator.
/// Does this by rerunning an entire L1Batch and extracting information from both the VM run and DB.
/// This component will upload Witness Inputs to the object store.
/// This allows Witness Generator workflow (that needs only Basic Witness Generator Inputs)
/// to be run only using the object store information, having no other external dependency.
#[derive(Debug)]
pub struct BasicWitnessInputProducer {
    connection_pool: ConnectionPool<Core>,
    l2_chain_id: L2ChainId,
    object_store: Arc<dyn ObjectStore>,
}

impl BasicWitnessInputProducer {
    pub async fn new(
        connection_pool: ConnectionPool<Core>,
        store_factory: &ObjectStoreFactory,
        l2_chain_id: L2ChainId,
    ) -> anyhow::Result<Self> {
        Ok(BasicWitnessInputProducer {
            connection_pool,
            object_store: store_factory.create_store().await,
            l2_chain_id,
        })
    }

    fn process_job_impl(
        rt_handle: Handle,
        l1_batch_number: L1BatchNumber,
        started_at: Instant,
        connection_pool: ConnectionPool<Core>,
        l2_chain_id: L2ChainId,
    ) -> anyhow::Result<WitnessBlockState> {
        let mut connection = rt_handle
            .block_on(connection_pool.connection())
            .context("failed to get connection for BasicWitnessInputProducer")?;

        let l2_blocks_execution_data = rt_handle.block_on(
            connection
                .transactions_dal()
                .get_l2_blocks_to_execute_for_l1_batch(l1_batch_number),
        )?;

        let (mut vm, storage_view) =
            create_vm(rt_handle.clone(), l1_batch_number, connection, l2_chain_id)
                .context("failed to create vm for BasicWitnessInputProducer")?;

        tracing::info!("Started execution of l1_batch: {l1_batch_number:?}");

        let next_l2_blocks_data = l2_blocks_execution_data
            .iter()
            .skip(1)
            .map(Some)
            .chain([None]);
        let l2_blocks_data = l2_blocks_execution_data.iter().zip(next_l2_blocks_data);

        for (l2_block_data, next_l2_block_data) in l2_blocks_data {
            tracing::debug!(
                "Started execution of L2 block: {:?}, executing {:?} transactions",
                l2_block_data.number,
                l2_block_data.txs.len(),
            );
            for tx in &l2_block_data.txs {
                tracing::trace!("Started execution of tx: {tx:?}");
                execute_tx(tx, &mut vm)
                    .context("failed to execute transaction in BasicWitnessInputProducer")?;
                tracing::trace!("Finished execution of tx: {tx:?}");
            }
            if let Some(next_l2_block_data) = next_l2_block_data {
                vm.start_new_l2_block(L2BlockEnv::from_l2_block_data(next_l2_block_data));
            }

            tracing::debug!("Finished execution of L2 block: {:?}", l2_block_data.number);
        }
        vm.finish_batch();
        tracing::info!("Finished execution of l1_batch: {l1_batch_number:?}");

        METRICS.process_batch_time.observe(started_at.elapsed());
        tracing::info!(
            "BasicWitnessInputProducer took {:?} for L1BatchNumber {}",
            started_at.elapsed(),
            l1_batch_number.0
        );

        let witness_block_state = (*storage_view).borrow().witness_block_state();
        Ok(witness_block_state)
    }
}

#[async_trait]
impl JobProcessor for BasicWitnessInputProducer {
    type Job = L1BatchNumber;
    type JobId = L1BatchNumber;
    type JobArtifacts = WitnessBlockState;
    const SERVICE_NAME: &'static str = "basic_witness_input_producer";

    async fn get_next_job(&self) -> anyhow::Result<Option<(Self::JobId, Self::Job)>> {
        let mut connection = self.connection_pool.connection().await?;
        let l1_batch_to_process = connection
            .basic_witness_input_producer_dal()
            .get_next_basic_witness_input_producer_job()
            .await?;
        Ok(l1_batch_to_process.map(|number| (number, number)))
    }

    async fn save_failure(&self, job_id: Self::JobId, started_at: Instant, error: String) {
        let attempts = self
            .connection_pool
            .connection()
            .await
            .unwrap()
            .basic_witness_input_producer_dal()
            .mark_job_as_failed(job_id, started_at, error)
            .await
            .expect("errored whilst marking job as failed");
        if let Some(tries) = attempts {
            tracing::warn!("Failed to process job: {job_id:?}, after {tries} tries.");
        } else {
            tracing::warn!("L1 Batch {job_id:?} was processed successfully by another worker.");
        }
    }

    async fn process_job(
        &self,
        _job_id: &Self::JobId,
        job: Self::Job,
        started_at: Instant,
    ) -> JoinHandle<anyhow::Result<Self::JobArtifacts>> {
        let l2_chain_id = self.l2_chain_id;
        let connection_pool = self.connection_pool.clone();
        tokio::task::spawn_blocking(move || {
            let rt_handle = Handle::current();
            Self::process_job_impl(
                rt_handle,
                job,
                started_at,
                connection_pool.clone(),
                l2_chain_id,
            )
        })
    }

    async fn save_result(
        &self,
        job_id: Self::JobId,
        started_at: Instant,
        artifacts: Self::JobArtifacts,
    ) -> anyhow::Result<()> {
        let upload_started_at = Instant::now();
        let object_path = self
            .object_store
            .put(job_id, &artifacts)
            .await
            .context("failed to upload artifacts for BasicWitnessInputProducer")?;
        METRICS
            .upload_input_time
            .observe(upload_started_at.elapsed());
        let mut connection = self.connection_pool.connection().await?;
        let mut transaction = connection.start_transaction().await?;
        transaction
            .basic_witness_input_producer_dal()
            .mark_job_as_successful(job_id, started_at, &object_path)
            .await?;
        transaction.commit().await?;
        METRICS.block_number_processed.set(job_id.0 as i64);
        Ok(())
    }

    fn max_attempts(&self) -> u32 {
        JOB_MAX_ATTEMPT as u32
    }

    async fn get_job_attempts(&self, job_id: &L1BatchNumber) -> anyhow::Result<u32> {
        let mut connection = self.connection_pool.connection().await?;
        Ok(connection
            .basic_witness_input_producer_dal()
            .get_basic_witness_input_producer_job_attempts(*job_id)
            .await
            .map(|attempts| attempts.unwrap_or(0))?)
    }
}
