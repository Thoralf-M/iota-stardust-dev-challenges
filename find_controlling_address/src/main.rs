//! Get the controlling Ed25519 address.
//!
//! ```sh
//! cargo run --release
//! ```

use std::str::FromStr;

use iota_sdk::{
    client::{Client, Result},
    types::block::{
        address::Address,
        output::{AliasTransition, OutputId},
    },
};

const NODE_URL: &str = "https://api.testnet.shimmer.network";
const OUTPUT_ID: &str = "0xdd5b0f0f305e4dac2b1c66774193dbbcc43d0f03b85a26da3755e5db5d0d55160000";
// Controlled by alias 0x86c9a6c9abbc0206498bd08517352b92a022f1b5f6596237a31761101d1fa447
// Controlled by NFT 0xcc5604bd572fa7db29cc158cdd9c39a3ea9d6b50c759b62acbe9911b7efe0b9a
// Controlled by alias 0x3e530ac4b2e1433147decf5825b92ea6e3345037c9c47a3f58ebc63d752f4f97
// Controlled by NFT 0x1cabb2d4829d9abb64817c5f61a241fcdc0d2f0830d9262c13a5447d8669708a
// Controlled by NFT 0x2dfeb9bce9abc696fa5f83f1f50c8a07d602df0fcd5ee810794680815449c9ff
// Controlled by Ed25519 0xdb5faa18645e7279802d0699ef7887871edacb047e697a80e06ac8a8d7496026

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().with_node(NODE_URL)?.finish().await?;

    let mut output = client.get_output(&OutputId::from_str(OUTPUT_ID)?).await?;

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("time went backwards")
        .as_secs() as u32;

    for _ in 0..1000 {
        let required_address = output.output().required_and_unlocked_address(
            current_time,
            output.metadata().output_id(),
            Some(AliasTransition::State),
        )?;
        match required_address.0 {
            Address::Alias(a) => {
                println!("Controlled by alias {}", a.alias_id());
                let output_id = client.alias_output_id(*a.alias_id()).await?;
                output = client.get_output(&output_id).await?;
            }
            Address::Nft(a) => {
                println!("Controlled by NFT {}", a.nft_id());
                let output_id = client.nft_output_id(*a.nft_id()).await?;
                output = client.get_output(&output_id).await?;
            }
            Address::Ed25519(a) => {
                println!("Controlled by Ed25519 {a}");
                return Ok(());
            }
        }
    }

    Ok(())
}
