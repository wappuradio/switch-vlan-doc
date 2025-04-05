mod snmp_utils;
use snmp_utils::{get_table_values, create_session, extract_last_id, decode_port_list};
use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;

// Q-BRIDGE-MIB OIDs
const VLAN_STATIC_NAME: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,1];  // dot1qVlanStaticName
const VLAN_STATIC_EGRESS_PORTS: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,2];  // dot1qVlanStaticEgressPorts
const VLAN_STATIC_UNTAGGED_PORTS: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,4];  // dot1qVlanStaticUntaggedPorts
const PORT_VLAN_TABLE: &[u32] = &[1,3,6,1,2,1,17,7,1,4,5,1,1];  // dot1qPvid

fn main() -> Result<()> {
    let agent_addr = "10.1.0.24:161";
    let community = b"public";
    let timeout = Duration::from_secs(2);

    let mut sess = create_session(agent_addr, community, timeout)?;
    
    println!("Fetching VLAN information...\n");

    // Get VLAN names
    let mut vlan_names = HashMap::new();
    let values = get_table_values(&mut sess, VLAN_STATIC_NAME)?;
    for (oid, value) in values {
        let vlan_id = extract_last_id(&oid);
        vlan_names.insert(vlan_id, String::from_utf8_lossy(&value).to_string());
    }

    // Get VLAN port memberships
    println!("VLAN Membership Information:");
    println!("---------------------------");
    
    let values = get_table_values(&mut sess, VLAN_STATIC_EGRESS_PORTS)?;
    for (oid, ports) in values {
        let vlan_id = extract_last_id(&oid);
        let vlan_name_str = vlan_id.to_string();
        let vlan_name = vlan_names.get(&vlan_id)
            .map(String::as_str)
            .unwrap_or(&vlan_name_str);
        
        println!("\nVLAN {} ({}):", vlan_id, vlan_name);
        println!("Member ports: {}", decode_port_list(&ports));
    }

    // Get untagged ports
    let values = get_table_values(&mut sess, VLAN_STATIC_UNTAGGED_PORTS)?;
    for (oid, ports) in values {
        let vlan_id = extract_last_id(&oid);
        println!("Untagged ports for VLAN {}: {}", vlan_id, decode_port_list(&ports));
    }

    // Get port VLAN assignments (PVID)
    println!("\nPort VLAN Assignments (PVID):");
    println!("-----------------------------");
    let values = get_table_values(&mut sess, PORT_VLAN_TABLE)?;
    for (oid, value) in values {
        let port = extract_last_id(&oid);
        let pvid = if value.len() >= 4 {
            u32::from_be_bytes(value[..4].try_into().unwrap_or([0; 4]))
        } else {
            0
        };
        println!("Port {}: PVID {}", port, pvid);
    }

    Ok(())
}
