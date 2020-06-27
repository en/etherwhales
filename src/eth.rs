use serde_json::json;
use tiny_keccak::{Hasher, Keccak};
use warp::ws::Message;
use web3::futures::StreamExt;
use web3::types::{Address, FilterBuilder, H160, H256, U256};

use crate::Users;

/// Compute the Keccak-256 hash of input bytes.
fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    hasher.finalize(&mut output);
    output
}

pub async fn subscribe(endpoint: String, users: Users) -> web3::Result {
    let ws = web3::transports::WebSocket::new(&endpoint).await?;
    let web3 = web3::Web3::new(ws.clone());

    let bat: Address = "0x0D8775F648430679A709E98d2b0Cb6250d2887EF"
        .trim_start_matches("0x")
        .parse()
        .unwrap();
    let comp: Address = "0xc00e94Cb662C3520282E6f5717214004A7f26888"
        .trim_start_matches("0x")
        .parse()
        .unwrap();
    let lend: Address = "0x80fB784B7eD66730e8b1DBd9820aFD29931aab03"
        .trim_start_matches("0x")
        .parse()
        .unwrap();

    let log_transfer_sig = b"Transfer(address,address,uint256)";
    let log_transfer_sig_hash = keccak256(&log_transfer_sig[..]);

    let filter = FilterBuilder::default()
        .address(vec![bat, comp, lend])
        .topics(
            Some(vec![H256::from_slice(&log_transfer_sig_hash)]),
            None,
            None,
            None,
        )
        .build();

    let sub = web3.eth_subscribe().subscribe_logs(filter).await?;
    println!("Got subscription id: {:?}", sub.id());

    // Ok(Log { address: 0xc00e94cb662c3520282e6f5717214004a7f26888, topics: [0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef, 0x0000000000000000000000003f5ce5fbfe3e9af3971dd833d26ba9b5c936f0be, 0x00000000000000000000000084e1212fbd2af43ae525ec760830fecf4a06d046], data: Bytes([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 221, 251, 201, 115, 161, 61, 0, 0]), block_hash: Some(0xee7dc738e07ad53c0eb2b1b7d8d4eee3e87da1554d0adc761738153ec77e63e0), block_number: Some(10349643), transaction_hash: Some(0x9586d8c150930012203d5ebd69218014bc16f22d08943af89e05211aa1de3513), transaction_index: Some(8), log_index: Some(7), transaction_log_index: None, log_type: None, removed: Some(false) })
    sub.for_each(|log| async {
        let log = log.unwrap();
        // timestamp
        let symbol = if log.address == bat {
            "BAT"
        } else if log.address == comp {
            "COMP"
        } else if log.address == lend {
            "LEND"
        } else {
            "UNKNOWN"
        };

        let amount: U256 = log.data.0[..32].into();

        let amount = amount / U256::exp10(18);
        let amount = amount.low_u64(); // TODO: serde_json bug

        let from: H160 = log.topics[1].into();
        let to: H160 = log.topics[2].into();
        let message = json!({
            "timestamp": 0,
            "symbol": symbol,
            "amount": amount,
            "from": from,
            "to": to,
        });
        println!("{}", message);
        for (_, tx) in users.read().await.iter() {
            if let Err(disconnected) = tx.send(Ok(Message::text(message.to_string()))) {
                println!("{}", disconnected);
            }
        }
    })
    .await;

    Ok(())
}
