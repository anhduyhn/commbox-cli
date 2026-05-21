use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PanelEntry {
    pub group: String,
    pub ip: String,
    pub name: Option<String>,
}

pub fn parse_panels_file(path: &PathBuf) -> Result<Vec<PanelEntry>> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("reading {}", path.display()))?;
    let mut entries = Vec::new();
    let mut current_group = String::from("default");

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

        if let Some(rest) = trimmed.strip_prefix('[') {
            if let Some(group) = rest.strip_suffix(']') {
                current_group = group.trim().to_string();
                continue;
            }
        }

        let (ip_part, name_part) = match trimmed.split_once('#') {
            Some((ip, name)) => (ip.trim().to_string(), Some(name.trim().to_string())),
            None             => (trimmed.to_string(), None),
        };

        if !ip_part.is_empty() {
            entries.push(PanelEntry {
                group: current_group.clone(),
                ip: ip_part,
                name: name_part,
            });
        }
    }

    Ok(entries)
}