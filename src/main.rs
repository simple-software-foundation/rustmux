use std::ffi::{CStr, CString};
use std::io::{Read, Write};
use std::os::unix::io::{FromRawFd, RawFd};
use std::process;
use std::thread;

extern crate libc;
extern crate termios;

mod mux;

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

fn parent_process(raw_master_fd: RawFd, child_pid: i32) {
    let mux = mux::Mux::new();

    let _stdout_thread = thread::spawn(move || {
        let mut master_fd = unsafe { std::fs::File::from_raw_fd(raw_master_fd) };
        mux::Mux::handle_stdout(&mut master_fd);
    });

    let _stdin_thread = thread::spawn(move || {
        let mut master_fd = unsafe { std::fs::File::from_raw_fd(raw_master_fd) };
        mux::Mux::handle_stdin(&mut master_fd);
    });

    // Wait for the child process to exit
    let mut status: libc::c_int = 0;
    unsafe { libc::waitpid(child_pid, &mut status, 0) };

    mux.cleanup();
    process::exit(0);
}


fn main() {
    let master_fd = open_pty_master();
    let slave_name = get_pty_slave_name(master_fd);

    let pid = unsafe { libc::fork() };
    if pid == 0 {
        child_process(master_fd, slave_name);
    } else {
        parent_process(master_fd, pid);
    }
}

