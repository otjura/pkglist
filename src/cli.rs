use clap::{Parser, Subcommand, ValueEnum};
use crate::graph::{DependencyGraph, Package};
use colored::*;
use serde::Serialize;

#[derive(Parser, Debug)]
#[command(name = "pkglist")]
#[command(version)]
#[command(about = format!("pkglist is a package disk space usage explorer. (v{v})", v = env!("CARGO_PKG_VERSION")), long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// List installed packages and their sizes
    List {
        /// Sort order
        #[arg(short, long, value_enum, default_value_t = SortBy::Exclusive)]
        sort: SortBy,

        /// Limit the number of packages to show
        #[arg(short, long)]
        limit: Option<usize>,

        /// Filter packages by name
        #[arg(short, long)]
        search: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value_t = Format::Table)]
        format: Format,
    },
    /// Show detailed info about a single package
    Info {
        /// The name of the package
        package: String,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Type,
    Size,
    Transitive,
    Exclusive,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Table,
    Csv,
    Tsv,
    Json,
}

#[derive(Serialize)]
struct JsonPackage {
    name: String,
    package_type: crate::graph::PackageType,
    version: String,
    release: String,
    arch: String,
    install_size: u64,
    transitive_size: u64,
    exclusive_size: u64,
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn run_list(
    graph: &DependencyGraph,
    sort: SortBy,
    limit: Option<usize>,
    search: Option<String>,
    format: Format,
) {
    let mut pkgs: Vec<&Package> = graph.packages.iter().collect();

    // Filter
    if let Some(query) = search {
        let query_lower = query.to_lowercase();
        pkgs.retain(|p| p.name.to_lowercase().contains(&query_lower));
    }

    // Sort
    pkgs.sort_by(|a, b| {
        match sort {
            SortBy::Name => a.name.cmp(&b.name),
            SortBy::Type => {
                let type_a = match a.pkg_type {
                    crate::graph::PackageType::Rpm => "rpm",
                    crate::graph::PackageType::Flatpak => "flatpak",
                    crate::graph::PackageType::Npm => "npm",
                    crate::graph::PackageType::Bun => "bun",
                    crate::graph::PackageType::Pip => "pip",
                };
                let type_b = match b.pkg_type {
                    crate::graph::PackageType::Rpm => "rpm",
                    crate::graph::PackageType::Flatpak => "flatpak",
                    crate::graph::PackageType::Npm => "npm",
                    crate::graph::PackageType::Bun => "bun",
                    crate::graph::PackageType::Pip => "pip",
                };
                type_a.cmp(type_b).then_with(|| a.name.cmp(&b.name))
            }
            SortBy::Size => b.installsize.cmp(&a.installsize),
            SortBy::Transitive => b.transitive_size.cmp(&a.transitive_size),
            SortBy::Exclusive => b.exclusive_size.cmp(&a.exclusive_size),
        }
    });

    // Limit
    let limit_val = limit.unwrap_or(pkgs.len());
    let pkgs = &pkgs[..limit_val.min(pkgs.len())];

    // Output
    match format {
        Format::Table => print_table(pkgs),
        Format::Csv => print_csv(pkgs),
        Format::Tsv => print_tsv(pkgs),
        Format::Json => print_json(pkgs),
    }
}

fn print_table(pkgs: &[&Package]) {
    if pkgs.is_empty() {
        println!("{}", "No packages found.".yellow());
        return;
    }

    // Determine max column widths for alignment
    let mut max_name_len = 7; // Length of "Package"
    for p in pkgs {
        max_name_len = max_name_len.max(p.name.len());
    }

    // Print headers
    let name_header = format!("{:<width$}", "Package", width = max_name_len);
    println!(
        "{} | {:<8} | {:>12} | {:>12} | {:>12}",
        name_header.bold().underline(),
        "Type".bold().underline(),
        "Size".bold().underline(),
        "Transitive".bold().underline(),
        "Saved Space".bold().underline()
    );

    for p in pkgs {
        let name_str = format!("{:<width$}", p.name, width = max_name_len).cyan().bold();
        let type_raw = match p.pkg_type {
            crate::graph::PackageType::Rpm => "rpm",
            crate::graph::PackageType::Flatpak => "flatpak",
            crate::graph::PackageType::Npm => "npm",
            crate::graph::PackageType::Bun => "bun",
            crate::graph::PackageType::Pip => "pip",
        };
        let type_padded = format!("{:<8}", type_raw);
        let type_str = match p.pkg_type {
            crate::graph::PackageType::Rpm => type_padded.blue(),
            crate::graph::PackageType::Flatpak => type_padded.magenta(),
            crate::graph::PackageType::Npm => type_padded.red(),
            crate::graph::PackageType::Bun => type_padded.cyan(),
            crate::graph::PackageType::Pip => type_padded.yellow(),
        };
        let size_str = format_bytes(p.installsize).green();
        let trans_str = format_bytes(p.transitive_size).yellow();
        let excl_str = format_bytes(p.exclusive_size).magenta();
        println!(
            "{} | {} | {:>12} | {:>12} | {:>12}",
            name_str,
            type_str,
            size_str,
            trans_str,
            excl_str
        );
    }
}

fn print_csv(pkgs: &[&Package]) {
    println!("name,type,version,release,arch,install_size,transitive_size,exclusive_size");
    for p in pkgs {
        let type_str = match p.pkg_type {
            crate::graph::PackageType::Rpm => "rpm",
            crate::graph::PackageType::Flatpak => "flatpak",
            crate::graph::PackageType::Npm => "npm",
            crate::graph::PackageType::Bun => "bun",
            crate::graph::PackageType::Pip => "pip",
        };
        println!(
            "{},{},{},{},{},{},{},{}",
            p.name, type_str, p.version, p.release, p.arch, p.installsize, p.transitive_size, p.exclusive_size
        );
    }
}

fn print_tsv(pkgs: &[&Package]) {
    println!("name\ttype\tversion\trelease\tarch\tinstall_size\ttransitive_size\texclusive_size");
    for p in pkgs {
        let type_str = match p.pkg_type {
            crate::graph::PackageType::Rpm => "rpm",
            crate::graph::PackageType::Flatpak => "flatpak",
            crate::graph::PackageType::Npm => "npm",
            crate::graph::PackageType::Bun => "bun",
            crate::graph::PackageType::Pip => "pip",
        };
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            p.name, type_str, p.version, p.release, p.arch, p.installsize, p.transitive_size, p.exclusive_size
        );
    }
}

fn print_json(pkgs: &[&Package]) {
    let json_pkgs: Vec<JsonPackage> = pkgs
        .iter()
        .map(|p| JsonPackage {
            name: p.name.clone(),
            package_type: p.pkg_type,
            version: p.version.clone(),
            release: p.release.clone(),
            arch: p.arch.clone(),
            install_size: p.installsize,
            transitive_size: p.transitive_size,
            exclusive_size: p.exclusive_size,
        })
        .collect();

    match serde_json::to_string_pretty(&json_pkgs) {
        Ok(s) => println!("{}", s),
        Err(e) => eprintln!("Failed to serialize to JSON: {}", e),
    }
}

pub fn run_info(graph: &DependencyGraph, name: &str) {
    let idx = match graph.name_to_index.get(name) {
        Some(&i) => i,
        None => {
            eprintln!("{}", format!("Error: Package '{}' is not installed or not found.", name).red().bold());
            return;
        }
    };

    let p = &graph.packages[idx];

    println!("{}", "================================================================================".blue());
    println!("{} : {}", "PACKAGE".blue().bold(), p.name.green().bold());
    println!("{}", "================================================================================".blue());
    println!("{:<15} {}-{}", "Version:".bold(), p.version, p.release);
    let type_str = match p.pkg_type {
        crate::graph::PackageType::Rpm => "rpm".blue().bold(),
        crate::graph::PackageType::Flatpak => "flatpak".magenta().bold(),
        crate::graph::PackageType::Npm => "npm".red().bold(),
        crate::graph::PackageType::Bun => "bun".cyan().bold(),
        crate::graph::PackageType::Pip => "pip".yellow().bold(),
    };
    println!("{:<15} {}", "Type:".bold(), type_str);
    println!("{:<15} {}", "Architecture:".bold(), p.arch);
    println!("{:<15} {}", "Summary:".bold(), p.summary);
    println!();
    println!("{:<15} {}", "Size:".bold(), format_bytes(p.installsize).green());
    println!("{:<15} {}", "Transitive Size:".bold(), format_bytes(p.transitive_size).yellow());
    println!("{:<15} {}", "Exclusive Size:".bold(), format_bytes(p.exclusive_size).magenta());
    println!();
    println!("{}", "Description:".bold().underline());
    for line in p.description.lines() {
        println!("  {}", line);
    }
    println!();

    println!("{} ({})", "Direct Dependents".bold().underline(), p.dependents.len());
    if p.dependents.is_empty() {
        println!("  (none)");
    } else {
        let mut reqs = p.dependents.clone();
        reqs.sort_by_key(|&r| std::cmp::Reverse(graph.packages[r].installsize));
        for &req_idx in &reqs {
            let req = &graph.packages[req_idx];
            println!("  - {:<30} ({:>10})", req.name.cyan(), format_bytes(req.installsize).green());
        }
    }
    println!();

    println!("{} ({})", "Direct Dependencies".bold().underline(), p.dependencies.len());
    if p.dependencies.is_empty() {
        println!("  (none)");
    } else {
        let mut deps = p.dependencies.clone();
        deps.sort_by_key(|&d| std::cmp::Reverse(graph.packages[d].installsize));
        for &dep_idx in &deps {
            let dep = &graph.packages[dep_idx];
            println!("  - {:<30} ({:>10})", dep.name.cyan(), format_bytes(dep.installsize).green());
        }
    }
    println!("{}", "================================================================================".blue());
}
