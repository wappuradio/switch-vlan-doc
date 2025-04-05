use snmp::{SyncSession, Value};
use std::collections::HashMap;
use std::time::Duration;
use anyhow::{Result, anyhow};

// Q-BRIDGE-MIB OIDs
const VLAN_STATIC_NAME: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,1];  // dot1qVlanStaticName
const VLAN_STATIC_EGRESS_PORTS: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,2];  // dot1qVlanStaticEgressPorts
const VLAN_STATIC_UNTAGGED_PORTS: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,4];  // dot1qVlanStaticUntaggedPorts
const PORT_VLAN_TABLE: &[u32] = &[1,3,6,1,2,1,17,7,1,4,5,1,1];  // dot1qPvid

fn main() -> Result<()> {
    let agent_addr = "10.1.0.24:161";
    let community = b"public";
    let timeout = Duration::from_secs(2);

    let mut sess = SyncSession::new(agent_addr, community, Some(timeout), 0)
        .map_err(|e| anyhow!("Failed to create SNMP session: {:?}", e))?;
    
    println!("Fetching VLAN information...\n");

    // Get VLAN names
    let mut vlan_names = HashMap::new();
    let mut current_oid = VLAN_STATIC_NAME.to_vec();
    loop {
        let mut response = sess.getnext(&current_oid)
            .map_err(|e| anyhow!("Failed to get next VLAN name: {:?}", e))?;
        if let Some((oid, Value::OctetString(name))) = response.varbinds.next() {
            let oid_str = format!("{}", oid);
            let oid_vec = parse_oid(&oid_str);
            // Check if we're still in the VLAN names table
            if !starts_with(&oid_vec, VLAN_STATIC_NAME) {
                break;
            }
            let vlan_id = extract_vlan_id(&oid_vec);
            vlan_names.insert(vlan_id, String::from_utf8_lossy(name).to_string());
            current_oid = oid_vec;
        } else {
            break;
        }
    }

    // Get VLAN port memberships
    println!("VLAN Membership Information:");
    println!("---------------------------");
    
    let mut current_oid = VLAN_STATIC_EGRESS_PORTS.to_vec();
    loop {
        let mut response = sess.getnext(&current_oid)
            .map_err(|e| anyhow!("Failed to get next VLAN egress ports: {:?}", e))?;
        if let Some((oid, Value::OctetString(ports))) = response.varbinds.next() {
            let oid_str = format!("{}", oid);
            let oid_vec = parse_oid(&oid_str);
            // Check if we're still in the egress ports table
            if !starts_with(&oid_vec, VLAN_STATIC_EGRESS_PORTS) {
                break;
            }
            let vlan_id = extract_vlan_id(&oid_vec);
            let vlan_name_str = vlan_id.to_string();
            let vlan_name = vlan_names.get(&vlan_id)
                .map(String::as_str)
                .unwrap_or(&vlan_name_str);
            
            println!("\nVLAN {} ({}):", vlan_id, vlan_name);
            println!("Member ports: {}", decode_port_list(ports));
            current_oid = oid_vec;
        } else {
            break;
        }
    }

    // Get untagged ports
    let mut current_oid = VLAN_STATIC_UNTAGGED_PORTS.to_vec();
    loop {
        let mut response = sess.getnext(&current_oid)
            .map_err(|e| anyhow!("Failed to get next VLAN untagged ports: {:?}", e))?;
        if let Some((oid, Value::OctetString(ports))) = response.varbinds.next() {
            let oid_str = format!("{}", oid);
            let oid_vec = parse_oid(&oid_str);
            // Check if we're still in the untagged ports table
            if !starts_with(&oid_vec, VLAN_STATIC_UNTAGGED_PORTS) {
                break;
            }
            let vlan_id = extract_vlan_id(&oid_vec);
            println!("Untagged ports for VLAN {}: {}", vlan_id, decode_port_list(ports));
            current_oid = oid_vec;
        } else {
            break;
        }
    }

    // Get port VLAN assignments (PVID)
    println!("\nPort VLAN Assignments (PVID):");
    println!("-----------------------------");
    let mut current_oid = PORT_VLAN_TABLE.to_vec();
    loop {
        let mut response = sess.getnext(&current_oid)
            .map_err(|e| anyhow!("Failed to get next port VLAN assignment: {:?}", e))?;
        if let Some((oid, Value::Integer(pvid))) = response.varbinds.next() {
            let oid_str = format!("{}", oid);
            let oid_vec = parse_oid(&oid_str);
            // Check if we're still in the PVID table
            if !starts_with(&oid_vec, PORT_VLAN_TABLE) {
                break;
            }
            let port = extract_port_number(&oid_vec);
            println!("Port {}: PVID {}", port, pvid);
            current_oid = oid_vec;
        } else {
            break;
        }
    }

    Ok(())
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

fn extract_vlan_id(oid: &[u32]) -> u16 {
    oid.last()
        .map(|&n| n as u16)
        .unwrap_or(0)
}

fn extract_port_number(oid: &[u32]) -> u16 {
    oid.last()
        .map(|&n| n as u16)
        .unwrap_or(0)
}

fn decode_port_list(ports: &[u8]) -> String {
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
