use testcontainers::{
    ContainerAsync, GenericBuildableImage, GenericImage, ImageExt,
    bollard::query_parameters::AttachContainerOptions,
    runners::{AsyncBuilder, AsyncRunner},
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
        docker_file_path: String,
        env: Vec<Env>,
        node_name: String,
    ) -> anyhow::Result<Self>;
    fn subscribe_stdout(&self, output_tx: mpsc::Sender<Packet>);
    fn stdin_tx(&self) -> mpsc::Sender<Packet>;
    async fn stop(&self);
}

struct Container {
    inner_container: ContainerAsync<GenericImage>,
    input_tx: mpsc::Sender<Packet>,
}

impl RunnableContainer for Container {
    async fn launch(
        docker_file_path: String,
        env: Vec<Env>,
        node_name: String,
    ) -> anyhow::Result<Self> {
        //launch container from docker file path with env
        let image = GenericBuildableImage::new("test", "0.0.1")
            .with_dockerfile(docker_file_path)
            .build_image()
            .await?;

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

        tokio::spawn(async move {
            while let Some(packet) = input_rx.recv().await {
                let input_str = serde_json::to_string(&packet).unwrap();
                stdin
                    .write_all(input_str.into_bytes().as_slice())
                    .await
                    .unwrap();
                stdin.flush().await.unwrap();
            }
        });

        Ok(Container {
            inner_container: container,
            input_tx,
        })
    }

    fn subscribe_stdout(&self, output_tx: mpsc::Sender<Packet>) {
        let mut stdout = self.inner_container.stdout(true).lines();
        tokio::spawn(async move {
            let mut decoder = LineDecoder::new();
            let output_tx = output_tx.clone();
            while let Some(line) = stdout.next_line().await.unwrap() {
                if let Some(result) = decoder.add_to_buffer(line.clone()) {
                    match result {
                        Ok(p) => {
                            output_tx.send(p);
                            decoder.clear();
                        }
                        Err(e) => {
                            eprintln!("Failed to decode packet: {}", e);
                        }
                    }
                }
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
        _docker_file_path: String,
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
