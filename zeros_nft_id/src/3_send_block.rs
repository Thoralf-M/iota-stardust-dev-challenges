//! Send the signed transaction in a block.
//!
//! ```sh
//! cargo run --bin send_block --release
//! ```

use iota_sdk::{
    client::{
        api::{verify_semantic, SignedTransactionData, SignedTransactionDataDto},
        Client, Error, Result,
    },
    types::{
        block::{payload::Payload, semantic::ConflictReason},
        TryFromDto,
    },
};

const NODE_URL: &str = "https://api.testnet.shimmer.network";
const EXPLORER_URL: &str = "https://explorer.shimmer.network/testnet";

#[tokio::main]
async fn main() -> Result<()> {
    // This example uses secrets in environment variables for simplicity which should not be done in production.
    dotenvy::dotenv().ok();

    let client = Client::builder().with_node(NODE_URL)?.finish().await?;

    let signed_transaction_payload =
        read_signed_transaction_from_file(std::env::var("SIGNED_TX").unwrap()).await?;

    let current_time = client.get_time_checked().await?;

    let conflict = verify_semantic(
        &signed_transaction_payload.inputs_data,
        &signed_transaction_payload.transaction_payload,
        current_time,
    )?;

    if conflict != ConflictReason::None {
        return Err(Error::TransactionSemantic(conflict));
    }

    let block = client
        .build_block()
        .finish_block(Some(Payload::Transaction(Box::new(
            signed_transaction_payload.transaction_payload,
        ))))
        .await?;

    println!("Posted block: {}/block/{}", EXPLORER_URL, block.id());

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
