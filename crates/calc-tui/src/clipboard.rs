use std::io::Write;
use std::process::{Command, Stdio};

/// Copy text to the system clipboard.
/// Uses OSC 52 escape sequence (works in most modern terminals)
/// plus platform-specific fallback commands.
pub fn copy(text: &str) {
    // OSC 52: works in Windows Terminal, iTerm2, Alacritty, kitty, etc.
    let encoded = base64_encode(text.as_bytes());
    let osc = format!("\x1b]52;c;{}\x07", encoded);
    let _ = std::io::stdout().write_all(osc.as_bytes());
    let _ = std::io::stdout().flush();

    // Platform fallback: pipe to a clipboard command
    let commands: &[&[&str]] = &[
        &["pbcopy"],                                          // macOS
        &["clip.exe"],                                        // WSL2
        &["xclip", "-selection", "clipboard"],                // Linux X11
        &["xsel", "--clipboard", "--input"],                  // Linux X11 alt
        &["wl-copy"],                                         // Wayland
    ];
    for cmd in commands {
        if let Ok(mut child) = Command::new(cmd[0])
            .args(&cmd[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return;
        }
    }
}

/// Read text from the system clipboard.
/// Tries platform-specific commands in order.
pub fn paste() -> Option<String> {
    let commands: &[&[&str]] = &[
        &["pbpaste"],                                         // macOS
        &["powershell.exe", "-command", "Get-Clipboard"],     // WSL2
        &["xclip", "-selection", "clipboard", "-o"],          // Linux X11
        &["xsel", "--clipboard", "--output"],                 // Linux X11 alt
        &["wl-paste", "--no-newline"],                        // Wayland
    ];
    for cmd in commands {
        if let Ok(output) = Command::new(cmd[0])
            .args(&cmd[1..])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
        {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                // powershell adds a trailing \r\n
                return Some(text.trim_end_matches("\r\n").trim_end_matches('\n').to_string());
            }
        }
    }
    None
}

const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(input: &[u8]) -> String {
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(BASE64_CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(BASE64_CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(BASE64_CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(BASE64_CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
