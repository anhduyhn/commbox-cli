mod protocol;
mod panels;
mod decode;
mod interactive;

use anyhow::Result;
use clap::Parser;
use inquire::Select;
use std::path::PathBuf;

use protocol::{send_frame, FrameOutcome, DEFAULT_PORT};
use panels::parse_panels_file;
use decode::format_status_value;
use interactive::select_panel_targets;

#[derive(Parser, Debug)]
#[command(name = "commbox", version, about = "CommBox panel control")]
struct Cli {
    #[arg(long, default_value = "panels.txt")]
    panels_file: PathBuf,
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
    let panel_futures = targets.into_iter().map(|(name, ip)| async move {
        let addr = if ip.contains(':') {
            ip.clone()
        } else {
            format!("{}:{}", ip, DEFAULT_PORT)
        };
        let results = query_status(&addr).await;
        (name, ip, results)
    });

    let all = futures::future::join_all(panel_futures).await;

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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let panels = parse_panels_file(&cli.panels_file).unwrap_or_else(|e| {
        eprintln!("Note: panels file not loaded ({}). Only 'Type IP' available.", e);
        Vec::new()
    });

    let actions = vec!["Status", "Quit"];

    loop {
        let action = Select::new("What would you like to do?", actions.clone()).prompt()?;
        match action {
            "Status" => match select_panel_targets(&panels) {
                Ok(targets) => run_status_for(targets).await,
                Err(e)      => eprintln!("Error: {}", e),
            },
            "Quit" => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}