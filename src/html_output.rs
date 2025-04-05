use std::collections::HashMap;
use crate::PortRange;

pub fn generate_port_table(
    port_ranges: &[PortRange],
    port_aliases: &HashMap<u32, Option<String>>,
    vlan_names: &HashMap<u32, String>,
) -> String {
    let mut table = String::new();
    
    // Start HTML with CSS styling
    table.push_str(r#"<style>
    .port-table {
        border-collapse: collapse;
        width: 100%;
        margin: 20px 0;
        font-family: Arial, sans-serif;
    }
    .port-table th, .port-table td {
        border: 1px solid #ddd;
        padding: 8px;
        text-align: left;
    }
    .port-table th {
        background-color: #f2f2f2;
        font-weight: bold;
    }
    .port-table tr:nth-child(even) {
        background-color: #f9f9f9;
    }
    .port-table tr:hover {
        background-color: #f5f5f5;
    }
</style>
<table class="port-table">
    <thead>
        <tr>
            <th>Port</th>
            <th>Alias</th>
            <th>VLAN(s)</th>
        </tr>
    </thead>
    <tbody>"#);

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
        table.push_str(&format!(r#"        <tr>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
        </tr>"#,
            port,
            alias,
            vlans
        ));
    }

    // Close HTML table
    table.push_str(r#"    </tbody>
</table>"#);

    table
} 