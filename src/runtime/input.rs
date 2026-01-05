use std::collections::HashMap;

use crate::packet::{NodeId, Packet};

pub struct Test {
    pub nodes: Vec<NodeId>,

    //input
    pub input: HashMap<NodeId, Vec<Vec<u8>>>,

    pub dockerfile_path: String,
    pub env: Vec<Env>,
    pub end_delay_secs: u64,
}

#[derive(Clone)]
pub struct Env {
    pub name: String,
    pub value: String,
}

#[derive(Debug)]
pub struct History(pub Vec<Packet>);

//Add all the assert implementations
