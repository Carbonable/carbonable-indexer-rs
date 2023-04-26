use apibara_core::starknet::v1alpha2::{
    transaction::Transaction::InvokeV1, Block, Event, EventWithTransaction, FieldElement,
    InvokeTransactionV1, Transaction,
};
use pbjson_types::Timestamp;

pub struct BlockBuilder {
    block: Block,
}

impl BlockBuilder {
    pub fn init() -> Self {
        Self {
            block: Block {
                status: 0,
                header: Some(apibara_core::starknet::v1alpha2::BlockHeader {
                    block_hash: Some(
                        FieldElement::from_hex(
                            "0x00e5b47eb49670292f1f7504488ea20cbf7381446a4761827476dbd9718dd7ed",
                        )
                        .unwrap(),
                    ),
                    parent_block_hash: None,
                    block_number: 1,
                    sequencer_address: None,
                    new_root: None,
                    timestamp: Some(Timestamp {
                        seconds: 10000,
                        nanos: 0,
                    }),
                }),
                transactions: vec![],
                state_update: None,
                events: vec![],
                l2_to_l1_messages: vec![],
            },
        }
    }

    pub fn with_events(&mut self, events: Vec<EventWithTransaction>) -> &mut Self {
        self.block.events = events;
        self
    }

    pub fn add_transfer_event(
        &mut self,
        tx_hash: &str,
        from: &str,
        event_key: &[u8; 32],
        data: Vec<FieldElement>,
    ) -> &mut Self {
        self.block.events.push(EventWithTransaction {
            transaction: Some(Transaction {
                meta: Some(apibara_core::starknet::v1alpha2::TransactionMeta {
                    hash: Some(FieldElement::from_hex(tx_hash).unwrap()),
                    max_fee: None,
                    signature: vec![],
                    nonce: None,
                    version: 100u64,
                }),
                transaction: Some(InvokeV1(InvokeTransactionV1 {
                    sender_address: Some(FieldElement::from_hex(from).unwrap()),
                    calldata: vec![],
                })),
            }),
            receipt: None,
            event: Some(Event {
                from_address: None,
                keys: vec![FieldElement::from_bytes(&event_key)],
                data,
            }),
        });
        self
    }

    pub fn build(&mut self) -> Block {
        self.block.clone()
    }
}
