use std::collections::{HashMap, HashSet};
use std::process::Command;
use crate::graph::PackageInput;

#[derive(Debug, Clone)]
pub struct RawPackage {
    pub name: String,
    pub version: String,
    pub release: String,
    pub arch: String,
    pub installsize: u64,
    pub summary: String,
    pub description: String,
    pub requires: Vec<String>,
    pub provides: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum ParseState {
    Idle,
    Pkg,
    Ver,
    Rel,
    Arch,
    Size,
    Sum,
    Desc,
    Req,
    Prov,
}

pub fn load_installed_packages() -> Result<Vec<PackageInput>, String> {
    let output = Command::new("dnf5")
        .args(&[
            "repoquery",
            "--installed",
            "--queryformat",
            "=PKG=%{name}\n=VER=%{version}\n=REL=%{release}\n=ARCH=%{arch}\n=SIZE=%{installsize}\n=SUM=%{summary}\n=DESC=\n%{description}\n=REQ=\n%{requires}\n=PROV=\n%{provides}\n=END=\n",
        ])
        .output()
        .map_err(|e| format!("Failed to execute dnf5: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("dnf5 command failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let raw_pkgs = parse_dnf_output(&stdout)?;
    Ok(resolve_packages(raw_pkgs))
}

pub fn parse_dnf_output(stdout: &str) -> Result<Vec<RawPackage>, String> {
    let mut packages = Vec::new();
    let mut current = None;
    let mut state = ParseState::Idle;

    for line in stdout.lines() {
        if line.starts_with("=PKG=") {
            if let Some(pkg) = current.take() {
                packages.push(pkg);
            }
            current = Some(RawPackage {
                name: line["=PKG=".len()..].to_string(),
                version: String::new(),
                release: String::new(),
                arch: String::new(),
                installsize: 0,
                summary: String::new(),
                description: String::new(),
                requires: Vec::new(),
                provides: Vec::new(),
            });
            state = ParseState::Pkg;
            continue;
        }

        let pkg = match current.as_mut() {
            Some(p) => p,
            None => continue,
        };

        if line.starts_with("=VER=") {
            pkg.version = line["=VER=".len()..].to_string();
            state = ParseState::Ver;
            continue;
        }
        if line.starts_with("=REL=") {
            pkg.release = line["=REL=".len()..].to_string();
            state = ParseState::Rel;
            continue;
        }
        if line.starts_with("=ARCH=") {
            pkg.arch = line["=ARCH=".len()..].to_string();
            state = ParseState::Arch;
            continue;
        }
        if line.starts_with("=SIZE=") {
            pkg.installsize = line["=SIZE=".len()..].parse::<u64>().unwrap_or(0);
            state = ParseState::Size;
            continue;
        }
        if line.starts_with("=SUM=") {
            pkg.summary = line["=SUM=".len()..].to_string();
            state = ParseState::Sum;
            continue;
        }
        if line == "=DESC=" {
            state = ParseState::Desc;
            continue;
        }
        if line == "=REQ=" {
            state = ParseState::Req;
            continue;
        }
        if line == "=PROV=" {
            state = ParseState::Prov;
            continue;
        }
        if line == "=END=" {
            state = ParseState::Idle;
            continue;
        }

        match state {
            ParseState::Desc => {
                if !pkg.description.is_empty() {
                    pkg.description.push('\n');
                }
                pkg.description.push_str(line);
            }
            ParseState::Req => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    pkg.requires.push(trimmed.to_string());
                }
            }
            ParseState::Prov => {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    pkg.provides.push(trimmed.to_string());
                }
            }
            _ => {}
        }
    }

    if let Some(pkg) = current {
        packages.push(pkg);
    }

    Ok(packages)
}

/// Strip version constraints from an RPM capability string.
///
/// e.g. "libfoo.so.1()(64bit) >= 1.2" → "libfoo.so.1()(64bit)"
fn clean_capability(cap: &str) -> String {
    let mut split_idx = cap.len();
    for op in &[" >= ", " <= ", " = ", " > ", " < ", ">=", "<=", "="] {
        if let Some(idx) = cap.find(op) {
            if idx < split_idx {
                split_idx = idx;
            }
        }
    }
    cap[..split_idx].trim().to_string()
}

/// Resolve RPM capability-based dependencies into direct package index references.
///
/// Builds a provides map from all packages, then resolves each package's requires
/// against it, filtering out RPM-internal virtual capabilities.
pub fn resolve_packages(raw_pkgs: Vec<RawPackage>) -> Vec<PackageInput> {
    // Map capabilities (provides) to all package indices that provide them
    let mut provides_map: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, rp) in raw_pkgs.iter().enumerate() {
        // A package always provides its own name
        provides_map.entry(rp.name.clone()).or_default().push(i);
        
        for prov in &rp.provides {
            let clean = clean_capability(prov);
            provides_map.entry(clean).or_default().push(i);
        }
    }
    
    // Resolve each package's requires into dependency indices
    let resolved_deps: Vec<Vec<usize>> = raw_pkgs.iter().enumerate().map(|(i, rp)| {
        let mut deps = HashSet::new();
        for req in &rp.requires {
            let clean = clean_capability(req);
            
            // Skip RPM-internal virtual requirements
            if clean.starts_with("rpmlib(") || clean.starts_with("rtld(") {
                continue;
            }
            
            if let Some(providers) = provides_map.get(&clean) {
                for &dep_idx in providers {
                    if dep_idx != i { // No self-loops
                        deps.insert(dep_idx);
                    }
                }
            }
        }
        deps.into_iter().collect()
    }).collect();
    
    // Convert to backend-agnostic PackageInput
    raw_pkgs.into_iter().zip(resolved_deps).map(|(rp, deps)| {
        PackageInput {
            name: rp.name,
            version: rp.version,
            release: rp.release,
            arch: rp.arch,
            installsize: rp.installsize,
            summary: rp.summary,
            description: rp.description,
            resolved_deps: deps,
            pkg_type: crate::graph::PackageType::Rpm,
            flatpak_deps: Vec::new(),
        }
    }).collect()
}
