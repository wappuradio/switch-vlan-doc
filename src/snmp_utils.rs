use snmp::{SyncSession, Value};
use std::time::Duration;
use anyhow::{Result, anyhow};
use std::collections::HashMap;

pub fn create_session(agent_addr: &str, community: &[u8], timeout: Duration) -> Result<SyncSession> {
    SyncSession::new(agent_addr, community, Some(timeout), 0)
        .map_err(|e| anyhow!("Failed to create SNMP session: {:?}", e))
}

#[derive(Debug)]
enum SnmpValue {
    Bytes(Vec<u8>),
    Integer(u32),
}

fn get_table_values(session: &mut SyncSession, base_oid: &[u32]) -> Result<HashMap<u32, SnmpValue>> {
    let mut results = HashMap::new();
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
            let value = match value {
                Value::OctetString(bytes) => SnmpValue::Bytes(bytes.to_vec()),
                Value::Integer(n) => SnmpValue::Integer(n as u32),
                Value::Unsigned32(n) => SnmpValue::Integer(n),
                _ => continue,
            };
            let last_id = extract_last_id(&oid_vec);
            results.insert(last_id as u32, value);
        } else {
            break;
        }
    }

    Ok(results)
}

pub fn get_u32_table(session: &mut SyncSession, base_oid: &[u32]) -> Result<HashMap<u32, u32>> {
    Ok(get_table_values(session, base_oid)?
        .into_iter()
        .map(|(k, v)| match v {
            SnmpValue::Integer(n) => (k, n),
            SnmpValue::Bytes(v) => (k, if v.len() >= 4 {
                u32::from_be_bytes(v[..4].try_into().unwrap_or([0; 4]))
            } else {
                0
            }),
        })
        .collect())
}

pub fn get_string_table(session: &mut SyncSession, base_oid: &[u32]) -> Result<HashMap<u32, String>> {
    Ok(get_table_values(session, base_oid)?
        .into_iter()
        .map(|(k, v)| match v {
            SnmpValue::Bytes(v) => Ok((k, String::from_utf8_lossy(&v).to_string())),
            SnmpValue::Integer(_) => Err(anyhow!("Expected string (OctetString) value but got integer")),
        })
        .collect::<Result<HashMap<u32, String>>>()?)
}

pub fn get_optional_string_table(session: &mut SyncSession, base_oid: &[u32]) -> Result<HashMap<u32, Option<String>>> {
    Ok(get_table_values(session, base_oid)?
        .into_iter()
        .map(|(k, v)| match v {
            SnmpValue::Bytes(v) => {
                let s = String::from_utf8_lossy(&v).to_string();
                Ok((k, if s.is_empty() { None } else { Some(s) }))
            },
            SnmpValue::Integer(_) => Err(anyhow!("Expected string (OctetString) value but got integer")),
        })
        .collect::<Result<HashMap<u32, Option<String>>>>()?)
}

pub fn get_raw_table(session: &mut SyncSession, base_oid: &[u32]) -> Result<HashMap<u32, Vec<u8>>> {
    Ok(get_table_values(session, base_oid)?
        .into_iter()
        .map(|(k, v)| match v {
            SnmpValue::Bytes(v) => (k, v),
            SnmpValue::Integer(n) => (k, n.to_be_bytes().to_vec()),
        })
        .collect())
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