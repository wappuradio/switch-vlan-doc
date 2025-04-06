use std::collections::HashMap;
use crate::PortRange;
use chrono::Local;

pub enum OutputFormat {
    Markdown,
    Html,
}

pub fn generate_port_table(
    port_ranges: &[PortRange],
    vlan_names: &HashMap<u32, String>,
    format: OutputFormat,
    ip_address: &str,
) -> String {
    match format {
        OutputFormat::Markdown => generate_markdown_table(port_ranges, vlan_names),
        OutputFormat::Html => crate::html_output::generate_port_table(port_ranges, vlan_names, ip_address),
    }
}

fn generate_markdown_table(
    port_ranges: &[PortRange],
    vlan_names: &HashMap<u32, String>,
) -> String {
    let mut table = String::new();
    
    // Add timestamp
    let now = Local::now();
    table.push_str(&format!("Generated on: {}\n\n", now.format("%Y-%m-%d %H:%M:%S")));
    
    // Header
    table.push_str("| Port | Alias | VLAN(s) | LACP |\n");
    table.push_str("|------|-------|----------|------|\n");

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
        let alias = range.alias.as_deref().unwrap_or_default();

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

        // LACP information
        let lacp = if let Some(lacp_info) = &range.lacp_info {
            let agg_name = lacp_info.agg_name.as_deref().unwrap_or("Unknown");
            agg_name.to_string()
        } else {
            String::new()
        };

        // Add row to table
        table.push_str(&format!("| {} | {} | {} | {} |\n",
            port,
            alias,
            vlans,
            lacp
        ));
    }

    table
} 