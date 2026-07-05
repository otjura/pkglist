use clap::Parser;
use colored::*;
use std::io::Write;

mod dnf;
mod flatpak;
mod graph;
mod npm;
mod bun;
mod pip;
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

    let mut pkg_inputs = Vec::new();
    let mut lines_printed = 0;

    if has_dnf() {
        if show_loading {
            print!("{}", "🔍 Loading rpm package database and resolving dependencies...".cyan().bold());
            let _ = std::io::stdout().flush();
        }
        match dnf::load_installed_packages() {
            Ok(pkgs) => {
                pkg_inputs.extend(pkgs);
                if show_loading {
                    print!("\r\x1b[2K{}\n", "✅ Loaded rpm package database and resolved dependencies.".green().bold());
                    let _ = std::io::stdout().flush();
                    lines_printed += 1;
                }
            }
            Err(err) => {
                if show_loading {
                    print!("\r\x1b[2K");
                    let _ = std::io::stdout().flush();
                }
                eprintln!("{} {}", "Error loading rpm package information:".red().bold(), err);
            }
        }
    }

    if has_flatpak() {
        if show_loading {
            print!("{}", "🔍 Loading flatpak package database and resolving dependencies...".cyan().bold());
            let _ = std::io::stdout().flush();
        }
        match flatpak::load_installed_flatpaks() {
            Ok(flatpaks) => {
                pkg_inputs.extend(flatpaks);
                if show_loading {
                    print!("\r\x1b[2K{}\n", "✅ Loaded flatpak package database and resolved dependencies.".green().bold());
                    let _ = std::io::stdout().flush();
                    lines_printed += 1;
                }
            }
            Err(err) => {
                if show_loading {
                    print!("\r\x1b[2K");
                    let _ = std::io::stdout().flush();
                }
                eprintln!("{} {}", "Warning: Error loading flatpak package information:".yellow().bold(), err);
            }
        }
    }

    if has_npm() {
        if show_loading {
            print!("{}", "🔍 Loading npm package database and resolving dependencies...".cyan().bold());
            let _ = std::io::stdout().flush();
        }
        match npm::load_installed_npm_packages() {
            Ok(npm_pkgs) => {
                pkg_inputs.extend(npm_pkgs);
                if show_loading {
                    print!("\r\x1b[2K{}\n", "✅ Loaded npm package database and resolved dependencies.".green().bold());
                    let _ = std::io::stdout().flush();
                    lines_printed += 1;
                }
            }
            Err(err) => {
                if show_loading {
                    print!("\r\x1b[2K");
                    let _ = std::io::stdout().flush();
                }
                eprintln!("{} {}", "Warning: Error loading npm package information:".yellow().bold(), err);
            }
        }
    }

    if has_bun() {
        if show_loading {
            print!("{}", "🔍 Loading bun package database and resolving dependencies...".cyan().bold());
            let _ = std::io::stdout().flush();
        }
        match bun::load_installed_bun_packages() {
            Ok(bun_pkgs) => {
                pkg_inputs.extend(bun_pkgs);
                if show_loading {
                    print!("\r\x1b[2K{}\n", "✅ Loaded bun package database and resolved dependencies.".green().bold());
                    let _ = std::io::stdout().flush();
                    lines_printed += 1;
                }
            }
            Err(err) => {
                if show_loading {
                    print!("\r\x1b[2K");
                    let _ = std::io::stdout().flush();
                }
                eprintln!("{} {}", "Warning: Error loading Bun package information:".yellow().bold(), err);
            }
        }
    }

    if has_pip() {
        if show_loading {
            print!("{}", "🔍 Loading pip package database and resolving dependencies...".cyan().bold());
            let _ = std::io::stdout().flush();
        }
        match pip::load_installed_pip_packages() {
            Ok(pip_pkgs) => {
                pkg_inputs.extend(pip_pkgs);
                if show_loading {
                    print!("\r\x1b[2K{}\n", "✅ Loaded pip package database and resolved dependencies.".green().bold());
                    let _ = std::io::stdout().flush();
                    lines_printed += 1;
                }
            }
            Err(err) => {
                if show_loading {
                    print!("\r\x1b[2K");
                    let _ = std::io::stdout().flush();
                }
                eprintln!("{} {}", "Warning: Error loading Pip package information:".yellow().bold(), err);
            }
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

    // Resolve Pip dependencies
    let mut pip_map = std::collections::HashMap::new();
    for (i, pkg) in pkg_inputs.iter().enumerate() {
        if pkg.pkg_type == graph::PackageType::Pip {
            let normalized_name = pkg.name.to_lowercase().replace('_', "-");
            pip_map.insert(normalized_name, i);
        }
    }

    let num_pkgs = pkg_inputs.len();
    for i in 0..num_pkgs {
        if pkg_inputs[i].pkg_type == graph::PackageType::Pip {
            let deps_to_resolve = pkg_inputs[i].flatpak_deps.clone();
            let mut resolved = Vec::new();
            for dep_name in deps_to_resolve {
                if let Some(&idx) = pip_map.get(&dep_name) {
                    if idx != i {
                        resolved.push(idx);
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
            if show_loading && lines_printed > 0 {
                for _ in 0..lines_printed {
                    print!("\x1b[A\r\x1b[2K");
                }
                let _ = std::io::stdout().flush();
            }
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

fn has_dnf() -> bool {
    std::process::Command::new("dnf5").arg("--version").output().is_ok()
}

fn has_flatpak() -> bool {
    std::process::Command::new("flatpak").arg("--version").output().is_ok()
}

fn has_npm() -> bool {
    std::process::Command::new("npm").arg("--version").output().is_ok()
}

fn has_bun() -> bool {
    std::process::Command::new("bun").arg("--version").output().is_ok()
}

fn has_pip() -> bool {
    std::process::Command::new("python3").arg("--version").output().is_ok()
}
