mod snmp_utils;
use snmp_utils::{get_u32_table, get_string_table, get_optional_string_table, create_session, decode_port_list, get_raw_table};
use std::collections::HashSet;
use std::time::Duration;
use anyhow::Result;

// Q-BRIDGE-MIB OIDs
const VLAN_STATIC_NAME: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,1];  // dot1qVlanStaticName
const VLAN_STATIC_EGRESS_PORTS: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,2];  // dot1qVlanStaticEgressPorts
const VLAN_STATIC_UNTAGGED_PORTS: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,4];  // dot1qVlanStaticUntaggedPorts
const PORT_VLAN_TABLE: &[u32] = &[1,3,6,1,2,1,17,7,1,4,5,1,1];  // dot1qPvid

// IF-MIB OIDs
const IF_INDEX: &[u32] = &[1,3,6,1,2,1,2,2,1,1];  // ifIndex
const IF_DESCR: &[u32] = &[1,3,6,1,2,1,2,2,1,2];  // ifDescr
const IF_ALIAS: &[u32] = &[1,3,6,1,2,1,31,1,1,1,18];  // ifAlias

#[derive(Debug)]
struct SwitchPort {
    port_number: u32,
    description: String,
    alias: Option<String>,
    pvid: u32,
    vlan_memberships: HashSet<u32>,
    untagged_vlans: HashSet<u32>,
}

fn main() -> Result<()> {
    let agent_addr = "10.1.0.24:161";
    let community = b"public";
    let timeout = Duration::from_secs(2);

    let mut sess = create_session(agent_addr, community, timeout)?;
    
    println!("Fetching VLAN information...\n");

    // Get all tables first
    let port_indices = get_u32_table(&mut sess, IF_INDEX)?;
    let port_descriptions = get_string_table(&mut sess, IF_DESCR)?;
    let port_aliases = get_optional_string_table(&mut sess, IF_ALIAS)?;
    let _vlan_names = get_string_table(&mut sess, VLAN_STATIC_NAME)?;
    let vlan_egress_ports = get_raw_table(&mut sess, VLAN_STATIC_EGRESS_PORTS)?;
    let vlan_untagged_ports = get_raw_table(&mut sess, VLAN_STATIC_UNTAGGED_PORTS)?;
    let port_vlans = get_u32_table(&mut sess, PORT_VLAN_TABLE)?;

    // Construct all port objects in a single pass
    let mut ports: Vec<SwitchPort> = Vec::new();

    for port_num in port_indices.into_values() {
        let description = port_descriptions.get(&port_num)
            .map(|s| s.clone())
            .unwrap_or_default();
        
        let alias = port_aliases.get(&port_num)
            .and_then(|opt| opt.clone());

        let pvid = port_vlans.get(&port_num)
            .copied()
            .unwrap_or(0);

        let mut vlan_memberships = HashSet::new();
        let mut untagged_vlans = HashSet::new();

        // Add VLAN memberships
        for (vlan_id, ports_data) in &vlan_egress_ports {
            let port_list = decode_port_list(ports_data);
            if port_list.split(", ").any(|p| p.parse::<u32>().map_or(false, |p| p == port_num)) {
                vlan_memberships.insert(*vlan_id);
            }
        }

        // Add untagged VLANs
        for (vlan_id, ports_data) in &vlan_untagged_ports {
            let port_list = decode_port_list(ports_data);
            if port_list.split(", ").any(|p| p.parse::<u32>().map_or(false, |p| p == port_num)) {
                untagged_vlans.insert(*vlan_id);
            }
        }

        ports.push(SwitchPort {
            port_number: port_num,
            description,
            alias,
            pvid,
            vlan_memberships,
            untagged_vlans,
        });
    }

    // Display final port information
    println!("\nComplete Port Information:");
    println!("------------------------");
    ports.sort_by_key(|p| p.port_number);
    for port_info in ports {
        if port_info.port_number > 52 {
            continue;
        }
        let alias_str = port_info.alias.as_deref().map(|a| format!(" [{}]", a)).unwrap_or_default();
        println!("Port {} ({}): PVID {}{}", 
            port_info.port_number, 
            port_info.description, 
            port_info.pvid,
            alias_str
        );
        println!("  VLAN Memberships: {:?}", port_info.vlan_memberships);
        println!("  Untagged VLANs: {:?}", port_info.untagged_vlans);
    }

    Ok(())
}
