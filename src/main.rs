use clap::Parser;
use colored::*;

mod dnf;
mod flatpak;
mod graph;
mod npm;
mod cli;
mod tui;

fn main() {
    let args = cli::Cli::parse();

    // Show a loading message if we are printing styled terminal output
    let show_loading = match &args.command {
        Some(cli::Commands::List { format, .. }) => *format == cli::Format::Table,
        Some(cli::Commands::Info { .. }) => true,
        None => true,
    };

    if show_loading {
        println!("{}", "🔍 Loading package database and resolving dependencies...".cyan().bold());
    }

    let mut pkg_inputs = match dnf::load_installed_packages() {
        Ok(pkgs) => pkgs,
        Err(err) => {
            eprintln!("{} {}", "Error loading package information:".red().bold(), err);
            std::process::exit(1);
        }
    };

    match flatpak::load_installed_flatpaks() {
        Ok(flatpaks) => {
            pkg_inputs.extend(flatpaks);
        }
        Err(err) => {
            eprintln!("{} {}", "Warning: Error loading flatpak package information:".yellow().bold(), err);
        }
    }

    match npm::load_installed_npm_packages() {
        Ok(npm_pkgs) => {
            pkg_inputs.extend(npm_pkgs);
        }
        Err(err) => {
            eprintln!("{} {}", "Warning: Error loading npm package information:".yellow().bold(), err);
        }
    }

    // Resolve Flatpak runtime and SDK dependencies
    let mut flatpak_map = std::collections::HashMap::new();
    for (i, pkg) in pkg_inputs.iter().enumerate() {
        if pkg.pkg_type == graph::PackageType::Flatpak {
            flatpak_map.insert((pkg.name.clone(), pkg.release.clone()), i);
        }
    }

    let mut flatpak_name_map = std::collections::HashMap::new();
    for (i, pkg) in pkg_inputs.iter().enumerate() {
        if pkg.pkg_type == graph::PackageType::Flatpak {
            flatpak_name_map.insert(pkg.name.clone(), i);
        }
    }

    let num_pkgs = pkg_inputs.len();
    for i in 0..num_pkgs {
        if pkg_inputs[i].pkg_type == graph::PackageType::Flatpak {
            let deps_to_resolve = pkg_inputs[i].flatpak_deps.clone();
            let mut resolved = Vec::new();
            for ref_str in deps_to_resolve {
                if let Some((runtime_id, branch)) = parse_runtime_ref(&ref_str) {
                    let mut resolved_idx = flatpak_map.get(&(runtime_id.clone(), branch.clone())).copied();
                    if resolved_idx.is_none() {
                        resolved_idx = flatpak_name_map.get(&runtime_id).copied();
                    }
                    if let Some(idx) = resolved_idx {
                        if idx != i {
                            resolved.push(idx);
                        }
                    }
                }
            }
            pkg_inputs[i].resolved_deps = resolved;
        }
    }

    let graph = graph::build_graph(pkg_inputs);

    match args.command {
        Some(cli::Commands::List { sort, limit, search, format }) => {
            cli::run_list(&graph, sort, limit, search, format);
        }
        Some(cli::Commands::Info { package }) => {
            cli::run_info(&graph, &package);
        }
        None => {
            if let Err(err) = tui::run_tui(&graph) {
                eprintln!("{} {}", "TUI Error:".red().bold(), err);
                std::process::exit(1);
            }
        }
    }
}

fn parse_runtime_ref(ref_str: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = ref_str.split('/').collect();
    if parts.len() >= 3 {
        Some((parts[0].to_string(), parts[2].to_string()))
    } else if parts.len() == 1 && !parts[0].is_empty() {
        Some((parts[0].to_string(), String::new()))
    } else {
        None
    }
}
