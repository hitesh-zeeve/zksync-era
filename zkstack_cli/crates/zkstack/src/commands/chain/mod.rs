use ::common::forge::ForgeScriptArgs;
pub(crate) use args::create::ChainCreateArgsFinal;
use clap::{command, Subcommand};
pub(crate) use create::create_chain_inner;
use xshell::Shell;

use crate::commands::chain::{
    args::{create::ChainCreateArgs, propose_registration::ProposeRegistrationArgs},
    deploy_l2_contracts::Deploy2ContractsOption,
    genesis::GenesisCommand,
    init::ChainInitCommand,
};

mod accept_chain_ownership;
pub(crate) mod args;
mod common;
mod create;
pub mod deploy_l2_contracts;
pub mod deploy_paymaster;
pub mod genesis;
pub mod init;
pub mod propose_chain;
mod set_token_multiplier_setter;
mod setup_legacy_bridge;

#[derive(Subcommand, Debug)]
pub enum ChainCommands {
    /// Create a new chain, setting the necessary configurations for later initialization
    Create(ChainCreateArgs),
    /// Initialize chain, deploying necessary contracts and performing on-chain operations
    Init(Box<ChainInitCommand>),
    /// Run server genesis
    Genesis(GenesisCommand),
    /// Deploy all L2 contracts (executed by L1 governor).
    #[command(alias = "l2")]
    DeployL2Contracts(ForgeScriptArgs),
    /// Accept ownership of L2 chain (executed by L2 governor).
    /// This command should be run after `register-chain` to accept ownership of newly created
    /// DiamondProxy contract.
    #[command(alias = "accept-ownership")]
    AcceptChainOwnership(ForgeScriptArgs),
    /// Initialize bridges on L2
    #[command(alias = "bridge")]
    InitializeBridges(ForgeScriptArgs),
    /// Deploy L2 consensus registry
    #[command(alias = "consensus")]
    DeployConsensusRegistry(ForgeScriptArgs),
    /// Deploy L2 multicall3
    #[command(alias = "multicall3")]
    DeployMulticall3(ForgeScriptArgs),
    /// Deploy L2 TimestampAsserter
    #[command(alias = "timestamp-asserter")]
    DeployTimestampAsserter(ForgeScriptArgs),
    /// Deploy Default Upgrader
    #[command(alias = "upgrader")]
    DeployUpgrader(ForgeScriptArgs),
    /// Deploy paymaster smart contract
    #[command(alias = "paymaster")]
    DeployPaymaster(ForgeScriptArgs),
    /// Update Token Multiplier Setter address on L1
    UpdateTokenMultiplierSetter(ForgeScriptArgs),
    ProposeChain(ProposeRegistrationArgs),
}

pub(crate) async fn run(shell: &Shell, args: ChainCommands) -> anyhow::Result<()> {
    match args {
        ChainCommands::Create(args) => create::run(args, shell),
        ChainCommands::Init(args) => init::run(*args, shell).await,
        ChainCommands::Genesis(args) => genesis::run(args, shell).await,
        ChainCommands::DeployL2Contracts(args) => {
            deploy_l2_contracts::run(args, shell, Deploy2ContractsOption::All).await
        }
        ChainCommands::AcceptChainOwnership(args) => accept_chain_ownership::run(args, shell).await,
        ChainCommands::DeployConsensusRegistry(args) => {
            deploy_l2_contracts::run(args, shell, Deploy2ContractsOption::ConsensusRegistry).await
        }
        ChainCommands::DeployMulticall3(args) => {
            deploy_l2_contracts::run(args, shell, Deploy2ContractsOption::Multicall3).await
        }
        ChainCommands::DeployTimestampAsserter(args) => {
            deploy_l2_contracts::run(args, shell, Deploy2ContractsOption::TimestampAsserter).await
        }
        ChainCommands::DeployUpgrader(args) => {
            deploy_l2_contracts::run(args, shell, Deploy2ContractsOption::Upgrader).await
        }
        ChainCommands::InitializeBridges(args) => {
            deploy_l2_contracts::run(args, shell, Deploy2ContractsOption::InitiailizeBridges).await
        }
        ChainCommands::DeployPaymaster(args) => deploy_paymaster::run(args, shell).await,
        ChainCommands::UpdateTokenMultiplierSetter(args) => {
            set_token_multiplier_setter::run(args, shell).await
        }
        ChainCommands::ProposeChain(args) => propose_chain::run(shell, args).await,
    }
}
