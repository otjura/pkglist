use std::process::Command;
use serde::Deserialize;
use crate::graph::{PackageInput, PackageType};

#[derive(Deserialize, Debug)]
struct PipPackageJson {
    name: String,
    version: String,
    summary: String,
    description: String,
    size: u64,
    dependencies: Vec<String>,
}

pub fn load_installed_pip_packages() -> Result<Vec<PackageInput>, String> {
    if Command::new("python3").arg("--version").output().is_err() {
        return Ok(Vec::new());
    }

    let script = r#"
import importlib.metadata
import json
import os
import pathlib
import re

def get_dist_size(dist):
    size = 0
    if dist.files:
        for file in dist.files:
            try:
                size += os.path.getsize(dist.locate_file(file))
            except OSError:
                pass
        if size > 0:
            return size

    dist_path = getattr(dist, "_path", None)
    if not dist_path:
        return 0
        
    dist_path = pathlib.Path(dist_path)
    site_packages = dist_path.parent
    
    try:
        size += sum(f.stat().st_size for f in dist_path.rglob("*") if f.is_file())
    except Exception:
        pass
    
    top_level_file = dist_path / "top_level.txt"
    if top_level_file.exists():
        try:
            for line in top_level_file.read_text().splitlines():
                name = line.strip()
                if not name:
                    continue
                dir_path = site_packages / name
                if dir_path.is_dir():
                    size += sum(f.stat().st_size for f in dir_path.rglob("*") if f.is_file())
                else:
                    for ext in [".py", ".pyc", ".so"]:
                        file_path = site_packages / (name + ext)
                        if file_path.exists():
                            size += file_path.stat().st_size
        except Exception:
            pass
            
    return size

pkgs = []
for dist in importlib.metadata.distributions():
    meta = dist.metadata
    name = meta.get("Name", dist.name)
    version = meta.get("Version", dist.version)
    summary = meta.get("Summary", "")
    description = meta.get("Description", "")
    
    reqs = []
    if dist.requires:
        for req in dist.requires:
            match = re.match(r"^([a-zA-Z0-9_\-\.]+)", req.strip())
            if match:
                reqs.append(match.group(1).lower().replace("_", "-"))
                
    size = get_dist_size(dist)
    
    pkgs.append({
        "name": name,
        "version": version,
        "summary": summary,
        "description": description,
        "size": size,
        "dependencies": reqs,
    })

print(json.dumps(pkgs))
"#;

    let output = Command::new("python3")
        .args(&["-c", script])
        .output()
        .map_err(|e| format!("Failed to execute python3 metadata script: {}", e))?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pip_pkgs: Vec<PipPackageJson> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse pip package JSON: {}", e))?;

    let inputs = pip_pkgs
        .into_iter()
        .map(|p| PackageInput {
            name: p.name,
            version: p.version,
            release: "N/A".to_string(),
            arch: "all".to_string(),
            installsize: p.size,
            summary: p.summary,
            description: p.description,
            resolved_deps: Vec::new(),
            pkg_type: PackageType::Pip,
            flatpak_deps: p.dependencies,
        })
        .collect();

    Ok(inputs)
}
