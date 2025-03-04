// Copyright (c) 2022 MASSA LABS <info@massa.net>

use massa_models::{
    constants::CHANNEL_SIZE, signed::Signable, Block, BlockId, SerializeCompact, SignedHeader,
};
use massa_protocol_exports::{
    ProtocolCommand, ProtocolCommandSender, ProtocolEvent, ProtocolEventReceiver,
};
use massa_storage::Storage;
use massa_time::MassaTime;
use tokio::{sync::mpsc, time::sleep};

pub struct MockProtocolController {
    protocol_command_rx: mpsc::Receiver<ProtocolCommand>,
    protocol_event_tx: mpsc::Sender<ProtocolEvent>,
    storage: Storage,
}

impl MockProtocolController {
    pub fn new(storage: Storage) -> (Self, ProtocolCommandSender, ProtocolEventReceiver) {
        let (protocol_command_tx, protocol_command_rx) =
            mpsc::channel::<ProtocolCommand>(CHANNEL_SIZE);
        let (protocol_event_tx, protocol_event_rx) = mpsc::channel::<ProtocolEvent>(CHANNEL_SIZE);
        (
            MockProtocolController {
                protocol_event_tx,
                protocol_command_rx,
                storage,
            },
            ProtocolCommandSender(protocol_command_tx),
            ProtocolEventReceiver(protocol_event_rx),
        )
    }

    pub async fn wait_command<F, T>(&mut self, timeout: MassaTime, filter_map: F) -> Option<T>
    where
        F: Fn(ProtocolCommand) -> Option<T>,
    {
        let timer = sleep(timeout.into());
        tokio::pin!(timer);
        loop {
            tokio::select! {
                cmd_opt = self.protocol_command_rx.recv() => match cmd_opt {
                    Some(orig_cmd) => if let Some(res_cmd) = filter_map(orig_cmd) { return Some(res_cmd); },
                    None => panic!("Unexpected closure of protocol command channel."),
                },
                _ = &mut timer => return None
            }
        }
    }

    /// Note: if you care about the operation set, use another method.
    pub async fn receive_block(&mut self, block: Block) {
        let block_id = block.header.content.compute_id().unwrap();
        let serialize_block = block.to_bytes_compact().expect("Fail to serialize block.");
        self.storage
            .store_block(block_id, block.clone(), serialize_block);
        self.protocol_event_tx
            .send(ProtocolEvent::ReceivedBlock {
                block_id,
                slot: block.header.content.slot,
                operation_set: Default::default(),
                endorsement_ids: Default::default(),
            })
            .await
            .expect("could not send protocol event");
    }

    /// Retrieve a block from storage.
    pub fn get_block(&self, block_id: &BlockId) -> Option<Block> {
        self.storage
            .retrieve_block(block_id)
            .map(|stored_block| stored_block.read().block.clone())
    }

    pub async fn receive_header(&mut self, header: SignedHeader) {
        let block_id = header.content.compute_id().unwrap();
        self.protocol_event_tx
            .send(ProtocolEvent::ReceivedBlockHeader { block_id, header })
            .await
            .expect("could not send protocol event");
    }

    pub async fn receive_get_active_blocks(&mut self, list: Vec<BlockId>) {
        self.protocol_event_tx
            .send(ProtocolEvent::GetBlocks(list))
            .await
            .expect("could not send protocol event");
    }

    /// ignore all commands while waiting for a future
    pub async fn ignore_commands_while<FutureT: futures::Future + Unpin>(
        &mut self,
        mut future: FutureT,
    ) -> FutureT::Output {
        loop {
            tokio::select!(
                res = &mut future => return res,
                cmd = self.protocol_command_rx.recv() => match cmd {
                    Some(_) => {},
                    None => return future.await,  // if the protocol controlled dies, wait for the future to finish
                }
            );
        }
    }
}
