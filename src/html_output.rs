use std::collections::HashMap;
use crate::PortRange;
use chrono::Local;

pub fn generate_port_table(
    port_ranges: &[PortRange],
    vlan_names: &HashMap<u32, String>,
    ip_address: &str,
) -> String {
    let mut table = String::new();
    
    // Start HTML with CSS styling
    table.push_str(r#"<style>
    body {
        max-width: 1200px;
        margin: 0 auto;
        padding: 20px;
        font-family: Arial, sans-serif;
    }
    .device-header {
        margin-bottom: 30px;
        padding-bottom: 10px;
        border-bottom: 2px solid #eee;
    }
    .device-header h1 {
        margin: 0;
        color: #333;
        font-size: 24px;
    }
    .device-header h2 {
        margin: 5px 0 0;
        color: #666;
        font-size: 18px;
    }
    .generated-time {
        color: #666;
        font-size: 14px;
        margin-bottom: 20px;
    }
    .port-table {
        border-collapse: collapse;
        width: 100%;
        margin: 20px 0;
        background-color: white;
        box-shadow: 0 1px 3px rgba(0,0,0,0.1);
    }
    .port-table th, .port-table td {
        border: 1px solid #ddd;
        padding: 12px;
        text-align: left;
    }
    .port-table th {
        background-color: #f2f2f2;
        font-weight: bold;
        color: #333;
    }
    .port-table tr:nth-child(even) {
        background-color: #f9f9f9;
    }
    .port-table tr:hover {
        background-color: #f5f5f5;
    }
    .port-table tr.multi-port td {
        padding-top: 24px;
        padding-bottom: 24px;
    }
    .port-table tr.vlan-10 {
        background-color: #e6f3ff;
    }
    .port-table tr.vlan-10:hover {
        background-color: #d9edff;
    }
    .port-table tr.vlan-531 {
        background-color: #e6ffe6;
    }
    .port-table tr.vlan-531:hover {
        background-color: #d9ffd9;
    }
    .port-table tr.vlan-10.even {
        background-color: #d9edff;
    }
    .port-table tr.vlan-10.even:hover {
        background-color: #cce7ff;
    }
    .port-table tr.vlan-531.even {
        background-color: #d9ffd9;
    }
    .port-table tr.vlan-531.even:hover {
        background-color: #ccffcc;
    }
    .port-table tr.multi-tagged {
        background-color: #fff3e6;
    }
    .port-table tr.multi-tagged:hover {
        background-color: #ffe6cc;
    }
    .port-table tr.multi-tagged.even {
        background-color: #ffe6cc;
    }
    .port-table tr.multi-tagged.even:hover {
        background-color: #ffd9b3;
    }
    .port-table tr.lacp {
        background-color: #e6e6ff;
    }
    .port-table tr.lacp:hover {
        background-color: #d9d9ff;
    }
    .port-table tr.lacp.even {
        background-color: #d9d9ff;
    }
    .port-table tr.lacp.even:hover {
        background-color: #ccccff;
    }
</style>
<div class="device-header">
    <h1>Switch Port Configuration</h1>
    <h2>Device: "#);

    table.push_str(ip_address);
    table.push_str(r#"</h2>
    <div class="generated-time">Generated on: "#);

    let now = Local::now();
    table.push_str(&format!("{}</div>", now.format("%Y-%m-%d %H:%M:%S")));
    table.push_str(r#"</div>
<table class="port-table">
    <thead>
        <tr>
            <th>Port</th>
            <th>Alias</th>
            <th>VLAN(s)</th>
            <th>LACP</th>
        </tr>
    </thead>
    <tbody>"#);

    for (index, range) in port_ranges.iter().enumerate() {
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

        // Determine row classes
        let mut row_classes = Vec::new();
        
        // Multi-port class
        if range.first_port != range.last_port {
            row_classes.push("multi-port");
        }
        
        // VLAN-specific classes
        if range.untagged_vlans.len() == 1 {
            let untagged_vlan = *range.untagged_vlans.iter().next().unwrap();
            if untagged_vlan == 10 {
                row_classes.push("vlan-10");
            } else if untagged_vlan == 531 {
                row_classes.push("vlan-531");
            }
        }

        // Multi-tagged class
        if range.vlan_memberships.len() > 1 {
            row_classes.push("multi-tagged");
        }

        // LACP class
        if range.lacp_info.is_some() {
            row_classes.push("lacp");
        }
        
        // Even/odd row styling
        if index % 2 == 1 {
            row_classes.push("even");
        }

        // Add row to table with classes
        let class_str = if !row_classes.is_empty() {
            format!(" class=\"{}\"", row_classes.join(" "))
        } else {
            String::new()
        };

        table.push_str(&format!(r#"        <tr{}>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
            <td>{}</td>
        </tr>"#,
            class_str,
            port,
            alias,
            vlans,
            lacp
        ));
    }

    // Close HTML table
    table.push_str(r#"    </tbody>
</table>"#);

    table
} 