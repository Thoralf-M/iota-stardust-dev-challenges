//! Get an input and prepare a transaction in a loop and check the nft id it would generate.
//!
//! ```sh
//! cargo run --bin mine_transaction --release
//! ```

use iota_sdk::{
    client::{
        api::{
            verify_semantic, PreparedTransactionData, SignedTransactionData,
            SignedTransactionDataDto,
        },
        constants::SHIMMER_COIN_TYPE,
        secret::{types::InputSigningData, SecretManage, SecretManager},
        Client, Result,
    },
    crypto::keys::bip44::Bip44,
    types::block::{
        address::Bech32Address,
        input::Input,
        output::{
            feature::{Irc27Metadata, IssuerFeature, MetadataFeature, TagFeature},
            unlock_condition::AddressUnlockCondition,
            BasicOutputBuilder, InputsCommitment, NftId, NftOutputBuilder, OutputId,
        },
        payload::{
            transaction::{RegularTransactionEssence, TransactionEssence},
            TransactionPayload,
        },
        semantic::ConflictReason,
    },
};

const ADDRESS_FILE_NAME: &str = "address.json";
const NODE_URL: &str = "https://api.testnet.shimmer.network";
const NFT_ID_START_BYTES: &[u8] = &[0, 0];

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let threads = 16;
    let total_rounds = 1_000_000;
    // Increase offset in subsequent runs to not check the same txs again
    let offset = 0;
    let amount = total_rounds / threads;

    let _ = std::fs::create_dir("signed");

    let mut tasks = Vec::new();

    for i in 0..threads {
        tasks.push(tokio::spawn(async move {
            if i == threads - 1 {
                check_transaction(
                    offset + i * amount,
                    offset + ((i + 1) * amount) + total_rounds % threads,
                )
                .await
            } else {
                check_transaction(offset + i * amount, offset + (i + 1) * amount).await
            }
        }));
    }

    let results = futures::future::try_join_all(tasks).await?;

    for res in results {
        res?;
    }

    Ok(())
}
async fn check_transaction(start: u32, end: u32) -> Result<()> {
    println!("checking range: {start}..{end}");

    let client = Client::builder().with_node(NODE_URL)?.finish().await?;

    let rent_structure = client.get_rent_structure().await?;
    let network_id = client.get_network_id().await?;
    let token_supply = client.get_token_supply().await?;
    let protocol_parameters = client.get_protocol_parameters().await?;

    // Recovers address from the previous example.
    let address = read_address_from_file(ADDRESS_FILE_NAME).await?;

    // currently assumes only a single input
    let inputs = client.find_inputs(vec![address], 2_000_000).await?;
    if inputs.is_empty() {
        panic!("No input found for address {address}");
    }
    let input_output = client.get_output(inputs[0].output_id()).await?;

    let inputs_data = vec![InputSigningData {
        output_metadata: *input_output.metadata(),
        output: input_output.output().clone(),
        chain: Some(Bip44::new(SHIMMER_COIN_TYPE).with_address_index(0)),
    }];

    let inputs_commitment = InputsCommitment::new([input_output.output()].into_iter());

    let mut essence = RegularTransactionEssence::builder(network_id, inputs_commitment);
    essence = essence.with_inputs(vec![Input::Utxo(inputs[0])]);

    let nft_builder =
        NftOutputBuilder::new_with_minimum_storage_deposit(rent_structure, NftId::null())
            .add_unlock_condition(AddressUnlockCondition::new(address))
            .add_immutable_feature(IssuerFeature::new(address))
            .add_immutable_feature(MetadataFeature::try_from(
                Irc27Metadata::new(
                    "video/mp4",
                    "https://www.youtube.com/watch?v=dQw4w9WgXcQ"
                        .parse()
                        .unwrap(),
                    "Can you mine a better id? :trollface:",
                )
                .with_description("Watch the video to see how I did it!"),
            )?);
    let secret_manager = SecretManager::try_from_mnemonic(std::env::var("MNEMONIC").unwrap())?;

    for i in start..end {
        let nft = nft_builder
            .clone()
            .add_feature(TagFeature::new(i.to_be_bytes())?)
            .finish_output(token_supply)?;
        let remainder =
            BasicOutputBuilder::new_with_amount(input_output.output().amount() - nft.amount())
                .add_unlock_condition(AddressUnlockCondition::new(address))
                .finish_output(token_supply)?;
        let outputs = vec![nft, remainder];

        let prepared_transaction_data = {
            let essence = essence.clone().with_outputs(outputs);
            let regular_essence = essence.finish_with_params(protocol_parameters.clone())?;

            PreparedTransactionData {
                essence: TransactionEssence::Regular(regular_essence),
                inputs_data: inputs_data.clone(),
                remainder: None,
            }
        };

        let unlocks = secret_manager
            .sign_transaction_essence(&prepared_transaction_data, None)
            .await?;
        let signed_transaction =
            TransactionPayload::new(prepared_transaction_data.essence.clone(), unlocks)?;

        let nft_id = NftId::from(&OutputId::new(signed_transaction.id(), 0)?);

        // extra check for the first iteration to make sure that it would actually create valid transactions
        if i == 0 {
            let signed_transaction_data = SignedTransactionData {
                transaction_payload: signed_transaction.clone(),
                inputs_data: prepared_transaction_data.inputs_data.clone(),
            };
            let current_time = client.get_time_checked().await?;

            let conflict = verify_semantic(
                &signed_transaction_data.inputs_data,
                &signed_transaction_data.transaction_payload,
                current_time,
            )?;

            if conflict != ConflictReason::None {
                return Err(iota_sdk::client::Error::TransactionSemantic(conflict));
            }
        }

        if &nft_id.as_slice()[0..NFT_ID_START_BYTES.len()] == NFT_ID_START_BYTES {
            let signed_transaction_data = SignedTransactionData {
                transaction_payload: signed_transaction,
                inputs_data: prepared_transaction_data.inputs_data,
            };
            println!("nft_id: {nft_id} round {i}");
            write_signed_transaction_to_file(
                format!("signed/{i}_signed_transaction.json"),
                &signed_transaction_data,
            )
            .await?;
        }
    }
    Ok(())
}

async fn read_address_from_file(path: impl AsRef<std::path::Path>) -> Result<Bech32Address> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(&path)
        .await
        .expect("failed to open file");
    let mut json = String::new();
    file.read_to_string(&mut json)
        .await
        .expect("failed to read file");

    Ok(serde_json::from_str(&json)?)
}

async fn write_signed_transaction_to_file(
    path: impl AsRef<std::path::Path>,
    signed_transaction_data: &SignedTransactionData,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let dto = SignedTransactionDataDto::from(signed_transaction_data);
    let json = serde_json::to_string_pretty(&dto)?;
    let mut file = tokio::io::BufWriter::new(
        tokio::fs::File::create(path)
            .await
            .expect("failed to create file"),
    );

    file.write_all(json.as_bytes())
        .await
        .expect("failed to write file");
    file.flush().await.expect("failed to flush output stream");

    Ok(())
}
