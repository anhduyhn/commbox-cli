use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const READ_TIMEOUT: Duration = Duration::from_millis(800);
const CONNECTION_TIMEOUT: Duration = Duration::from_millis(2000);
pub const DEFAULT_PORT: u16 = 4660;

#[derive(Debug)]
pub enum FrameOutcome {
    ResponseOk(String), // panel returned a response
    Sent, // write succeeded, no response within read timeout
    Locked, //ERR4 - locked or wrong panel ID
    PanelError(String), // other ERR response
    Fail(String), // connect / write / read failure
}

pub async fn send_frame(panel: &str, frame: &[u8]) -> FrameOutcome {
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
pub fn extract_value(response: &str) -> &str {
    //Extracts the value in the command "!000VOLM=050" -> "050"
    response.rsplit_once('=').map(|(_, v)| v.trim()).unwrap_or(response)
}