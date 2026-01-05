use std::{collections::HashMap, time::Duration};

use anyhow::anyhow;
use futures::TryFutureExt;
use testcontainers::bollard::fsutil::types::packet;
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    time::timeout,
};

/*
 * What to do here?
 * 1. we get inputs, checkers...
 * 2. Run test by feeding inputs to the containers respectively
 * 3. save it to history
 * 4. assert with the checkers
 */
use crate::{
    packet::{Broadcast, Init, NodeId, Packet, Rpc},
    runtime::{
        container::RunnableContainer,
        input::{Env, History, Test},
    },
    util::ErrorLoggable,
};

mod container;
mod input;
mod line_decoder;
#[cfg(test)]
mod test;

pub struct Runtime<C: RunnableContainer> {
    pub(crate) containers: HashMap<NodeId, C>,
}

//Launch nodes in test. But how can I get binary image?
impl<C: RunnableContainer> Runtime<C> {
    pub fn new() -> Self {
        Self {
            containers: HashMap::new(),
        }
    }

    pub async fn launch_test(&mut self, t: Test) -> anyhow::Result<History> {
        let (tx, rx) = oneshot::channel();

        self.launch_all_nodes(&t).await?;

        // consume at interconnect nodes but at the same time, gatehr hisory
        //connect all outputs to history gather
        self.interconnect_nodes(tx, Duration::from_secs(t.end_delay_secs))
            .await?;

        //send init packets
        for (node_name, input_packets) in t.input {
            if let Some(container) = self.containers.get(&node_name) {
                let input_tx = container.stdin_tx().clone();
                for packet in input_packets {
                    let input_tx = input_tx.clone();

                    //TODO: fix how to pass init packets
                    let node_name = node_name.clone();
                    tokio::spawn(async move {
                        input_tx
                            .send(Packet::Init(Init {
                                node_id: node_name,
                                data: packet,
                            }))
                            .await
                            .unwrap()
                    });
                }
            }
        }

        rx.await.map_err(|e| anyhow!(e))
    }

    async fn launch_all_nodes(&mut self, t: &Test) -> anyhow::Result<()> {
        for node_name in &t.nodes {
            self.launch_node(t.dockerfile_path.clone(), t.env.clone(), node_name.clone())
                .await?;
        }
        Ok(())
    }

    async fn launch_node(
        &mut self,
        docker_file_path: String,
        env: Vec<Env>,
        node_name: NodeId,
    ) -> anyhow::Result<()> {
        let c = C::launch(docker_file_path, env, node_name.clone())
            .map_err(|e| anyhow!(e))
            .await?;
        self.containers.insert(node_name, c);
        Ok(())
    }

    async fn interconnect_nodes(
        &self,
        tx: oneshot::Sender<History>,
        timeout_duration: Duration,
    ) -> anyhow::Result<()> {
        let mut stdin_txs: HashMap<String, mpsc::Sender<Packet>> = HashMap::new();
        let mut stdouts: HashMap<String, mpsc::Receiver<Packet>> = HashMap::new();

        for (container_name, container) in &self.containers {
            let (tx, rx) = mpsc::channel(50);
            container.subscribe_stdout(tx);

            stdin_txs.insert(container_name.clone(), container.stdin_tx());
            stdouts.insert(container_name.clone(), rx);
        }

        let (history_packet_tx, history_packet_rx) = mpsc::channel(100);

        for (_, output_rx) in stdouts {
            //launch a task per container
            let inputs = stdin_txs.clone();
            let history_packet_tx = history_packet_tx.clone();

            tokio::spawn(async move {
                let mut output_rx = output_rx;
                while let Some(packet) = output_rx.recv().await {
                    history_packet_tx.send(packet.clone()).await.log_on_error();
                    match packet {
                        Packet::Rpc(_) => {
                            if let Some(input_tx) =
                                inputs.get(&packet.dst().unwrap_or(String::new()))
                            {
                                let input_tx = input_tx.clone();
                                let packet = packet.clone();
                                tokio::spawn(async move { input_tx.send(packet).await.unwrap() });
                            }
                        }
                        Packet::Broadcast(_) => {
                            for (node_id, input_tx) in &inputs {
                                if node_id != &packet.src().unwrap_or(String::new()) {
                                    let input_tx = input_tx.clone();
                                    let packet = packet.clone();
                                    tokio::spawn(
                                        async move { input_tx.send(packet).await.unwrap() },
                                    );
                                }
                            }
                        }
                        _ => {} //Init
                    }
                }
            });
        }

        tokio::spawn(gather_node_outputs(history_packet_rx, tx, timeout_duration));

        Ok(())
    }
}

async fn gather_node_outputs(
    mut history_packet_rx: mpsc::Receiver<Packet>,
    result_tx: oneshot::Sender<History>,
    timeout_duration: Duration,
) {
    let mut history = vec![];
    //gather all outputs to history
    loop {
        let packet = timeout(timeout_duration, history_packet_rx.recv()).await;
        match packet {
            Ok(Some(packet)) => {
                history.push(packet);
            }
            _ => {
                break;
            }
        }
    }
    result_tx.send(History(history)).log_on_error();
}
