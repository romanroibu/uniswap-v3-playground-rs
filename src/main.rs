pub mod buffer;
pub mod event;
pub mod parser;

use futures::StreamExt;

const UNI_V3_DAI_USDC_POOL: &str = "5777d92f208679db4b9778590fa3cab3ac9e2168";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
	dotenv::dotenv().ok();

	let ws_url = &std::env::var("INFURA_WEBSOCKET_ENDPOINT").unwrap();

	let web3 = web3::Web3::new(web3::transports::ws::WebSocket::new(ws_url).await?);

	let contract_address = hex::decode(UNI_V3_DAI_USDC_POOL).unwrap();
	let contract_address = web3::types::H160::from_slice(&contract_address[..]);

	let contract = web3::contract::Contract::from_json(
		web3.eth(),
		contract_address,
		include_bytes!("contracts/uniswap_pool_abi.json"),
	)?;

	let swap_event_abi = contract.abi().events_by_name("Swap")?.first().unwrap();

	let swap_event_signature = swap_event_abi.signature();

	let mut block_stream = web3.eth_subscribe().subscribe_new_heads().await?;

	let mut buffer = buffer::ReorganizingBuffer::new(5);

	loop {
		let block = match block_stream.next().await {
			Some(Ok(block)) => block,
			_ => continue,
		};

		let block_number = match block.number {
			Some(number) => number,
			_ => continue,
		};

		let logs = web3
			.eth()
			.logs(
				web3::types::FilterBuilder::default()
					.block_hash(block.hash.unwrap())
					.address(vec![contract_address])
					.topics(Some(vec![swap_event_signature]), None, None, None)
					.build(),
			)
			.await?;

		let events = logs
			.into_iter()
			.map(|log| parser::SwapParser::parse(log, swap_event_abi))
			.collect::<Result<Vec<_>, _>>()?;

		println!("BLOCK {} - {} Swap Events", block_number, events.len());

		match buffer.push((block_number.as_u64(), events)) {
			Ok(Some((block_number, events))) =>
				if !events.is_empty() {
					println!("---");
					println!("CONFIRMED EVENTS FROM BLOCK {}:", block_number);
					for event in events {
						println!("- {}", event.to_string());
					}
					println!("---");
				},
			Ok(None) => (),
			Err(buffer::ReorganizingBufferError::DepthExceeded(depth)) => {
				println!(
					"WARNING: Maximal reorganization depth {} exceeded ({}). Terminating.",
					buffer.depth, depth,
				);
				return Ok(());
			},
			Err(buffer::ReorganizingBufferError::MissingOffset(expected_block_number)) => {
				println!("WARNING: Skipped block number {}. Terminating.", expected_block_number,);
				return Ok(());
			},
		}
	}
}
