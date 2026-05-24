use clap::Parser;
use colored::*;

mod dnf;
mod graph;
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

    let pkg_inputs = match dnf::load_installed_packages() {
        Ok(pkgs) => pkgs,
        Err(err) => {
            eprintln!("{} {}", "Error loading package information:".red().bold(), err);
            std::process::exit(1);
        }
    };

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
