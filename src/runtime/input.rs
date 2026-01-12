use core::fmt;
use std::collections::HashMap;

use crate::packet::{NodeId, Packet};

pub struct Test {
    pub nodes: Vec<NodeId>,

    //input
    pub input: HashMap<NodeId, Vec<String>>,

    pub image_name: &'static str,
    pub image_tag: &'static str,
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

impl fmt::Display for History {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut str = String::new();
        str.push_str("History(\n");
        for v in self.0.clone() {
            str.push_str(format!("{},\n", v).as_str());
        }
        str.push_str(")");
        f.write_str(&str)
    }
}

//Add all the assert implementations
