mod snmp_utils;
mod output;
mod html_output;
use snmp_utils::{get_u32_table, get_string_table, create_session, decode_port_list, get_raw_table};
use std::collections::{HashSet, HashMap};
use std::time::Duration;
use anyhow::Result;
use output::{generate_port_table, OutputFormat};
use clap::Parser;

// Q-BRIDGE-MIB OIDs
const VLAN_STATIC_NAME: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,1];  // dot1qVlanStaticName
const VLAN_STATIC_EGRESS_PORTS: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,2];  // dot1qVlanStaticEgressPorts
const VLAN_STATIC_UNTAGGED_PORTS: &[u32] = &[1,3,6,1,2,1,17,7,1,4,3,1,4];  // dot1qVlanStaticUntaggedPorts
const PORT_VLAN_TABLE: &[u32] = &[1,3,6,1,2,1,17,7,1,4,5,1,1];  // dot1qPvid

// IF-MIB OIDs
const IF_INDEX: &[u32] = &[1,3,6,1,2,1,2,2,1,1];  // ifIndex
const IF_DESCR: &[u32] = &[1,3,6,1,2,1,2,2,1,2];  // ifDescr
const IF_ALIAS: &[u32] = &[1,3,6,1,2,1,31,1,1,1,18];  // ifAlias
const IF_NAME: &[u32] = &[1,3,6,1,2,1,31,1,1,1,1];  // ifName
const IF_TYPE: &[u32] = &[1,3,6,1,2,1,2,2,1,3];  // ifType

// IEEE8023-LAG-MIB OIDs
const LAG_PORT_SELECTED: &[u32] = &[1,2,840,10006,300,43,1,2,1,1,13];  // dot3adAggPortSelectedAggID
const LAG_AGG_NAME: &[u32] = &[1,3,6,1,2,1,31,1,1,1,1];  // ifName for LACP interfaces

#[derive(Debug, PartialEq, Eq)]
pub struct PortConfig {
    port_num: u32,
    description: String,
    alias: Option<String>,
    pvid: u32,
    vlan_memberships: HashSet<u32>,
    untagged_vlans: HashSet<u32>,
    lacp_info: Option<LacpInfo>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LacpInfo {
    selected_agg_id: u32,
    agg_name: Option<String>,
    agg_vlans: Option<(HashSet<u32>, HashSet<u32>)>, // (tagged, untagged)
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// IP address of the SNMP agent (e.g., 10.1.0.23)
    #[arg(short, long)]
    ip: String,

    /// SNMP community string
    #[arg(short, long, default_value = "public")]
    community: String,

    /// Ignore interface aliases
    #[arg(short = 'n', long)]
    ignore_alias: bool,

    /// SNMP timeout in seconds
    #[arg(short, long, default_value = "2")]
    timeout: u64,

    /// Output format (markdown or html)
    #[arg(short, long, default_value = "markdown")]
    format: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct PortRange {
    first_port: u32,
    last_port: u32,
    description: String,
    pvid: u32,
    vlan_memberships: HashSet<u32>,
    untagged_vlans: HashSet<u32>,
    lacp_info: Option<LacpInfo>,
}

fn is_physical_port(port_type: u32, ip: &str) -> bool {
    // Keep all ports on toimisto-sw
    if ip.contains("toimisto-sw") {
        return true;
    }

    // For other switches, only keep 100M and 1G ports
    // ifType 6 = ethernetCsmacd (100M)
    // ifType 117 = gigabitEthernet (1G)
    port_type == 6 || port_type == 117
}

fn main() -> Result<()> {
    let args = Args::parse();
    let timeout = Duration::from_secs(args.timeout);
    
    // Validate IP address and construct agent address
    let agent_addr = format!("{}:161", args.ip);

    let mut sess = create_session(&agent_addr, args.community.as_bytes(), timeout)?;
    
    eprintln!("Fetching VLAN information...\n");

    // Get all tables first
    let port_indices = get_u32_table(&mut sess, IF_INDEX)?;
    let port_descriptions = get_string_table(&mut sess, IF_DESCR)?;
    let port_names = get_string_table(&mut sess, IF_NAME)?;
    let port_types = get_u32_table(&mut sess, IF_TYPE)?;
    let port_aliases = if !args.ignore_alias { 
        if let Ok(aliases) = get_string_table(&mut sess, IF_ALIAS) {
            aliases.into_iter().map(|(k, v)| (k, Some(v))).collect()
        } else {
            port_names.into_iter().map(|(k, v)| (k, Some(v))).collect()
        }
    } else { 
        HashMap::new() 
    };
    let vlan_names = get_string_table(&mut sess, VLAN_STATIC_NAME)?;
    let vlan_egress_ports = get_raw_table(&mut sess, VLAN_STATIC_EGRESS_PORTS)?;
    let vlan_untagged_ports = get_raw_table(&mut sess, VLAN_STATIC_UNTAGGED_PORTS)?;
    let port_vlans = get_u32_table(&mut sess, PORT_VLAN_TABLE)?;

    // Get LACP information
    let lag_selected_agg_ids = get_u32_table(&mut sess, LAG_PORT_SELECTED)?;
    let lag_agg_names = get_string_table(&mut sess, LAG_AGG_NAME)?;

    // Get VLAN information for LACP interfaces
    let mut lag_vlan_info: HashMap<u32, (HashSet<u32>, HashSet<u32>)> = HashMap::new();
    for agg_id in lag_selected_agg_ids.values() {
        if *agg_id > 0 {
            let mut tagged = HashSet::new();
            let mut untagged = HashSet::new();
            
            // Check VLAN memberships for the LACP interface using the LAG interface number
            for (vlan_id, ports_data) in &vlan_egress_ports {
                let port_list = decode_port_list(ports_data);
                if port_list.split(", ").any(|p| p.parse::<u32>().map_or(false, |p| p == *agg_id)) {
                    tagged.insert(*vlan_id);
                }
            }

            // Check untagged VLANs for the LACP interface using the LAG interface number
            for (vlan_id, ports_data) in &vlan_untagged_ports {
                let port_list = decode_port_list(ports_data);
                if port_list.split(", ").any(|p| p.parse::<u32>().map_or(false, |p| p == *agg_id)) {
                    untagged.insert(*vlan_id);
                }
            }

            if !tagged.is_empty() || !untagged.is_empty() {
                lag_vlan_info.insert(*agg_id, (tagged, untagged));
            }
        }
    }

    // First, collect all individual port configurations
    let mut port_configs: Vec<PortConfig> = Vec::new();

    for port_num in port_indices.into_values() {
        let description = port_descriptions.get(&port_num)
            .cloned()
            .unwrap_or_default();
        
        // Skip non-physical ports based on ifType
        let port_type = port_types.get(&port_num).copied().unwrap_or(0);
        if !is_physical_port(port_type, &args.ip) {
            continue;
        }
        
        let alias = port_aliases.get(&port_num).cloned().flatten();
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

        // Check if port is part of an LACP trunk
        let lacp_info = if let Some(&selected_agg_id) = lag_selected_agg_ids.get(&port_num) {
            if selected_agg_id > 0 {
                let agg_name = lag_agg_names.get(&selected_agg_id).cloned();
                let agg_vlans = lag_vlan_info.get(&selected_agg_id).cloned();
                Some(LacpInfo {
                    selected_agg_id,
                    agg_name,
                    agg_vlans,
                })
            } else {
                None
            }
        } else {
            None
        };

        // If this port is part of an LACP trunk with VLAN info, use that instead
        if let Some(lacp_info) = &lacp_info {
            if let Some((tagged, untagged)) = &lacp_info.agg_vlans {
                vlan_memberships = tagged.clone();
                untagged_vlans = untagged.clone();
            }
        }

        port_configs.push(PortConfig {
            port_num,
            description,
            alias,
            pvid,
            vlan_memberships,
            untagged_vlans,
            lacp_info,
        });
    }

    // Sort by port number to ensure ranges are contiguous
    port_configs.sort_by_key(|config| config.port_num);

    // Group ports with identical configuration into ranges
    let mut port_ranges: Vec<PortRange> = Vec::new();
    let mut current_config: Option<PortConfig> = None;
    let mut current_start: u32 = 0;
    let mut current_end: u32 = 0;

    // Helper function to check if configurations match
    let configs_match = |a: &PortConfig, b: &PortConfig| -> bool {
        a.pvid == b.pvid && 
        a.vlan_memberships == b.vlan_memberships && 
        a.untagged_vlans == b.untagged_vlans &&
        a.alias == b.alias &&
        a.lacp_info == b.lacp_info
    };

    for config in port_configs {
        let port_num = config.port_num;
        match &current_config {
            Some(current) => {
                if configs_match(current, &config) && port_num == current_end + 1 {
                    // Extend current range
                    current_end = port_num;
                } else {
                    // End current range and start new one
                    if let Some(current) = current_config.take() {
                        port_ranges.push(PortRange {
                            first_port: current_start,
                            last_port: current_end,
                            description: current.description,
                            pvid: current.pvid,
                            vlan_memberships: current.vlan_memberships,
                            untagged_vlans: current.untagged_vlans,
                            lacp_info: current.lacp_info,
                        });
                    }
                    current_config = Some(config);
                    current_start = port_num;
                    current_end = port_num;
                }
            }
            None => {
                current_config = Some(config);
                current_start = port_num;
                current_end = port_num;
            }
        }
    }

    // Add the last range if it exists
    if let Some(current) = current_config {
        port_ranges.push(PortRange {
            first_port: current_start,
            last_port: current_end,
            description: current.description,
            pvid: current.pvid,
            vlan_memberships: current.vlan_memberships,
            untagged_vlans: current.untagged_vlans,
            lacp_info: current.lacp_info,
        });
    }

    // Display final port information using the new table format
    let output_format = match args.format.to_lowercase().as_str() {
        "html" => OutputFormat::Html,
        "markdown" => OutputFormat::Markdown,
        _ => {
            eprintln!("Invalid output format. Using markdown.");
            OutputFormat::Markdown
        }
    };

    let output = match output_format {
        OutputFormat::Html => generate_port_table(&port_ranges, &port_aliases, &vlan_names, output_format, &args.ip),
        OutputFormat::Markdown => {
            let mut output = String::new();
            output.push_str("\nPort Information Table:\n");
            output.push_str(&generate_port_table(&port_ranges, &port_aliases, &vlan_names, output_format, ""));
            output
        }
    };

    println!("{}", output);

    Ok(())
}
