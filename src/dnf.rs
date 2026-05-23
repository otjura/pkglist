use std::process::Command;

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

pub fn load_installed_packages() -> Result<Vec<RawPackage>, String> {
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
    parse_dnf_output(&stdout)
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
