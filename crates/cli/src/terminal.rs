//! Terminal handling: raw mode, signal management, interactive input.

#![allow(static_mut_refs)]

use std::io::{IsTerminal, Read, Write};

static mut SAVED_TERM: libc::termios = libc::termios {
    c_iflag: 0,
    c_oflag: 0,
    c_cflag: 0,
    c_lflag: 0,
    c_cc: [0; 20],
    c_ispeed: 0,
    c_ospeed: 0,
};

/// Flag set by SIGINT handler during interactive mode.
/// The main loop checks it after each locale and exits cleanly,
/// allowing a final save before quitting.
pub static SHOULD_EXIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// RAII guard that restores terminal settings when dropped.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        unsafe {
            restore_terminal();
        }
    }
}

unsafe fn install_sigint_handler() {
    extern "C" fn handle_sigint(_sig: i32) {
        SHOULD_EXIT.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    let mut sa: libc::sigaction = std::mem::zeroed();
    let handler: extern "C" fn(i32) = handle_sigint;
    sa.sa_sigaction = handler as usize;
    sa.sa_flags = 0;
    libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
}

unsafe fn restore_terminal() {
    libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &SAVED_TERM);
}

/// Read a line of input. Arrow-right copies `copy_val` into the input.
///
/// In a TTY, switches to raw mode to capture arrow keys without echoing
/// escape sequences. Falls back to normal read_line when piped.
pub fn read_interactive(prompt: &str, copy_val: Option<&str>) -> Result<Option<String>, String> {
    if !std::io::stdin().is_terminal() {
        // Piped input: plain read_line
        print!("{}", prompt);
        std::io::stdout().flush().ok();
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(0) => return Ok(None),
            Ok(_) => {}
            Err(e) => return Err(format!("input error: {}", e)),
        }
        let trimmed = input.trim_end_matches(['\n', '\r']).to_string();
        let copy_source = trimmed == ">" || trimmed.starts_with('\u{1b}');
        if copy_source {
            return Ok(Some(copy_val.unwrap_or("").to_string()));
        }
        return Ok(Some(trimmed));
    }

    // TTY mode: raw read to capture arrows without echo
    print!("{}", prompt);
    std::io::stdout().flush().ok();

    let _guard = TerminalGuard;
    unsafe {
        libc::tcgetattr(libc::STDIN_FILENO, &mut SAVED_TERM);
        let mut raw = SAVED_TERM;
        raw.c_lflag &= !(libc::ICANON | libc::ECHO);
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &raw);
        install_sigint_handler();
    }

    let mut result = String::new();
    let mut buf = [0u8; 1];
    loop {
        match std::io::stdin().read(&mut buf) {
            Ok(0) => {
                println!();
                break;
            }
            Ok(_) => {}
            Err(e) => return Err(format!("input error: {}", e)),
        }

        let ch = buf[0];

        if ch == b'\n' || ch == b'\r' {
            println!();
            break;
        }

        // ESC sequence (arrow keys)
        if ch == 0x1b {
            let mut seq = vec![ch];
            for _ in 0..2 {
                match std::io::stdin().read(&mut buf) {
                    Ok(1) => {
                        seq.push(buf[0]);
                        if matches!(buf[0], b'C' | b'D' | b'A' | b'B') {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            if seq.contains(&b'C') {
                if let Some(cv) = copy_val {
                    result.push_str(cv);
                    print!("{}", cv);
                    std::io::stdout().flush().ok();
                }
            }
            continue;
        }

        // Backspace
        if ch == 0x7f || ch == 0x08 {
            if !result.is_empty() {
                result.pop();
                print!("\u{8} \u{8}");
                std::io::stdout().flush().ok();
            }
            continue;
        }

        // ">" at start of buffer copies source (same as arrow-right)
        if ch == b'>' && result.is_empty() {
            if let Some(cv) = copy_val {
                result.push_str(cv);
                print!("{}", cv);
                std::io::stdout().flush().ok();
            }
            continue;
        }

        result.push(ch as char);
        print!("{}", ch as char);
        std::io::stdout().flush().ok();
    }

    Ok(Some(result))
}
