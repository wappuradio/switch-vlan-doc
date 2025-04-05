use snmp::{SyncSession, Value};
use std::time::Duration;
use anyhow::{Result, anyhow};

pub fn create_session(agent_addr: &str, community: &[u8], timeout: Duration) -> Result<SyncSession> {
    SyncSession::new(agent_addr, community, Some(timeout), 0)
        .map_err(|e| anyhow!("Failed to create SNMP session: {:?}", e))
}

pub fn get_table_values(session: &mut SyncSession, base_oid: &[u32]) -> Result<Vec<(Vec<u32>, Vec<u8>)>> {
    let mut results = Vec::new();
    let mut current_oid = base_oid.to_vec();

    loop {
        let mut response = session.getnext(&current_oid)
            .map_err(|e| anyhow!("Failed to get next SNMP value: {:?}", e))?;

        if let Some((oid, value)) = response.varbinds.next() {
            let oid_str = format!("{}", oid);
            let oid_vec = parse_oid(&oid_str);
            
            // Check if we're still in the same table
            if !starts_with(&oid_vec, base_oid) {
                break;
            }

            current_oid = oid_vec.clone();
            let value_vec = match value {
                Value::OctetString(bytes) => bytes.to_vec(),
                Value::Integer(n) => n.to_be_bytes().to_vec(),
                _ => continue,
            };
            results.push((oid_vec, value_vec));
        } else {
            break;
        }
    }

    Ok(results)
}

pub fn extract_last_id(oid: &[u32]) -> u16 {
    oid.last()
        .map(|&n| n as u16)
        .unwrap_or(0)
}

fn parse_oid(oid_str: &str) -> Vec<u32> {
    oid_str.split('.')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

fn starts_with(oid: &[u32], prefix: &[u32]) -> bool {
    if oid.len() < prefix.len() {
        return false;
    }
    &oid[..prefix.len()] == prefix
}

pub fn decode_port_list(ports: &[u8]) -> String {
    let mut port_list = Vec::new();
    for (byte_index, &byte) in ports.iter().enumerate() {
        for bit_index in 0..8 {
            if (byte & (1 << (7 - bit_index))) != 0 {
                let port_number = byte_index * 8 + bit_index + 1;
                port_list.push(port_number.to_string());
            }
        }
    }
    port_list.join(", ")
} 