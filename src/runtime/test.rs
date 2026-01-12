use std::{collections::HashMap, time::Duration};

use tokio::sync::{mpsc, oneshot};

use crate::{
    packet::{Broadcast, Packet, Rpc},
    runtime::{Runtime, container::MockContainer, input::Test},
};

#[tokio::test]
async fn test_runtime_launch_creates_and_saves_containers() {
    // Arrange
    let mut runtime = Runtime::<MockContainer>::new();
    let nodenames = vec!["node1", "node2", "node3"];

    // Act
    let result = runtime
        .launch_test(Test {
            nodes: nodenames.iter().map(|s| s.to_string()).collect(),
            input: HashMap::new(),
            image_name: "",
            image_tag: "",
            env: vec![],
            end_delay_secs: 2,
        })
        .await;

    assert!(result.is_ok());

    // Check if the number of containers matches
    assert_eq!(
        runtime.containers.len(),
        nodenames.len(),
        "Number of containers should match number of nodenames"
    );

    // Check if each nodename exists in the containers map
    for name in nodenames {
        assert!(
            runtime.containers.contains_key(name),
            "Container for {} should exist",
            name
        );
    }
}

//test sending rpc packet
#[tokio::test]
async fn test_runtime_sends_and_receives_rpc_packets() {
    // Arrange
    let mut runtime = Runtime::<MockContainer>::new();
    let nodenames = vec!["node1", "node2", "node3"];

    runtime.containers = nodenames
        .iter()
        .map(|name| (name.to_string(), MockContainer::new(name.to_string())))
        .collect();

    runtime
        .containers
        .get_mut("node1")
        .unwrap()
        .expected_stdout_packets = Some(vec![Packet::Rpc(Rpc {
        src: "node1".to_string(),
        dst: "node2".to_string(),
        data: String::new(),
    })]);

    let (stdin_tx, mut stdin_rx) = mpsc::channel(10);
    runtime.containers.get_mut("node2").unwrap().expected_stdin = Some(stdin_tx);

    let (tx, rx) = oneshot::channel();
    // Act
    let result = runtime.interconnect_nodes(tx, Duration::from_secs(2)).await;

    assert!(result.is_ok());
    // Check if the rpc packet is received by node2
    let received_packet = stdin_rx.recv().await;
    match received_packet {
        Some(Packet::Rpc(rpc)) => {
            assert_eq!(rpc.src, "node1");
            assert_eq!(rpc.dst, "node2");
        }
        _ => {
            assert!(false); // failed
        }
    }

    assert!(rx.await.unwrap().0.len() == 1);
}

//test sending broadcast packet
#[tokio::test]
async fn test_runtime_sends_and_receives_broadcast_packets() {
    let mut runtime = Runtime::<MockContainer>::new();
    let nodenames = vec!["node1", "node2", "node3"];

    runtime.containers = nodenames
        .iter()
        .map(|name| (name.to_string(), MockContainer::new(name.to_string())))
        .collect();

    runtime
        .containers
        .get_mut("node1")
        .unwrap()
        .expected_stdout_packets = Some(vec![Packet::Broadcast(Broadcast {
        src: "node1".to_string(),
        data: String::new(),
    })]);

    let mut stdins = vec![];

    for node in &["node2", "node3"] {
        let (stdin_tx, stdin_rx) = mpsc::channel(10);
        runtime.containers.get_mut(*node).unwrap().expected_stdin = Some(stdin_tx);
        stdins.push((node.to_string(), stdin_rx));
    }

    let (tx, rx) = oneshot::channel();
    // Act
    let result = runtime.interconnect_nodes(tx, Duration::from_secs(2)).await;

    assert!(result.is_ok());

    for (_, mut stdin_rx) in stdins {
        let received_packet = stdin_rx.recv().await;
        match received_packet {
            Some(Packet::Broadcast(broadcast)) => {
                assert_eq!(broadcast.src, "node1");
            }
            _ => {
                assert!(false); // failed
            }
        }
    }

    assert!(rx.await.unwrap().0.len() == 1);
}

//multi node test...
