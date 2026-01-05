use anyhow::anyhow;

use crate::packet::Packet;

pub struct LineDecoder {
    buffer: Vec<u8>,
}

impl LineDecoder {
    pub fn new() -> Self {
        LineDecoder { buffer: vec![] }
    }

    pub fn add_to_buffer(&mut self, line: String) -> Option<anyhow::Result<Packet>> {
        self.buffer.extend_from_slice(line.as_bytes());
        if line == "}\n".to_string() {
            //decode
            let result = serde_json::from_slice::<Packet>(&self.buffer);
            return Some(result.map_err(|e| anyhow!(e)));
        }
        None
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

#[test]
fn test_clear() {
    let mut d = LineDecoder {
        buffer: vec![1, 2, 3],
    };
    d.clear();
    assert!(d.buffer.is_empty())
}

#[test]
fn test_single_add() {
    let mut d = LineDecoder::new();
    let result = d.add_to_buffer("line".to_string());
    assert!(result.is_none());
}

#[test]
fn test_json_add() {
    let mut d = LineDecoder::new();
    let result1 = d.add_to_buffer("{\n".to_string());
    let result2 = d.add_to_buffer("}\n".to_string());
    assert!(result1.is_none());
    assert!(result2.is_some());
    assert!(result2.unwrap().is_err());
}

#[test]
fn test_packet_decode_is_success() {
    let mut d = LineDecoder::new();
    let result1 = d.add_to_buffer("{\n".to_string());
    let result2 = d.add_to_buffer("  \"type\": \"rpc\",".to_string());
    let result3 = d.add_to_buffer("  \"src\": \"hi\",".to_string());
    let result4 = d.add_to_buffer("  \"dst\": \"todo!()\",".to_string());
    let result5 = d.add_to_buffer("  \"data\": \"YWRzZmFz\"".to_string());
    let result6 = d.add_to_buffer("}\n".to_string());
    assert!(result1.is_none());
    assert!(result2.is_none());
    assert!(result3.is_none());
    assert!(result4.is_none());
    assert!(result5.is_none());
    assert!(result6.is_some());
    assert!(result6.unwrap().is_ok());
}
