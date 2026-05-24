use std::collections::{HashMap, HashSet, VecDeque};

/// Backend-agnostic package input. Any package manager backend should
/// produce a Vec of these with pre-resolved dependency indices.
#[derive(Debug, Clone)]
pub struct PackageInput {
    pub name: String,
    pub version: String,
    pub release: String,
    pub arch: String,
    pub installsize: u64,
    pub summary: String,
    pub description: String,
    /// Indices into the parent Vec<PackageInput> representing resolved dependencies.
    pub resolved_deps: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub release: String,
    pub arch: String,
    pub installsize: u64,
    pub summary: String,
    pub description: String,
    
    // Indices into the packages array
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

pub fn build_graph(inputs: Vec<PackageInput>) -> DependencyGraph {
    let mut packages = Vec::with_capacity(inputs.len());
    let mut name_to_index = HashMap::with_capacity(inputs.len());
    
    // Build reverse dependency map from pre-resolved forward edges
    let mut rev_deps: Vec<Vec<usize>> = vec![Vec::new(); inputs.len()];
    for (i, input) in inputs.iter().enumerate() {
        for &dep_idx in &input.resolved_deps {
            rev_deps[dep_idx].push(i);
        }
    }
    
    for (i, input) in inputs.into_iter().enumerate() {
        name_to_index.insert(input.name.clone(), i);
        packages.push(Package {
            name: input.name,
            version: input.version,
            release: input.release,
            arch: input.arch,
            installsize: input.installsize,
            summary: input.summary,
            description: input.description,
            dependencies: input.resolved_deps,
            dependents: std::mem::take(&mut rev_deps[i]),
            transitive_size: 0,
            exclusive_size: 0,
        });
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
