use std::os::unix::io::{FromRawFd, RawFd};
use std::io::{Read, Write};

extern crate termios;

pub struct Mux {
    internals: MuxInternals,
}

struct MuxInternals {
    original_term: termios::Termios,
}

impl Mux {
    pub fn new() -> Self {
        // Set the parent's terminal to raw mode
        let mut original_term: termios::Termios = unsafe { std::mem::zeroed() };
        termios::tcgetattr(libc::STDIN_FILENO, &mut original_term).unwrap();
        let mut new_term = original_term.clone();
        termios::cfmakeraw(&mut new_term);
        termios::tcsetattr(libc::STDIN_FILENO, termios::TCSANOW, &new_term).unwrap();
        Self { internals: MuxInternals { original_term } }
    }

    pub fn cleanup(& self) {
        // Restore the parent's terminal settings
        termios::tcsetattr(libc::STDIN_FILENO, termios::TCSANOW, &self.internals.original_term).unwrap();
    }

    pub fn handle_stdout(master_fd: &mut std::fs::File) {
        let mut buf = [0; 4096 * 16];
        loop {
            match master_fd.read(&mut buf) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    let mut output_data = Vec::with_capacity(n);
                    for b in &buf[..n] {
                        output_data.push(*b);
                        if *b == b'\n' {
                            output_data.push(b'\r');
                        }
                    }
                    let stdout = std::io::stdout();
                    let mut stdout_lock = stdout.lock();
                    let _ = stdout_lock.write_all(&output_data);
                    let _ = stdout_lock.flush();
                }
                Err(_) => break,
            }
        }
    }

    pub fn handle_stdin(master_fd: &mut std::fs::File) {
        let mut buf = [0; 4096 * 16];
        loop {
            match std::io::stdin().read(&mut buf) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    let stdout = std::io::stdout();
                    let mut stdout_lock = stdout.lock();
                    for b in &buf[..n] {
                        let _ = master_fd.write_all(&[*b]);
                        if *b != b'\r' {
                            let _ = stdout_lock.write_all(&[*b]);
                        }
                    }
                    let _ = stdout_lock.flush();
                }
                Err(_) => break,
            }
        }
    }
}
