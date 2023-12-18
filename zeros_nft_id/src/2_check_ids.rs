//! Check the tx which results in the NFT ID with the lowest hash.
//!
//! ```sh
//! cargo run --bin check_ids
//! ```

use iota_sdk::{
    client::{
        api::{SignedTransactionData, SignedTransactionDataDto},
        Result,
    },
    types::{
        block::output::{NftId, OutputId},
        TryFromDto,
    },
};
use std::fs;
use tokio::fs::OpenOptions;

#[tokio::main]
async fn main() -> Result<()> {
    let paths = fs::read_dir("signed").unwrap();

    let mut nft_ids = Vec::new();

    for path in paths {
        let path = path.unwrap().path();
        let signed_transaction = read_signed_transaction_from_file(&path).await?;
        let nft_id = NftId::from(&OutputId::new(
            signed_transaction.transaction_payload.id(),
            0,
        )?);
        println!("nft_id {nft_id} {path:?}");
        nft_ids.push((nft_id, path));
    }

    nft_ids.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    println!("NFT ID with the lowest hash: {:?}", nft_ids[0]);
    write_tx_path_to_env_file(".env", nft_ids[0].1.clone()).await?;

    Ok(())
}

async fn read_signed_transaction_from_file(
    path: impl AsRef<std::path::Path>,
) -> Result<SignedTransactionData> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(path)
        .await
        .expect("failed to open file");
    let mut json = String::new();
    file.read_to_string(&mut json)
        .await
        .expect("failed to read file");

    let dto = serde_json::from_str::<SignedTransactionDataDto>(&json)?;

    Ok(SignedTransactionData::try_from_dto(dto)?)
}

async fn write_tx_path_to_env_file(
    path: impl AsRef<std::path::Path>,
    tx_path: std::path::PathBuf,
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

    file.write_all(format!("\nSIGNED_TX=\"{}\"", tx_path.display()).as_bytes())
        .await
        .expect("failed to write to file");
    file.flush().await.expect("failed to flush output stream");

    Ok(())
}
