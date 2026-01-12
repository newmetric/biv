use testcontainers::core::WaitFor;
use testcontainers::{
    ContainerAsync, GenericImage, ImageExt,
    bollard::query_parameters::AttachContainerOptions,
    runners::AsyncRunner,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

use crate::{
    packet::Packet,
    runtime::{input::Env, line_decoder::LineDecoder},
};

pub trait RunnableContainer
where
    Self: Sized + Send,
{
    async fn launch(
        image_name: &'static str,
        image_tag: &'static str,
        env: Vec<Env>,
        node_name: String,
    ) -> anyhow::Result<Self>;
    fn subscribe_stdout(&self, output_tx: mpsc::Sender<Packet>);
    fn stdin_tx(&self) -> mpsc::Sender<Packet>;
    async fn stop(&self);
}

pub struct Container {
    node_name: String,
    inner_container: ContainerAsync<GenericImage>,
    input_tx: mpsc::Sender<Packet>,
}

impl RunnableContainer for Container {
    async fn launch(
        image_name: &'static str,
        image_tag: &'static str,
        env: Vec<Env>,
        node_name: String,
    ) -> anyhow::Result<Self> {
        //launch container from docker file path with env

        let image = GenericImage::new(image_name, image_tag)
        .with_wait_for(WaitFor::millis(1000));

        let mut container_req = image.with_container_name(&node_name).with_open_stdin(true);
        for e in env {
            container_req = container_req.with_env_var(e.name, e.value);
        }
        let container = container_req
            .start()
            .await
            .expect("Failed to start container");

        let docker_client = testcontainers::core::client::docker_client_instance()
            .await
            .unwrap();
        let result = docker_client
            .attach_container(
                &node_name,
                Some(AttachContainerOptions {
                    stdin: true,
                    detach_keys: None,
                    logs: false,
                    stream: true,
                    stdout: false,
                    stderr: false,
                }),
            )
            .await
            .unwrap();

        let mut stdin = result.input;

        let (input_tx, mut input_rx) = tokio::sync::mpsc::channel(10);

        let cloned_node_name = node_name.clone();
        tokio::spawn(async move {
            while let Some(packet) = input_rx.recv().await {
                let input_str = serde_json::to_string(&packet).unwrap() + "\n";
                eprintln!("{} stdin: {}", cloned_node_name, input_str);
                stdin
                    .write_all(input_str.into_bytes().as_slice())
                    .await
                    .unwrap();
                stdin.flush().await.unwrap();
            }
        });

        Ok(Container {
            node_name,
            inner_container: container,
            input_tx,
        })
    }

    fn subscribe_stdout(&self, output_tx: mpsc::Sender<Packet>) {
        let mut stdout = self.inner_container.stdout(true).lines();
        let node_name = self.node_name.clone();
        tokio::spawn(async move {
            let mut decoder = LineDecoder::new();
            let output_tx = output_tx.clone();
            while let Some(line) = stdout.next_line().await.unwrap() {
                // eprintln!("{} stdout: {}", &node_name, line);
                if let Some(result) = decoder.add_to_buffer(line.clone()) {
                    match result {
                        Ok(p) => {
                            eprintln!("stdout packet");
                            output_tx.send(p).await.unwrap();
                            decoder.clear();
                        }
                        Err(e) => {
                            eprintln!("Failed to decode packet: {}", e);
                            decoder.clear();
                        }
                    }
                }
            }
        });

        let node_name = self.node_name.clone();
        let mut stderr = self.inner_container.stderr(true).lines();
        tokio::spawn(async move {
            while let Some(line) = stderr.next_line().await.unwrap() {
                eprintln!("{} stderr: {}", &node_name, line);
            }
        });
    }

    fn stdin_tx(&self) -> mpsc::Sender<Packet> {
        self.input_tx.clone()
    }

    async fn stop(&self) {
        self.inner_container
            .stop()
            .await
            .expect("Stop should be done without error.")
    }
}

#[cfg(test)]
pub struct MockContainer {
    pub node_name: String,
    pub expected_stdout_packets: Option<Vec<Packet>>,
    pub expected_stdin: Option<mpsc::Sender<Packet>>,
}

#[cfg(test)]
impl MockContainer {
    pub fn new(node_name: String) -> Self {
        MockContainer {
            node_name,
            expected_stdout_packets: None,
            expected_stdin: None,
        }
    }
}

#[cfg(test)]
impl RunnableContainer for MockContainer {
    async fn launch(
        _image_name: &'static str,
        _image_tag: &'static str,
        _env: Vec<Env>,
        node_name: String,
    ) -> anyhow::Result<Self> {
        Ok(MockContainer::new(node_name))
    }

    fn subscribe_stdout(&self, output_tx: mpsc::Sender<Packet>) {
        for packet in &self.expected_stdout_packets.clone().unwrap_or(vec![]) {
            output_tx.try_send(packet.clone()).unwrap();
        }
    }

    fn stdin_tx(&self) -> mpsc::Sender<Packet> {
        match &self.expected_stdin {
            Some(t) => t.clone(),
            None => {
                let (tx, _) = mpsc::channel(10);
                tx
            }
        }
    }

    async fn stop(&self) {}
}
