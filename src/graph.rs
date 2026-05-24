use std::collections::{HashMap, HashSet, VecDeque};
use crate::dnf::RawPackage;

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub release: String,
    pub arch: String,
    pub installsize: u64,
    pub summary: String,
    pub description: String,
    
    // Indices into the dependency array
    pub dependencies: Vec<usize>,
    pub dependents: Vec<usize>,
    
    // Computed size metrics
    pub transitive_size: u64,
    pub exclusive_size: u64,
}

pub struct DependencyGraph {
    pub packages: Vec<Package>,
    pub name_to_index: HashMap<String, usize>,
}

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

pub fn build_graph(raw_pkgs: Vec<RawPackage>) -> DependencyGraph {
    let mut packages = Vec::with_capacity(raw_pkgs.len());
    let mut name_to_index = HashMap::with_capacity(raw_pkgs.len());
    
    for (i, rp) in raw_pkgs.iter().enumerate() {
        name_to_index.insert(rp.name.clone(), i);
        packages.push(Package {
            name: rp.name.clone(),
            version: rp.version.clone(),
            release: rp.release.clone(),
            arch: rp.arch.clone(),
            installsize: rp.installsize,
            summary: rp.summary.clone(),
            description: rp.description.clone(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            transitive_size: 0,
            exclusive_size: 0,
        });
    }
    
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
    
    // Build dependency edges
    let mut temp_deps = vec![HashSet::new(); packages.len()];
    let mut temp_rev_deps = vec![HashSet::new(); packages.len()];
    
    for (i, rp) in raw_pkgs.iter().enumerate() {
        for req in &rp.requires {
            let clean = clean_capability(req);
            
            // Skip system/manager requirements
            if clean.starts_with("rpmlib(") || clean.starts_with("rtld(") {
                continue;
            }
            
            if let Some(providers) = provides_map.get(&clean) {
                for &dep_idx in providers {
                    if dep_idx != i { // No self-loops
                        temp_deps[i].insert(dep_idx);
                        temp_rev_deps[dep_idx].insert(i);
                    }
                }
            }
        }
    }
    
    // Populate packages dependencies and dependents lists
    for i in 0..packages.len() {
        packages[i].dependencies = temp_deps[i].iter().copied().collect();
        packages[i].dependents = temp_rev_deps[i].iter().copied().collect();
    }
    
    // Compute transitive sizes
    for i in 0..packages.len() {
        packages[i].transitive_size = compute_transitive_size(i, &packages);
    }
    
    // Precompute initial reference counts for exclusive size calculation
    let initial_ref_counts: Vec<usize> = packages.iter().map(|p| p.dependents.len()).collect();
    
    // Compute exclusive sizes
    for i in 0..packages.len() {
        packages[i].exclusive_size = compute_exclusive_size(i, &packages, &initial_ref_counts);
    }
    
    DependencyGraph {
        packages,
        name_to_index,
    }
}

fn compute_transitive_size(pkg_idx: usize, packages: &[Package]) -> u64 {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    
    queue.push_back(pkg_idx);
    visited.insert(pkg_idx);
    
    let mut total_size = 0;
    
    while let Some(curr) = queue.pop_front() {
        total_size += packages[curr].installsize;
        for &dep in &packages[curr].dependencies {
            if visited.insert(dep) {
                queue.push_back(dep);
            }
        }
    }
    
    total_size
}

fn compute_exclusive_size(pkg_idx: usize, packages: &[Package], initial_ref_counts: &[usize]) -> u64 {
    let mut ref_counts = initial_ref_counts.to_vec();
    let mut removed = HashSet::new();
    let mut queue = VecDeque::new();
    
    queue.push_back(pkg_idx);
    removed.insert(pkg_idx);
    
    let mut saved_space = 0;
    
    while let Some(curr) = queue.pop_front() {
        saved_space += packages[curr].installsize;
        
        for &dep in &packages[curr].dependencies {
            if ref_counts[dep] > 0 {
                ref_counts[dep] -= 1;
                if ref_counts[dep] == 0 {
                    if removed.insert(dep) {
                        queue.push_back(dep);
                    }
                }
            }
        }
    }
    
    saved_space
}
