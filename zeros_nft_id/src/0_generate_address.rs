//! Generate an address which will be used later to find inputs and request funds to it.
//!
//! ```sh
//! cargo run --bin generate_address
//! ```

use tokio::fs::OpenOptions;

use iota_sdk::{
    client::{
        api::GetAddressesOptions, constants::SHIMMER_TESTNET_BECH32_HRP, request_funds_from_faucet,
        secret::SecretManager, Client, Result,
    },
    crypto::keys::bip39::Mnemonic,
    types::block::address::Bech32Address,
};

const ADDRESS_FILE_NAME: &str = "address.json";
const FAUCET_URL: &str = "https://faucet.testnet.shimmer.network/api/enqueue";

#[tokio::main]
async fn main() -> Result<()> {
    // This example uses secrets in environment variables for simplicity which should not be done in production.
    dotenvy::dotenv().ok();

    let mnemonic = match std::env::var("MNEMONIC") {
        Ok(m) => Mnemonic::from(m),
        Err(_) => {
            let mnemonic: Mnemonic = Client::generate_mnemonic()?;
            write_mnemonic_to_env_file(".env", &mnemonic).await?;
            println!("Mnemonic: {}", mnemonic.as_ref());
            mnemonic
        }
    };

    let secret_manager = SecretManager::try_from_mnemonic(mnemonic)?;

    let address = secret_manager
        .generate_ed25519_addresses(
            GetAddressesOptions::default()
                .with_bech32_hrp(SHIMMER_TESTNET_BECH32_HRP)
                .with_range(0..1),
        )
        .await?[0];

    write_address_to_file(ADDRESS_FILE_NAME, &address).await?;

    let faucet_response = request_funds_from_faucet(FAUCET_URL, &address).await?;
    println!("{faucet_response}");

    Ok(())
}

async fn write_mnemonic_to_env_file(
    path: impl AsRef<std::path::Path>,
    mnemonic: &Mnemonic,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let mut file = tokio::io::BufWriter::new(
        OpenOptions::new()
            .append(true)
            .create(true)
            .open(path)
            .await
            .expect("failed to open file"),
    );

    file.write_all(format!("\nMNEMONIC=\"{}\"", &mnemonic.as_ref()).as_bytes())
        .await
        .expect("failed to write to file");
    file.flush().await.expect("failed to flush output stream");

    Ok(())
}

async fn write_address_to_file(
    path: impl AsRef<std::path::Path>,
    address: &Bech32Address,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let json = serde_json::to_string_pretty(&address)?;
    let mut file = tokio::io::BufWriter::new(
        tokio::fs::File::create(path)
            .await
            .expect("failed to create file"),
    );

    println!("{json}");

    file.write_all(json.as_bytes())
        .await
        .expect("failed to write to file");
    file.flush().await.expect("failed to flush output stream");

    Ok(())
}
