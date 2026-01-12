use std::fmt;

use serde::{Deserialize, Serialize};
use serde_with::serde_as;

pub type NodeId = String;

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Packet {
    Rpc(Rpc),
    Broadcast(Broadcast),
    Init(Init),
}

impl fmt::Display for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        return match self {
            Packet::Rpc(rpc) => {
                write!(f, "Rpc {{ src: {}, dst: {}, data: {} }}", rpc.src, rpc.dst, rpc.data)
            },
            Packet::Broadcast(broadcast) => {
                write!(f, "Broadcast {{ src: {}, data: {} }}", broadcast.src, broadcast.data)
            },
            Packet::Init(init) => {
                write!(f, "Init {{ node_id: {}, data: {} }}", init.node_id, init.data)
            },
        };
    }
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
    pub fn data(&self) -> String {
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
    pub data: String,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Broadcast {
    pub src: NodeId,
    pub data: String,
}

#[serde_as]
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub struct Init {
    pub node_id: NodeId,
    pub data: String,
}
