use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::process;
use std::thread;

extern crate libc;
extern crate termios;

fn open_pty_master() -> RawFd {
    let master_fd = unsafe { libc::posix_openpt(libc::O_RDWR) };
    unsafe { libc::grantpt(master_fd) };
    unsafe { libc::unlockpt(master_fd) };
    master_fd
}

fn get_pty_slave_name(master_fd: RawFd) -> CString {
    let slave_name = unsafe { CStr::from_ptr(libc::ptsname(master_fd)) };
    slave_name.to_owned()
}

fn child_process(master_fd: RawFd, slave_name: CString) {
	unsafe {
		// Close the master fd
		libc::close(master_fd);

		// Open the slave fd
		let slave_fd = libc::open(slave_name.as_ptr(), libc::O_RDWR);

		// Set the terminal attributes for the child process
		let mut term: termios::Termios = std::mem::zeroed();
		termios::tcgetattr(slave_fd, &mut term).unwrap();
		termios::cfmakeraw(&mut term);
		termios::tcsetattr(slave_fd, termios::TCSANOW, &term).unwrap();

		// Wire up the child fds
		libc::dup2(slave_fd, libc::STDIN_FILENO);
		libc::dup2(slave_fd, libc::STDOUT_FILENO);
		libc::dup2(slave_fd, libc::STDERR_FILENO);

		let mut winsize: libc::winsize = std::mem::zeroed();
		libc::ioctl(libc::STDIN_FILENO, libc::TIOCGWINSZ, &mut winsize as *mut _);
		libc::ioctl(slave_fd, libc::TIOCSWINSZ, &winsize as *const _);

		// Start the shell or other program
		libc::execl(
			b"/bin/bash\0".as_ptr() as *const libc::c_char,
			b"/bin/bash\0".as_ptr() as *const libc::c_char,
			std::ptr::null() as *const libc::c_char,
		);
	}
}

fn parent_setup() -> termios::Termios {
	// Set the parent's terminal to raw mode
	let mut parent_term: termios::Termios = unsafe { std::mem::zeroed() };
	termios::tcgetattr(libc::STDIN_FILENO, &mut parent_term).unwrap();
	let mut raw_parent_term = parent_term.clone();
	termios::cfmakeraw(&mut raw_parent_term);
	termios::tcsetattr(libc::STDIN_FILENO, termios::TCSANOW, &raw_parent_term).unwrap();
    parent_term
}

fn handle_stdout(master_fd: RawFd) {
    let mut master = unsafe { std::fs::File::from_raw_fd(master_fd) };

    let mut buf = [0; 4096 * 16];
    loop {
        match master.read(&mut buf) {
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

fn handle_stdin(master_fd: RawFd) {
    let mut master = unsafe { std::fs::File::from_raw_fd(master_fd) };

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
                    let _ = master.write_all(&[*b]);
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

fn parent_process(master_fd: RawFd) {

	let _stdout_thread = thread::spawn(move || {
        handle_stdout(master_fd);
    });

    let _stdin_thread = thread::spawn(move || {
        handle_stdin(master_fd);
    });
}

fn parent_cleanup(parent_term: termios::Termios) {
    // Restore the parent's terminal settings
    termios::tcsetattr(libc::STDIN_FILENO, termios::TCSANOW, &parent_term).unwrap();
}

fn main() {
    let master_fd = open_pty_master();
    let slave_name = get_pty_slave_name(master_fd);

    let pid = unsafe { libc::fork() };
    if pid == 0 {
        child_process(master_fd, slave_name);
    } else {
        let parent_term = parent_setup();
        parent_process(master_fd);

        // Wait for the child process to exit
        let mut status: libc::c_int = 0;
        unsafe { libc::waitpid(pid, &mut status, 0) };

        parent_cleanup(parent_term);
        process::exit(0);
    }
}

