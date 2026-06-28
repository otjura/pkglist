use std::process::Command;
use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use serde::Deserialize;
use crate::graph::{PackageInput, PackageType};

#[derive(Deserialize, Debug)]
struct NpmListJson {
    dependencies: Option<HashMap<String, NpmDependencyJson>>,
}

#[derive(Deserialize, Debug)]
struct NpmDependencyJson {
    version: Option<String>,
}

#[derive(Deserialize, Debug)]
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

fn get_npm_global_root() -> Option<PathBuf> {
    let output = Command::new("npm")
        .args(&["root", "-g"])
        .output()
        .ok()?;
    
    if output.status.success() {
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
            return Some(PathBuf::from(path_str));
        }
    }
    None
}

pub fn load_installed_npm_packages() -> Result<Vec<PackageInput>, String> {
    // 1. Check if npm is installed
    if Command::new("npm").arg("--version").output().is_err() {
        return Ok(Vec::new());
    }

    // 2. Get global root
    let root_path = match get_npm_global_root() {
        Some(path) => path,
        None => return Ok(Vec::new()),
    };

    // 3. Run npm list
    let output = Command::new("npm")
        .args(&["list", "-g", "--depth=0", "--json"])
        .output()
        .map_err(|e| format!("Failed to execute npm list: {}", e))?;

    if !output.status.success() {
        // Return empty if it fails (e.g. no global packages at all or command fails)
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let list_json: NpmListJson = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse npm list JSON: {}", e))?;

    let mut inputs = Vec::new();

    if let Some(deps) = list_json.dependencies {
        for (pkg_name, dep_info) in deps {
            let version = dep_info.version.unwrap_or_else(|| "unknown".to_string());
            
            // Path to package directory
            let pkg_dir = root_path.join(&pkg_name);
            let size = get_dir_size(&pkg_dir);

            // Try to load description from package.json
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
                summary = format!("Globally installed npm package: {}", pkg_name);
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
                pkg_type: PackageType::Npm,
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
    fn test_dir_size() {
        let temp_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("pkglist_test_npm_dir_size");
        if temp_dir.exists() {
            fs::remove_dir_all(&temp_dir).ok();
        }
        fs::create_dir_all(&temp_dir).unwrap();

        let file1 = temp_dir.join("file1.txt");
        fs::write(&file1, b"hello").unwrap(); // 5 bytes

        let sub_dir = temp_dir.join("sub");
        fs::create_dir_all(&sub_dir).unwrap();

        let file2 = sub_dir.join("file2.txt");
        fs::write(&file2, b"world!").unwrap(); // 6 bytes

        // symlink (should not be followed or counted)
        let symlink_path = temp_dir.join("symlink.txt");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&file1, &symlink_path).ok();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&file1, &symlink_path).ok();

        let size = get_dir_size(&temp_dir);
        assert_eq!(size, 11);

        fs::remove_dir_all(&temp_dir).ok();
    }
}
