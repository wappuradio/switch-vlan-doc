use std::collections::HashMap;
use crate::PortRange;

pub fn generate_port_table(
    port_ranges: &[PortRange],
    port_aliases: &HashMap<u32, Option<String>>,
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
        if range.pvid != 0 {
            vlan_info.push(format!("PVID:{}", range.pvid));
        }
        if !range.vlan_memberships.is_empty() {
            vlan_info.push(format!("Tagged:{:?}", range.vlan_memberships));
        }
        if !range.untagged_vlans.is_empty() {
            vlan_info.push(format!("Untagged:{:?}", range.untagged_vlans));
        }
        let vlans = vlan_info.join(" ");

        // Add row to table
        table.push_str(&format!("| {} | {} | {} |\n",
            port,
            alias,
            vlans
        ));
    }

    table
} 