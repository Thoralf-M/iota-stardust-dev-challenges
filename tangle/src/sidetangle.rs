//! Create a sidetangle.
//!
//! ```sh
//! cargo run --bin sidetangle --release
//! ```

use iota_sdk::client::{Client, Result};
use std::collections::VecDeque;

const NODE_URL: &str = "https://api.testnet.shimmer.network";
const EXPLORER_URL: &str = "https://explorer.shimmer.network/testnet";

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().with_node(NODE_URL)?.finish().await?;

    let tips = client.get_tips().await?;

    let mut latest_block_ids = VecDeque::from(tips);

    for _ in 0..1000 {
        let block = client
            .build_block()
            .with_tag("sidetangle".as_bytes().to_vec())
            .with_parents(vec![
                latest_block_ids.pop_front().unwrap(),
                latest_block_ids[0],
                // using a different number of parents or the way they're inserted can result in a very different structure
                // latest_block_ids[1],
                // latest_block_ids[2],
            ])?
            .finish()
            .await?;

        let block_id = block.id();
        latest_block_ids.push_back(block_id);
        println!("{}/block/{}", EXPLORER_URL, block_id);
    }

    Ok(())
}
