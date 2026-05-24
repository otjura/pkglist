use std::process::Command;
use serde::Deserialize;
use crate::graph::{PackageInput, PackageType};

#[derive(Deserialize, Debug)]
struct FlatpakJson {
    application_id: String,
    name: Option<String>,
    version: Option<String>,
    branch: Option<String>,
    arch: Option<String>,
    installed_size: Option<String>,
    description: Option<String>,
    runtime: Option<String>,
}

fn parse_flatpak_size(size_str: &str) -> u64 {
    let cleaned: String = size_str
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{a0}')
        .collect();
    
    if let Some(pos) = cleaned.find(|c: char| c.is_alphabetic()) {
        let (num_part, unit_part) = cleaned.split_at(pos);
        if let Ok(val) = num_part.parse::<f64>() {
            let multiplier = match unit_part.to_lowercase().as_str() {
                "gb" | "g" => 1024.0 * 1024.0 * 1024.0,
                "mb" | "m" => 1024.0 * 1024.0,
                "kb" | "k" => 1024.0,
                _ => 1.0,
            };
            return (val * multiplier) as u64;
        }
    }
    0
}

pub fn load_installed_flatpaks() -> Result<Vec<PackageInput>, String> {
    // Check if flatpak is in PATH
    if Command::new("flatpak").arg("--version").output().is_err() {
        // Flatpak is not installed/available, return empty list gracefully
        return Ok(Vec::new());
    }

    let output = Command::new("flatpak")
        .args(&[
            "list",
            "--json",
            "--columns=application,name,version,branch,arch,size,description,runtime",
        ])
        .output()
        .map_err(|e| format!("Failed to execute flatpak list: {}", e))?;

    // If flatpak fails, return empty list or error
    if !output.status.success() {
        // Return empty list if flatpak list fails (e.g. no flatpak setup/perms)
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let flatpaks: Vec<FlatpakJson> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse flatpak JSON: {}", e))?;

    let inputs = flatpaks
        .into_iter()
        .map(|f| {
            let size = f.installed_size.as_deref().map(parse_flatpak_size).unwrap_or(0);
            let mut flatpak_deps = Vec::new();
            if let Some(ref rt) = f.runtime {
                if !rt.is_empty() {
                    flatpak_deps.push(rt.clone());
                    if rt.contains(".Platform") {
                        let sdk = rt.replace(".Platform", ".Sdk");
                        flatpak_deps.push(sdk);
                    }
                }
            }
            PackageInput {
                name: f.application_id,
                version: f.version.unwrap_or_else(|| "unknown".to_string()),
                release: f.branch.unwrap_or_else(|| "stable".to_string()),
                arch: f.arch.unwrap_or_else(|| "unknown".to_string()),
                installsize: size,
                summary: f.name.unwrap_or_else(|| "Flatpak application".to_string()),
                description: f.description.unwrap_or_default(),
                resolved_deps: Vec::new(),
                pkg_type: PackageType::Flatpak,
                flatpak_deps,
            }
        })
        .collect();

    Ok(inputs)
}
