use crate::protocol::{extract_value, FrameOutcome};

fn decode_onoff(value: &str) -> String {
    if value.trim().trim_start_matches('0').is_empty() {
        "OFF".to_string()
    } else {
        "ON".to_string()
    }
}

fn decode_volume(value: &str) -> String {
    value.trim().parse::<u32>()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| value.to_string())
}

fn decode_input(value: &str) -> String {
    match value.trim() {
        "111" => "VGA".into(),
        "131" => "AV".into(),
        "151" => "YPbPr".into(),
        "211" => "HDMI 1 (Vivi)".into(),
        "212" => "HDMI 2".into(),
        "213" => "HDMI 3".into(),
        "214" => "HDMI 4".into(),
        "231" => "DisplayPort".into(),
        "241" => "PC (OPS)".into(),
        "271" => "USB-C 1".into(),
        "272" => "USB-C 2".into(),
        "411" => "Android".into(),
        other => other.to_string(),
    }
}

pub fn format_status_value(label: &str, outcome: &FrameOutcome) -> String {
    match outcome {
        FrameOutcome::ResponseOk(response) => {
            let raw = extract_value(response);
            match label {
                "Power" | "Mute" => decode_onoff(raw),
                "Volume"         => decode_volume(raw),
                "Input"          => decode_input(raw),
                _                => raw.to_string(),
            }
        }
        FrameOutcome::Sent          => "(no response)".to_string(),
        FrameOutcome::Locked        => "LOCKED".to_string(),
        FrameOutcome::PanelError(r) => format!("ERR ({})", r),
        FrameOutcome::Fail(why)     => format!("FAIL ({})", why),
    }
}
pub fn format_set_outcome(outcome: &FrameOutcome) -> String {
    match outcome {
        FrameOutcome::ResponseOk(r) => format!("OK ({})", extract_value(r)),
        FrameOutcome::Sent          => "OK (no echo)".to_string(),
        FrameOutcome::Locked        => "LOCKED".to_string(),
        FrameOutcome::PanelError(r) => format!("ERR ({})", r),
        FrameOutcome::Fail(why)     => format!("FAIL ({})", why),
    }
}