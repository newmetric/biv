use anyhow::anyhow;

use crate::packet::Packet;

pub struct LineDecoder {
    buffer: Vec<u8>,
    brace_depth: i32,
    started: bool, // Track if we've seen an opening brace
}

impl LineDecoder {
    pub fn new() -> Self {
        LineDecoder {
            buffer: vec![],
            brace_depth: 0,
            started: false,
        }
    }

    pub fn add_to_buffer(&mut self, line: String) -> Option<anyhow::Result<Packet>> {
        self.buffer.extend_from_slice(line.as_bytes());

        // Track brace depth to know when we have a complete JSON object
        let mut in_string = false;
        let mut escape_next = false;

        for ch in line.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match ch {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '{' if !in_string => {
                    self.brace_depth += 1;
                    self.started = true;
                }
                '}' if !in_string => self.brace_depth -= 1,
                _ => {}
            }
        }

        // When brace depth returns to 0 after starting, we have a complete JSON object
        if self.started && self.brace_depth == 0 {
            let result = serde_json::from_slice::<Packet>(&self.buffer);
            eprintln!("current buffer: {}", String::from_utf8(self.buffer.clone()).unwrap());
            return Some(result.map_err(|e| anyhow!(e)));
        }

        None
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.brace_depth = 0;
        self.started = false;
    }
}

#[test]
fn test_clear() {
    let mut d = LineDecoder {
        buffer: vec![1, 2, 3],
        brace_depth: 2,
        started: true,
    };
    d.clear();
    assert!(d.buffer.is_empty());
    assert_eq!(d.brace_depth, 0);
    assert!(!d.started);
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

#[test]
fn test_single_line_json() {
    let mut d = LineDecoder::new();
    let result = d.add_to_buffer("{\"type\": \"rpc\", \"src\": \"hi\", \"dst\": \"todo!()\", \"data\": \"YWRzZmFz\"}\n".to_string());
    assert!(result.is_some());
    assert!(result.unwrap().is_ok());
}

#[test]
fn test_json_with_braces_in_string() {
    let mut d = LineDecoder::new();
    // JSON with braces inside string values should not confuse the parser
    let result = d.add_to_buffer("{\"type\": \"rpc\", \"src\": \"hi{}\", \"dst\": \"todo!()\", \"data\": \"YWRzZmFz\"}\n".to_string());
    assert!(result.is_some());
    assert!(result.unwrap().is_ok());
}

#[test]
fn test_brace_depth_tracking() {
    let mut d = LineDecoder::new();
    d.add_to_buffer("{".to_string());
    assert_eq!(d.brace_depth, 1);
    d.add_to_buffer("  \"nested\": {".to_string());
    assert_eq!(d.brace_depth, 2);
    d.add_to_buffer("  }".to_string());
    assert_eq!(d.brace_depth, 1);
}
