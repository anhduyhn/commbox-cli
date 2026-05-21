use anyhow::Result;
use clap::Parser;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const READ_TIMEOUT: Duration = Duration::from_millis(800);
const CONNECTION_TIMEOUT: Duration = Duration::from_millis(2000);
const DEFAULT_PORT: u16 = 4660;

#[derive(Parser, Debug)]
#[command(
    name = "commbox",
    version,
    about = "Query CommBox interactive panels over TCP/4660",
    after_help = "EXAMPLES:\n  \
                  commbox 10.128.169.30           # default port 4660\n  \
                  commbox 10.128.169.30:4660      # explicit port",
)]
struct Cli {
    // Panel's IP Address. Port defaults to 4660 if not provided.
    #[arg(value_name = "PANEL's IP ADDRESS")]
    panel: String,
}

#[derive(Debug)]
enum FrameOutcome {
    ResponseOk(String), // panel returned a response
    Sent, // write succeeded, no response within read timeout
    Locked, //ERR4 - locked or wrong panel ID
    PanelError(String), // other ERR response
    Fail(String), // connect / write / read failure
}

async fn send_frame(panel: &str, frame: &[u8]) -> FrameOutcome {
    let mut stream = match timeout(CONNECTION_TIMEOUT, TcpStream::connect(panel)).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => return FrameOutcome::Fail(format!("Connect: {}", e)),
        Err(_) => return FrameOutcome::Fail("Connection timed out".into()),
    };
    if let Err(e) = stream.write_all(frame).await {
        return FrameOutcome::Fail(format!("Write: {}", e));
    }
    if let Err(e) = stream.flush().await {
        return FrameOutcome::Fail(format!("Flush: {}", e));
    }
    let mut buf = [0u8; 256];
    let n = match timeout(READ_TIMEOUT, stream.read(&mut buf)).await {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return FrameOutcome::Fail(format!("Read: {}", e)),
        Err(_) => return FrameOutcome::Sent,
    };
    let response = String::from_utf8_lossy(&buf[..n]).trim().to_string();
    if response.is_empty()          {return FrameOutcome::Sent;}
    if response.contains("ERR4")    {return FrameOutcome::Locked;}
    if response.contains("ERR")     {return FrameOutcome::PanelError(response);}
    FrameOutcome::ResponseOk(response)
}

//Value extractor helper
fn extract_value(response: &str) -> &str {
    //Extracts the value in the command "!000VOLM=050" -> "050"
    response.rsplit_once('=').map(|(_, v)| v.trim()).unwrap_or(response)
}
#[tokio::main]
async fn main() -> Result <()> {
    let cli = Cli::parse();
    let panel: String = if cli.panel.contains(':') {
        cli.panel
    } else {
        format!("{}:{}", cli.panel, DEFAULT_PORT)
    };
    let panel = panel.as_str();

    let queries: [(&str, &[u8]); 4] = [
        ("Power", b"!000POWR ?\r"),
        ("Volume", b"!000VOLM ?\r"),
        ("Mute", b"!000MUTE ?\r"),
        ("Input", b"!000INPT ?\r"),
    ];

    let futures = queries.iter().map(|(label, frame) | async move {
        (*label, send_frame(panel, frame).await)
    });

    let results = futures::future::join_all(futures).await;

    println!("{}", panel);
    for (label, outcome) in results {
        match outcome {
            FrameOutcome::ResponseOk(response)  => println!(" {:6} {}", label, extract_value(&response)),
            FrameOutcome::Sent                  => println!(" {:6} (no response)", label),
            FrameOutcome::Locked                => println!(" {:6} LOCKED", label),
            FrameOutcome::PanelError(response)  => println!(" {:6} ERR: {}", label, response),
            FrameOutcome::Fail(why)             => println!(" {:6} FAIL: {}", label, why),
        }
    }
    Ok(())
}
