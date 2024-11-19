use std::cell::OnceCell;

use anyhow::Context;
use common::{logger, spinner::Spinner};
use config::{
    create_local_configs_dir, create_wallets,
    traits::{ReadConfigWithBasePath, SaveConfigWithBasePath},
    ChainConfig, EcosystemConfig, GenesisConfig,
};
use xshell::Shell;
use zksync_basic_types::L2ChainId;

use crate::{
    commands::chain::args::create::{ChainCreateArgs, ChainCreateArgsFinal},
    messages::{
        MSG_ARGS_VALIDATOR_ERR, MSG_CHAIN_CREATED, MSG_CREATING_CHAIN,
        MSG_CREATING_CHAIN_CONFIGURATIONS_SPINNER, MSG_EVM_EMULATOR_HASH_MISSING_ERR,
        MSG_SELECTED_CONFIG,
    },
    utils::link_to_code::resolve_link_to_code,
};

pub fn run(args: ChainCreateArgs, shell: &Shell) -> anyhow::Result<()> {
    let mut ecosystem_config = EcosystemConfig::from_file(shell)?;
    create(args, &mut ecosystem_config, shell)
}

fn create(
    args: ChainCreateArgs,
    ecosystem_config: &mut EcosystemConfig,
    shell: &Shell,
) -> anyhow::Result<()> {
    let tokens = ecosystem_config.get_erc20_tokens();
    let args = args
        .fill_values_with_prompt(
            ecosystem_config.list_of_chains().len() as u32,
            &ecosystem_config.l1_network,
            tokens,
            ecosystem_config.link_to_code.clone().display().to_string(),
        )
        .context(MSG_ARGS_VALIDATOR_ERR)?;

    logger::note(MSG_SELECTED_CONFIG, logger::object_to_string(&args));
    logger::info(MSG_CREATING_CHAIN);

    let spinner = Spinner::new(MSG_CREATING_CHAIN_CONFIGURATIONS_SPINNER);
    let name = args.chain_name.clone();
    let set_as_default = args.set_as_default;
    create_chain_inner(args, ecosystem_config, shell)?;
    if set_as_default {
        ecosystem_config.default_chain = name;
        ecosystem_config.save_with_base_path(shell, ".")?;
    }
    spinner.finish();

    logger::success(MSG_CHAIN_CREATED);

    Ok(())
}

pub(crate) fn create_chain_inner(
    args: ChainCreateArgsFinal,
    ecosystem_config: &EcosystemConfig,
    shell: &Shell,
) -> anyhow::Result<()> {
    if args.legacy_bridge {
        logger::warn("WARNING!!! You are creating a chain with legacy bridge, use it only for testing compatibility")
    }
    let default_chain_name = args.chain_name.clone();
    let chain_path = ecosystem_config.chains.join(&default_chain_name);
    let chain_configs_path = create_local_configs_dir(shell, &chain_path)?;
    let (chain_id, legacy_bridge) = if args.legacy_bridge {
        // Legacy bridge is distinguished by using the same chain id as ecosystem
        (ecosystem_config.era_chain_id, Some(true))
    } else {
        (L2ChainId::from(args.chain_id), None)
    };
    let internal_id = ecosystem_config.list_of_chains().len() as u32;
    let link_to_code = resolve_link_to_code(
        shell,
        chain_path.clone(),
        args.link_to_code.clone(),
        args.skip_submodules_checkout,
    )?;
    let default_genesis_config = GenesisConfig::read_with_base_path(
        shell,
        EcosystemConfig::default_configs_path(&link_to_code),
    )?;
    let has_evm_emulation_support = default_genesis_config.evm_emulator_hash.is_some();
    if args.evm_emulator && !has_evm_emulation_support {
        anyhow::bail!(MSG_EVM_EMULATOR_HASH_MISSING_ERR);
    }

    let chain_config = ChainConfig {
        id: internal_id,
        name: default_chain_name.clone(),
        chain_id,
        prover_version: args.prover_version,
        l1_network: ecosystem_config.l1_network,
        link_to_code: ecosystem_config.link_to_code.clone(),
        rocks_db_path: ecosystem_config.get_chain_rocks_db_path(&default_chain_name),
        artifacts: ecosystem_config.get_chain_artifacts_path(&default_chain_name),
        configs: chain_configs_path.clone(),
        external_node_config_path: None,
        l1_batch_commit_data_generator_mode: args.l1_batch_commit_data_generator_mode,
        base_token: args.base_token,
        wallet_creation: args.wallet_creation,
        shell: OnceCell::from(shell.clone()),
        legacy_bridge,
        evm_emulator: args.evm_emulator,
    };

    create_wallets(
        shell,
        &chain_config.configs,
        &ecosystem_config.link_to_code,
        internal_id,
        args.wallet_creation,
        args.wallet_path,
    )?;

    chain_config.save_with_base_path(shell, chain_path)?;
    Ok(())
}
