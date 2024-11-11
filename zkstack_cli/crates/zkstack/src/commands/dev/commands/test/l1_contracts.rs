use common::{cmd::Cmd, logger};
use config::zkstack_config::ZkStackConfig;
use xshell::{cmd, Shell};

use crate::commands::dev::messages::MSG_L1_CONTRACTS_TEST_SUCCESS;

pub fn run(shell: &Shell) -> anyhow::Result<()> {
    let ecosystem = ZkStackConfig::ecosystem(shell)?;
    let _dir_guard = shell.push_dir(&ecosystem.link_to_code);

    Cmd::new(cmd!(shell, "yarn l1-contracts test"))
        .with_force_run()
        .run()?;

    logger::outro(MSG_L1_CONTRACTS_TEST_SUCCESS);

    Ok(())
}
