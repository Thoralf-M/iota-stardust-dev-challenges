//! Create a blowball.
//!
//! ```sh
//! cargo run --bin blowball --release
//! ```

use iota_sdk::client::{Client, Result};

const NODE_URL: &str = "https://api.testnet.shimmer.network";
const EXPLORER_URL: &str = "https://explorer.shimmer.network/testnet";

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().with_node(NODE_URL)?.finish().await?;

    let tip = client.get_tips().await?[0];

    let mut latest_block_id = tip;

    for _ in 0..1000 {
        let block = client
            .build_block()
            .with_tag("blowball".as_bytes().to_vec())
            .with_parents(vec![tip, latest_block_id])?
            .finish()
            .await?;

        let block_id = block.id();
        latest_block_id = block_id;
        println!("{}/block/{}", EXPLORER_URL, block_id);
    }

    Ok(())
}
