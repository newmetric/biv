use serde::{Deserialize, Serialize};
use serde_with::base64::Base64;
use serde_with::serde_as;

pub type NodeId = String;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Packet {
    Rpc(Rpc),
    Broadcast(Broadcast),
    Init(Init),
}

impl Packet {
    pub fn dst(&self) -> Option<NodeId> {
        match self {
            Packet::Rpc(rpc) => Some(rpc.dst.clone()),
            Packet::Init(init) => Some(init.node_id.clone()),
            _ => None,
        }
    }
    pub fn src(&self) -> Option<NodeId> {
        match self {
            Packet::Rpc(rpc) => Some(rpc.src.clone()),
            Packet::Broadcast(broadcast) => Some(broadcast.src.clone()),
            Packet::Init(_) => None,
        }
    }
    pub fn data(&self) -> Vec<u8> {
        match self {
            Packet::Rpc(rpc) => rpc.data.clone(),
            Packet::Broadcast(broadcast) => broadcast.data.clone(),
            Packet::Init(init) => init.data.clone(),
        }
    }
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Rpc {
    pub src: NodeId,
    pub dst: NodeId,
    #[serde_as(as = "Base64")]
    pub data: Vec<u8>,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Broadcast {
    pub src: NodeId,
    #[serde_as(as = "Base64")]
    pub data: Vec<u8>,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Init {
    pub node_id: NodeId,
    #[serde_as(as = "Base64")]
    pub data: Vec<u8>,
}
