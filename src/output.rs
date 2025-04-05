use std::collections::HashMap;
use crate::PortRange;

pub fn generate_port_table(
    port_ranges: &[PortRange],
    port_aliases: &HashMap<u32, Option<String>>,
    vlan_names: &HashMap<u32, String>,
) -> String {
    let mut table = String::new();
    
    // Header
    table.push_str("| Port | Alias | VLAN(s) |\n");
    table.push_str("|------|-------|----------|\n");

    for range in port_ranges {
        if range.first_port > 52 {
            continue;
        }

        // Port number/range
        let port = if range.first_port == range.last_port {
            format!("{}", range.first_port)
        } else {
            format!("{}-{}", range.first_port, range.last_port)
        };

        // Alias (if available)
        let alias = if range.first_port == range.last_port {
            port_aliases.get(&range.first_port)
                .and_then(|s| s.clone())
                .unwrap_or_default()
        } else {
            let first_alias = port_aliases.get(&range.first_port);
            let last_alias = port_aliases.get(&range.last_port);
            if first_alias == last_alias {
                first_alias.and_then(|s| s.clone()).unwrap_or_default()
            } else {
                format!("{}-{}", 
                    first_alias.and_then(|s| s.as_deref()).unwrap_or(""),
                    last_alias.and_then(|s| s.as_deref()).unwrap_or(""))
            }
        };

        // VLAN information
        let mut vlan_info = Vec::new();
        if !range.vlan_memberships.is_empty() {
            let mut tagged_vlans: Vec<u32> = range.vlan_memberships.iter().copied().collect();
            tagged_vlans.sort_unstable();
            let tagged_vlans: Vec<String> = tagged_vlans.iter()
                .map(|&vlan_id| {
                    if vlan_id == 1 {
                        vlan_id.to_string()
                    } else if let Some(name) = vlan_names.get(&vlan_id) {
                        format!("{} ({})", name, vlan_id)
                    } else {
                        vlan_id.to_string()
                    }
                })
                .collect();
            vlan_info.push(format!("Tagged:[{}]", tagged_vlans.join(", ")));
        }
        if !range.untagged_vlans.is_empty() {
            let mut untagged_vlans: Vec<u32> = range.untagged_vlans.iter().copied().collect();
            untagged_vlans.sort_unstable();
            let untagged_vlans: Vec<String> = untagged_vlans.iter()
                .map(|&vlan_id| {
                    if vlan_id == 1 {
                        vlan_id.to_string()
                    } else if let Some(name) = vlan_names.get(&vlan_id) {
                        format!("{} ({})", name, vlan_id)
                    } else {
                        vlan_id.to_string()
                    }
                })
                .collect();
            vlan_info.push(format!("Untagged:[{}]", untagged_vlans.join(", ")));
        }
        let vlans = if range.untagged_vlans.len() == 1 
            && range.vlan_memberships.len() <= 1  // Allow the same VLAN to be tagged and untagged
            && range.pvid == *range.untagged_vlans.iter().next().unwrap() {
            // If only one untagged VLAN exists and PVID matches it
            let vlan_id = range.untagged_vlans.iter().next().unwrap();
            if *vlan_id == 1 {
                vlan_id.to_string()
            } else if let Some(name) = vlan_names.get(vlan_id) {
                format!("{} ({})", name, vlan_id)
            } else {
                vlan_id.to_string()
            }
        } else {
            vlan_info.join(" ")
        };

        // Add row to table
        table.push_str(&format!("| {} | {} | {} |\n",
            port,
            alias,
            vlans
        ));
    }

    table
} 