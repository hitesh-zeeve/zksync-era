use anyhow::Context;
use common::logger;
use common::spinner::Spinner;
use config::{get_link_to_prover, EcosystemConfig, GeneralConfig};
use xshell::Shell;

use super::args::compressor_keys::{CompressorKeysArgs, CompressorType};
use crate::consts::{FFLONK_COMPACT_CRS_KEY, FFLONK_CRS_KEY, PLONK_CRS_KEY};
use crate::messages::{
    MSG_CHAIN_NOT_FOUND_ERR, MSG_DOWNLOADING_SETUP_COMPRESSOR_KEY_SPINNER,
    MSG_PROOF_COMPRESSOR_CONFIG_NOT_FOUND_ERR, MSG_SETUP_KEY_PATH_ERROR,
};

pub(crate) async fn run(shell: &Shell, args: CompressorKeysArgs) -> anyhow::Result<()> {
    let ecosystem_config = EcosystemConfig::from_file(shell)?;
    let chain_config = ecosystem_config
        .load_current_chain()
        .context(MSG_CHAIN_NOT_FOUND_ERR)?;
    let mut general_config = chain_config.get_general_config()?;

    let default_path = get_default_compressor_keys_path(&ecosystem_config)?;
    let args = args.fill_values_with_prompt(&default_path);

    match args.compressor_type {
        CompressorType::Fflonk => {
            let path = args.clone().path.context(MSG_SETUP_KEY_PATH_ERROR)?;

            download_compressor_key(
                shell,
                &mut general_config,
                FFLONK_CRS_KEY,
                &format!("{}{}", path, FFLONK_CRS_KEY),
            )?;
            download_compressor_key(
                shell,
                &mut general_config,
                FFLONK_COMPACT_CRS_KEY,
                &format!("{}{}", path, FFLONK_COMPACT_CRS_KEY),
            )?;
        }
        CompressorType::Plonk => {
            let path = args.path.context(MSG_SETUP_KEY_PATH_ERROR)?;

            download_compressor_key(
                shell,
                &mut general_config,
                PLONK_CRS_KEY,
                &format!("{}{}", path, PLONK_CRS_KEY),
            )?;
        }
    }

    chain_config.save_general_config(&general_config)?;

    Ok(())
}

pub(crate) fn download_compressor_key(
    shell: &Shell,
    general_config: &mut GeneralConfig,
    key: &str,
    path: &str,
) -> anyhow::Result<()> {
    let spinner = Spinner::new(MSG_DOWNLOADING_SETUP_COMPRESSOR_KEY_SPINNER);
    let mut compressor_config: zksync_config::configs::FriProofCompressorConfig = general_config
        .proof_compressor_config
        .as_ref()
        .expect(MSG_PROOF_COMPRESSOR_CONFIG_NOT_FOUND_ERR)
        .clone();

    compressor_config.universal_setup_path = path.to_string();
    general_config.proof_compressor_config = Some(compressor_config.clone());

    let mut url = compressor_config.universal_setup_download_url;
    url.push_str(key);

    let path = std::path::Path::new(path);

    logger::info(format!("Downloading setup key {} by URL: {}", key, url));

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let response = client.get(url).send()?.bytes()?;
    shell.write_file(path, &response)?;

    spinner.finish();
    Ok(())
}

pub fn get_default_compressor_keys_path(
    ecosystem_config: &EcosystemConfig,
) -> anyhow::Result<String> {
    let link_to_prover = get_link_to_prover(ecosystem_config);
    let path = link_to_prover.join("keys/setup/");
    let string = path.to_str().unwrap();

    Ok(String::from(string))
}
