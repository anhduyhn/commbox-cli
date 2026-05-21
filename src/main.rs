mod protocol;
mod panels;
mod decode;
mod interactive;

use anyhow::Result;
use clap::Parser;
use inquire::{Confirm, CustomType, Select};
use std::path::PathBuf;
use std::future::Future;
use std::time::Duration;
use indicatif::{ProgressBar, ProgressStyle};

use protocol::{send_frame, FrameOutcome, DEFAULT_PORT};
use panels::parse_panels_file;
use decode::{format_status_value, format_set_outcome, INPUT_ALIASES};
use inquire::validator::Validation;
use interactive::select_panel_targets;

#[derive(Parser, Debug)]
#[command(name = "commbox", version, about = "CommBox panel control")]
struct Cli {
    #[arg(long, default_value = "panels.txt")]
    panels_file: PathBuf,
}
async fn with_spinner<F: Future>(message: impl Into<String>, fut: F) -> F::Output {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner());
    pb.set_message(message.into());
    pb.enable_steady_tick(Duration::from_millis(100));
    let result = fut.await;
    pb.finish_and_clear();
    result
}
async fn query_status(panel: &str) -> Vec<(&'static str, FrameOutcome)> {
    let queries: [(&str, &[u8]); 4] = [
        ("Power",  b"!000POWR ?\r"),
        ("Volume", b"!000VOLM ?\r"),
        ("Mute",   b"!000MUTE ?\r"),
        ("Input",  b"!000INPT ?\r"),
    ];
    let futures = queries.iter().map(|(label, frame)| async move {
        (*label, send_frame(panel, frame).await)
    });
    futures::future::join_all(futures).await
}

async fn run_status_for(targets: Vec<(String, String)>) {
    let message = if targets.len() == 1 {
        "Querying panel...".to_string()
    } else {
        format!("Querying {} panels...", targets.len())
    };
    let panel_futures = targets.into_iter().map(|(name, ip)| async move {
        let addr = if ip.contains(':') {
            ip.clone()
        } else {
            format!("{}:{}", ip, DEFAULT_PORT)
        };
        let results = query_status(&addr).await;
        (name, ip, results)
    });

    let all = with_spinner(
        message,
        futures::future::join_all(panel_futures),
    ).await;

    for (name, ip, results) in all {
        println!();
        if name == ip {
            println!("{}", ip);
        } else {
            println!("{} ({})", name, ip);
        }
        for (label, outcome) in results {
            println!("  {:7} {}", format!("{}:", label), format_status_value(label, &outcome));
        }
    }
}
async fn run_set_for(
    targets: Vec<(String, String)>,
    action_label: &str,
    value_display: &str,
    frame: String,
) {
    let message = if targets.len() == 1 {
        format!("Setting {} to {}...", action_label, value_display)
    } else {
        format!(
            "Setting {} to {} on {} panels...",
            action_label, value_display, targets.len()
        )
    };

    let panel_futures = targets.into_iter().map(|(name, ip)| {
        let frame = frame.clone();
        async move {
            let addr = if ip.contains(':') {
                ip.clone()
            } else {
                format!("{}:{}", ip, DEFAULT_PORT)
            };
            let outcome = send_frame(&addr, frame.as_bytes()).await;
            (name, ip, outcome)
        }
    });

    let all = with_spinner(
        message,
        futures::future::join_all(panel_futures),
    ).await;

    println!();
    for (name, ip, outcome) in all {
        let label = if name == ip { ip.clone() } else { format!("{} ({})", name, ip) };
        println!("  {:30} {}", label, format_set_outcome(&outcome));
    }
}

fn confirm_if_many(count: usize, summary: &str) -> Result<bool> {
    if count <= 5 {
        return Ok(true);
    }
    let confirmed = Confirm::new(&format!("{} on {} panels. Continue?", summary, count))
        .with_default(false)
        .prompt()?;
    Ok(confirmed)
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let panels = parse_panels_file(&cli.panels_file).unwrap_or_else(|e| {
        eprintln!("Note: panels file not loaded ({}). Only 'Type IP' available.", e);
        Vec::new()
    });

    let actions = vec!["Status", "Volume", "Input", "Quit"];

    loop {
        let action = Select::new("What would you like to do?", actions.clone()).prompt()?;
        match action {
            "Status" => match select_panel_targets(&panels) {
                Ok(targets) => run_status_for(targets).await,
                Err(e)      => eprintln!("Error: {}", e),
            },
            "Volume" => match select_panel_targets(&panels) {
                Ok(targets) => {
                    let level = CustomType::<u8>::new("Volume level (0-100):")
                        .with_error_message("Please enter a number")
                        .with_validator(|v: &u8| {
                            if *v <= 100 {
                                Ok(Validation::Valid)
                            } else {
                                Ok(Validation::Invalid("Must be 0-100".into()))
                            }
                        })
                        .prompt()?;

                    if !confirm_if_many(targets.len(), &format!("Set volume to {}", level))? {
                        continue;
                    }

                    let frame = format!("!000VOLM {}\r", level);
                    run_set_for(targets, "volume", &level.to_string(), frame).await;
                }
                Err(e) => eprintln!("Error: {}", e),
            },
            "Input" => match select_panel_targets(&panels) {
                Ok(targets) => {
                    let labels: Vec<&str> = INPUT_ALIASES.iter().map(|(_, l)| *l).collect();
                    let chosen = Select::new("Input?", labels).prompt()?;
                    let code = INPUT_ALIASES.iter().find(|(_, l)| *l == chosen).unwrap().0;

                    if !confirm_if_many(targets.len(), &format!("Switch input to {}", chosen))? {
                        continue;
                    }

                    let frame = format!("!000INPT {}\r", code);
                    run_set_for(targets, "input", chosen, frame).await;
                }
                Err(e) => eprintln!("Error: {}", e),
            },
            "Quit" => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}