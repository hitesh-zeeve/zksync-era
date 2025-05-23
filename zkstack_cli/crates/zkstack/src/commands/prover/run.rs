use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use xshell::{cmd, Shell};
use zkstack_cli_common::{check_prerequisites, cmd::Cmd, logger, GPU_PREREQUISITES};
use zkstack_cli_config::{get_link_to_prover, ChainConfig, EcosystemConfig};

use super::args::run::{ProverComponent, ProverRunArgs};
use crate::messages::{
    MSG_BELLMAN_CUDA_DIR_ERR, MSG_CHAIN_NOT_FOUND_ERR, MSG_MISSING_COMPONENT_ERR,
    MSG_RUNNING_CIRCUIT_PROVER, MSG_RUNNING_CIRCUIT_PROVER_ERR, MSG_RUNNING_COMPRESSOR,
    MSG_RUNNING_COMPRESSOR_ERR, MSG_RUNNING_PROVER_GATEWAY, MSG_RUNNING_PROVER_GATEWAY_ERR,
    MSG_RUNNING_PROVER_JOB_MONITOR, MSG_RUNNING_PROVER_JOB_MONITOR_ERR,
    MSG_RUNNING_WITNESS_GENERATOR, MSG_RUNNING_WITNESS_GENERATOR_ERR,
};

pub(crate) async fn run(args: ProverRunArgs, shell: &Shell) -> anyhow::Result<()> {
    let args = args.fill_values_with_prompt()?;
    let ecosystem_config = EcosystemConfig::from_file(shell)?;
    let chain = ecosystem_config
        .load_current_chain()
        .expect(MSG_CHAIN_NOT_FOUND_ERR);

    let path_to_ecosystem = shell.current_dir();

    let link_to_prover = get_link_to_prover(&ecosystem_config);
    shell.change_dir(link_to_prover.clone());

    let component = args.component.context(anyhow!(MSG_MISSING_COMPONENT_ERR))?;
    let in_docker = args.docker.unwrap_or(false);

    let application_args = component.get_application_args(in_docker)?;
    let additional_args =
        component.get_additional_args(in_docker, args.clone(), &chain, &path_to_ecosystem)?;

    let (message, error) = match component {
        ProverComponent::WitnessGenerator => (
            MSG_RUNNING_WITNESS_GENERATOR,
            MSG_RUNNING_WITNESS_GENERATOR_ERR,
        ),
        ProverComponent::CircuitProver => {
            if !in_docker {
                check_prerequisites(shell, &GPU_PREREQUISITES, false);
            }
            (MSG_RUNNING_CIRCUIT_PROVER, MSG_RUNNING_CIRCUIT_PROVER_ERR)
        }
        ProverComponent::Compressor => {
            if !in_docker {
                check_prerequisites(shell, &GPU_PREREQUISITES, false);
                shell.set_var(
                    "BELLMAN_CUDA_DIR",
                    ecosystem_config
                        .bellman_cuda_dir
                        .clone()
                        .expect(MSG_BELLMAN_CUDA_DIR_ERR),
                );
            }
            (MSG_RUNNING_COMPRESSOR, MSG_RUNNING_COMPRESSOR_ERR)
        }
        ProverComponent::ProverJobMonitor => (
            MSG_RUNNING_PROVER_JOB_MONITOR,
            MSG_RUNNING_PROVER_JOB_MONITOR_ERR,
        ),
        ProverComponent::Gateway => (MSG_RUNNING_PROVER_GATEWAY, MSG_RUNNING_PROVER_GATEWAY_ERR),
    };

    if in_docker {
        let path_to_configs = chain.configs.clone();
        let path_to_prover = get_link_to_prover(&ecosystem_config);
        update_setup_data_path(&chain, "prover/data/keys").await?;
        run_dockerized_component(
            shell,
            component.image_name(),
            &args.tag.unwrap(),
            &application_args,
            &additional_args,
            message,
            error,
            &path_to_configs,
            &path_to_prover,
            &path_to_ecosystem,
        )?
    } else {
        update_setup_data_path(&chain, "data/keys").await?;
        run_binary_component(
            shell,
            component.binary_name(),
            &application_args,
            &additional_args,
            message,
            error,
        )?
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_dockerized_component(
    shell: &Shell,
    image_name: &str,
    tag: &str,
    application_args: &[String],
    args: &[String],
    message: &'static str,
    error: &'static str,
    path_to_configs: &PathBuf,
    path_to_prover: &PathBuf,
    path_to_ecosystem: &Path,
) -> anyhow::Result<()> {
    logger::info(message);

    let path_to_configs = path_to_ecosystem.join(path_to_configs);

    let mut cmd = Cmd::new(cmd!(
        shell,
        "docker run --net=host -v {path_to_prover}/data/keys:/prover/data/keys -v {path_to_prover}/artifacts:/artifacts -v {path_to_configs}:/configs {application_args...} {image_name}:{tag} {args...}"
    ));

    cmd = cmd.with_force_run();
    cmd.run().context(error)
}

fn run_binary_component(
    shell: &Shell,
    binary_name: &str,
    application_args: &[String],
    args: &[String],
    message: &'static str,
    error: &'static str,
) -> anyhow::Result<()> {
    logger::info(message);

    let mut cmd = Cmd::new(cmd!(
        shell,
        "cargo run {application_args...} --release --package {binary_name} -- {args...}"
    ));
    cmd = cmd.with_force_run();
    cmd.run().context(error)
}

async fn update_setup_data_path(chain: &ChainConfig, path: &str) -> anyhow::Result<()> {
    let mut general_config = chain.get_general_config().await?.patched();
    general_config.set_prover_setup_path(path.as_ref())?;
    general_config.save().await
}
