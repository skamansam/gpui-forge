//! Dev harness: feed a captured ANSI stream (e.g. a `frame` event's `data`
//! field from `forge-preview-ratatui`) through the emulator and print the
//! styled grid + a sample styled cell.
//!
//! `cargo run -p forge-term --example emu_check -- /tmp/frame.ansi`

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: emu_check <ansi-capture-file>");
    let data = std::fs::read_to_string(path).unwrap();
    let mut emu = forge_term::Emulator::new(100, 30);
    emu.feed(data.as_bytes());
    for r in 0..emu.rows().min(12) {
        let line = emu.line(r);
        let s: String = line.iter().map(|c| c.ch).collect();
        let end = s.char_indices().nth(80).map(|(i, _)| i).unwrap_or(s.len());
        println!("{:2}|{}", r, &s[..end]);
    }
    for r in 0..emu.rows() {
        let line = emu.line(r);
        if let Some(c) = line
            .iter()
            .find(|c| c.bold || c.fg != forge_term::CellColor::Foreground)
        {
            println!(
                "first styled cell: row {r} {:?} fg={:?} bg={:?} bold={} underline={}",
                c.ch, c.fg, c.bg, c.bold, c.underline
            );
            break;
        }
    }
}
