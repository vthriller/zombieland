extern crate nix;

use std::process::Command;
use nix::sys::{signal, wait};

extern fn handle_sigchld(_: i32) {
	// zombie orphanage… more like a zombie death camp
	loop {
		// XXX `Option<WaitPidFlag>` for options? Man that is weird, especially considering the fact that `WaitPidFlag` is defined using `bitflags!()`—just like `signal::SigFlags` et al!
		match wait::waitpid(-1, Some(wait::WNOHANG)) {
			Ok(wait::WaitStatus::StillAlive) => break,
			Err(_) => break, // XXX maybe logging?
			_ => ()
		}
	}
}

fn main() {
	unsafe {
		for s in signal::Signal::iterator() {
			// > don't panic, unless your situation is really a life or death one, in which case, sure, go ahead, panic
			let _ = signal::sigaction(s, &signal::SigAction::new(
				signal::SigHandler::SigIgn,
				signal::SaFlags::empty(),
				signal::SigSet::empty()
			));
		}

		let _ = signal::sigaction(signal::SIGCHLD, &signal::SigAction::new(
			signal::SigHandler::Handler(handle_sigchld),
			signal::SaFlags::empty(),
			signal::SigSet::empty()
		));

		// TODO: SIGSEGV?
	}

	loop {
		let _ = Command::new("/sbin/agetty")
			.arg("tty1")
			.status(); // XXX should we keep spawning the process no matter what?
	}
}
