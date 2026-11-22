//! Dev harness: replay a live `forge-preview-ratatui` frame stream through
//! the emulator the way Studio's card does — resize to the frame's grid,
//! then feed the frame bytes. Prints non-space cells per frame so a stream
//! that lands as a blank grid is visible.
//!
//! `cargo run -p forge-term --example stream_replay`

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Field puller for the flat JSON the host emits — `"key":value` on one line.
fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{key}\":");
    let start = line.find(&pat)? + pat.len();
    let rest = line[start..].trim_start();
    if let Some(quoted) = rest.strip_prefix('"') {
        // String value — scan for the unescaped closing quote.
        let mut end = 0;
        let mut esc = false;
        for (i, c) in quoted.char_indices() {
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                end = i;
                break;
            }
        }
        return Some(&quoted[..end]);
    }
    let end = rest.find([',', '}']).unwrap_or(rest.len());
    Some(&rest[..end])
}

/// Unescape a JSON string body (the host only emits \" \\ and \uXXXX).
fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                if let Some(c) = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32) {
                    out.push(c);
                }
            }
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn main() {
    let exe = std::env::current_exe().unwrap();
    let host = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("forge-preview-ratatui");
    let mut child = Command::new(&host)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn host");
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let doc = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/demos/ratatui-dashboard.forge.yaml".into());
    writeln!(stdin, "{{\"cmd\":\"load\",\"doc\":\"{doc}\"}}").unwrap();
    // Mimic Studio's frame_rect — the Theme tab slot's px size.
    writeln!(
        stdin,
        "{{\"cmd\":\"frame_rect\",\"x\":800.0,\"y\":190.0,\"w\":1041.0,\"h\":464.0}}"
    )
    .unwrap();
    stdin.flush().unwrap();

    let mut emu: Option<forge_term::Emulator> = None;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let event = match field(&line, "event") {
            Some(e) => e.to_owned(),
            None => continue,
        };
        if event != "frame" {
            eprintln!("event={event}");
            continue;
        }
        let seq: u64 = field(&line, "seq").and_then(|s| s.parse().ok()).unwrap();
        let cols: u16 = field(&line, "cols").and_then(|s| s.parse().ok()).unwrap();
        let rows: u16 = field(&line, "rows").and_then(|s| s.parse().ok()).unwrap();
        let data = unescape(field(&line, "data").unwrap_or(""));
        let emu = emu.get_or_insert_with(|| forge_term::Emulator::new(cols, rows));
        if emu.cols() != cols as usize || emu.rows() != rows as usize {
            // Grid change → fresh emulator: sparse diffs only land right on
            // a blank grid (see feed_term_frame in gpui_kit.rs).
            *emu = forge_term::Emulator::new(cols, rows);
        }
        emu.feed(data.as_bytes());
        let nonzero: usize = (0..emu.rows())
            .map(|r| emu.line(r).iter().filter(|c| c.ch != ' ').count())
            .sum();
        eprintln!(
            "frame seq={seq} {cols}x{rows} data_len={} -> nonzero_cells={nonzero}",
            data.len()
        );
        if seq >= 40 {
            for r in 0..emu.rows().min(12) {
                let line = emu.line(r);
                let s: String = line.iter().map(|c| c.ch).collect();
                println!("{:2}|{}", r, s.trim_end());
            }
            break;
        }
    }
    let _ = writeln!(stdin, "{{\"cmd\":\"quit\"}}");
    let _ = stdin.flush();
    let _ = child.wait();
}
