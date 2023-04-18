use apibara_core::starknet::v1alpha2::{Block, Event, EventWithTransaction};

pub struct BlockBuilder {
    block: Block,
}


impl BlockBuilder {
    pub fn init() -> Self {
        Self {
            block: Block {
            status: 0,
            header: Some(apibara_core::starknet::v1alpha2::BlockHeader {
                block_hash: None,
                parent_block_hash: None,
                block_number: 1,
                sequencer_address: None,
                new_root: None,
                timestamp: None,
            }),
            transactions: vec![],
            state_update: None,
            events: vec![],
            l2_to_l1_messages: vec![],
            }
        }

    }

    pub fn with_events(&mut self, events: Vec<EventWithTransaction>) -> &mut Self {
        self.block.events = events;
        self
    }

    pub fn build(&mut self) -> Block {
        self.block.clone()
    }
}
