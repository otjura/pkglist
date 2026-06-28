use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use crate::graph::{PackageInput, PackageType};

#[derive(serde::Deserialize, Debug)]
struct PackageJson {
    description: Option<String>,
}

fn get_dir_size<P: AsRef<Path>>(path: P) -> u64 {
    let mut total_size = 0;
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        let file_type = metadata.file_type();
        if file_type.is_file() {
            total_size += metadata.len();
        } else if file_type.is_dir() {
            if let Ok(entries) = fs::read_dir(&path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        total_size += get_dir_size(entry.path());
                    }
                }
            }
        }
    }
    total_size
}

fn parse_bun_list_line(line: &str) -> Option<(String, String)> {
    let cleaned = line.trim()
        .trim_start_matches(|c: char| c == '├' || c == '└' || c == '│' || c == '─' || c == '┬' || c == ' ' || c == '┼');
    
    if let Some(pos) = cleaned.rfind('@') {
        if pos > 0 {
            let name = cleaned[..pos].to_string();
            let version = cleaned[pos + 1..].to_string();
            return Some((name, version));
        }
    }
    None
}

pub fn load_installed_bun_packages() -> Result<Vec<PackageInput>, String> {
    // 1. Check if bun is installed
    if Command::new("bun").arg("--version").output().is_err() {
        return Ok(Vec::new());
    }

    // 2. Run bun -g list
    let output = Command::new("bun")
        .args(&["-g", "list"])
        .output()
        .map_err(|e| format!("Failed to execute bun -g list: {}", e))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    // 3. Extract root path from first line
    // E.g. "/home/ojr/.bun/install/global node_modules (27)"
    let first_line = lines[0];
    let mut root_path = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".bun/install/global");
    if let Some(pos) = first_line.find(" node_modules") {
        let path_candidate = first_line[..pos].trim();
        if !path_candidate.is_empty() {
            root_path = PathBuf::from(path_candidate);
        }
    }

    let node_modules_dir = root_path.join("node_modules");

    let mut inputs = Vec::new();

    // 4. Parse packages from remaining lines
    for line in &lines[1..] {
        if let Some((pkg_name, version)) = parse_bun_list_line(line) {
            let pkg_dir = node_modules_dir.join(&pkg_name);
            let size = get_dir_size(&pkg_dir);

            let mut description = String::new();
            let mut summary = String::new();

            let pkg_json_path = pkg_dir.join("package.json");
            if pkg_json_path.exists() {
                if let Ok(content) = fs::read_to_string(&pkg_json_path) {
                    if let Ok(pkg_json) = serde_json::from_str::<PackageJson>(&content) {
                        if let Some(desc) = pkg_json.description {
                            summary = desc.clone();
                            description = desc;
                        }
                    }
                }
            }

            if summary.is_empty() {
                summary = format!("Globally installed Bun package: {}", pkg_name);
            }

            inputs.push(PackageInput {
                name: pkg_name,
                version,
                release: "N/A".to_string(),
                arch: "all".to_string(),
                installsize: size,
                summary,
                description,
                resolved_deps: Vec::new(),
                pkg_type: PackageType::Bun,
                flatpak_deps: Vec::new(),
            });
        }
    }

    Ok(inputs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bun_line() {
        assert_eq!(
            parse_bun_list_line("├── @google/gemini-cli@0.49.0"),
            Some(("@google/gemini-cli".to_string(), "0.49.0".to_string()))
        );
        assert_eq!(
            parse_bun_list_line("└── opencode-ai@1.17.11"),
            Some(("opencode-ai".to_string(), "1.17.11".to_string()))
        );
        assert_eq!(
            parse_bun_list_line("├── arrpc@3.6.0"),
            Some(("arrpc".to_string(), "3.6.0".to_string()))
        );
        assert_eq!(
            parse_bun_list_line("/home/ojr/.bun/install/global node_modules (27)"),
            None
        );
    }
}
