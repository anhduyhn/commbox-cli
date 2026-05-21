use anyhow::Result;
use inquire::{Select, Text};

use crate::panels::PanelEntry;

pub fn select_panel_targets(panels: &[PanelEntry]) -> Result<Vec<(String, String)>> {
    let source_options: Vec<&str> = if panels.is_empty() {
        vec!["Type IP"]
    } else {
        vec!["Type IP", "Pick from list", "All panels in group"]
    };

    let source = Select::new("How to specify panel(s)?", source_options).prompt()?;

    match source {
        "Type IP" => {
            let ip = Text::new("Panel IP:").prompt()?.trim().to_string();
            Ok(vec![(ip.clone(), ip)])
        }
        "Pick from list" => {
            let labels: Vec<String> = panels.iter().map(|p| match &p.name {
                Some(n) => format!("{} - {} [{}]", n, p.ip, p.group),
                None    => format!("{} [{}]", p.ip, p.group),
            }).collect();
            let chosen = Select::new("Which panel?", labels.clone()).prompt()?;
            let idx = labels.iter().position(|l| l == &chosen).unwrap();
            let p = &panels[idx];
            let label = p.name.clone().unwrap_or_else(|| p.ip.clone());
            Ok(vec![(label, p.ip.clone())])
        }
        "All panels in group" => {
            let mut groups: Vec<String> = panels.iter().map(|p| p.group.clone()).collect();
            groups.sort();
            groups.dedup();
            let group = Select::new("Which group?", groups).prompt()?;
            let in_group: Vec<(String, String)> = panels.iter()
                .filter(|p| p.group == group)
                .map(|p| {
                    let label = p.name.clone().unwrap_or_else(|| p.ip.clone());
                    (label, p.ip.clone())
                })
                .collect();
            if in_group.is_empty() {
                anyhow::bail!("No panels in group '{}'", group);
            }
            Ok(in_group)
        }
        _ => unreachable!(),
    }
}