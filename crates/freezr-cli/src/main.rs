use clap::{Parser, Subcommand};
use colored::*;
use freezr_core::{ProcessScanner, SystemdService, VERSION};

#[derive(Parser)]
#[command(name = "freezr")]
#[command(author = "FreezR Team")]
#[command(version = VERSION)]
#[command(about = "Intelligent system resource guardian - CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current status of KESL and Node processes
    Monitor,

    /// Continuous monitoring (updates every 2 seconds)
    Watch,

    /// Force restart KESL service
    ForceRestart,

    /// Show version information
    Version,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Monitor) => {
            println!("{}", "=== FreezR Monitor ===".green().bold());
            show_status();
        }
        Some(Commands::Watch) => {
            println!("{}", "=== FreezR Watch Mode ===".green().bold());
            println!("Press Ctrl+C to exit...\n");
            watch_mode();
        }
        Some(Commands::ForceRestart) => {
            println!("{}", "=== FreezR Force Restart ===".yellow().bold());
            force_restart();
        }
        Some(Commands::Version) => {
            println!("FreezR v{}", VERSION);
            println!("Intelligent system resource guardian");
        }
        None => {
            // Default: show status
            println!("{}", "=== FreezR Status ===".green().bold());
            show_status();
        }
    }
}

fn show_status() {
    let scanner = ProcessScanner::new();

    // Scan KESL
    match scanner.scan_kesl() {
        Ok(Some(process)) => {
            println!("\n{}", "✅ KESL Process Found:".green());
            println!("  PID:     {}", process.pid);
            println!("  CPU:     {:.1}%", process.cpu_percent);
            println!("  Memory:  {} MB", process.memory_mb);
        }
        Ok(None) => {
            println!("\n{}", "⚠️  KESL process not found".yellow());
        }
        Err(e) => {
            println!("\n{}", format!("❌ Error scanning KESL: {}", e).red());
        }
    }

    // Scan Node processes
    match scanner.scan_node_processes() {
        Ok(processes) => {
            if processes.is_empty() {
                println!("\n{}", "✅ No Node.js processes found".green());
            } else {
                println!("\n{} {}", "🔍 Node.js Processes:".cyan(), processes.len());
                for proc in processes {
                    let status = if proc.cpu_percent > 80.0 {
                        "🔥 HIGH CPU".red()
                    } else {
                        "✅ OK".green()
                    };
                    println!(
                        "  {} - PID: {}, CPU: {:.1}%, Memory: {} MB",
                        status, proc.pid, proc.cpu_percent, proc.memory_mb
                    );
                }
            }
        }
        Err(e) => {
            println!(
                "\n{}",
                format!("❌ Error scanning Node processes: {}", e).red()
            );
        }
    }

    println!();
}

fn watch_mode() {
    loop {
        // Clear screen
        print!("\x1B[2J\x1B[1;1H");

        show_status();

        // Sleep 2 seconds
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
}

fn force_restart() {
    let mut service = SystemdService::new("kesl");

    println!("Attempting to restart KESL service...");

    match service.restart_with_reload() {
        Ok(_) => {
            println!("{}", "✅ KESL service restarted successfully!".green());
        }
        Err(e) => {
            println!("{}", format!("❌ Failed to restart KESL: {}", e).red());
            std::process::exit(1);
        }
    }

    // Check status after restart
    match service.is_active() {
        Ok(true) => {
            println!("{}", "✅ KESL service is now active".green());
        }
        Ok(false) => {
            println!("{}", "⚠️  KESL service is not active".yellow());
        }
        Err(e) => {
            println!("{}", format!("❌ Error checking status: {}", e).red());
        }
    }
}
