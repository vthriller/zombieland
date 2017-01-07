// > error: non-ident macro paths are experimental (see issue #35896)
#![feature(use_extern_macros)]
// #31398
#![feature(process_exec)]

extern crate nix;
extern crate syscall;

use std::process::Command;
use std::os::unix::process::CommandExt;
use nix::sys::{signal, wait};
use nix::unistd;

use std::fs::File;
use std::io::BufReader;
use std::io::BufRead;
use std::collections::HashMap;

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

extern fn handle_sigint(_: i32) {
	// XXX re-reading configuration file isn't that effective, but hey, you don't smash that three-key chord constantly do you?
	let conf = read_config();

	match conf.get("ctrlaltdel") {
		Some(s) => {
			// sure process::Child does not implement the Drop trait, but it does not really make that much of a difference:
			// * this is ctrl-alt-del handler, so we're probably going to die soon anyways;
			// * this is signal handler, so we'd rather return as soon as possible;
			// * as for zombie children situation, we already have `handle_sigchld`.
			let _ = Command::new(s).spawn(); // we clearly couldn't care less about the outcome of this one
		},
		None => {}
	};
}

fn read_config() -> HashMap<String, String> {
	let mut conf = HashMap::new();

	let f = match File::open("/etc/zombieland/conf") {
		Err(_) => return conf, // TODO logging?
		Ok(file) => file
	};
	let f = BufReader::new(&f);

	// poor man's parser
	for line in f.lines() {
		let line = line.unwrap();
		let line = line.trim();

		// ignore comments and empty lines
		if line.starts_with('#') { continue; }
		if line.is_empty() { continue; }

		let mut tokens = line.splitn(2,
			|c| c == ' ' || c == '\t'
		);
		let k = match tokens.next() {
			Some(s) => s.to_string(),
			None => continue
		};
		let v = match tokens.next() {
			Some(s) => s.to_string(),
			None => continue
		};
		conf.insert(k, v);
	}

	conf
}

fn main() {
	// I have no clue why sysvinit (or any other init, for that matter) does that, but at least it makes pid 1 visible in htop (yeah, I know)
	let _ = unistd::setsid(); // should not get EPERM

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

		let _ = signal::sigaction(signal::SIGINT, &signal::SigAction::new(
			signal::SigHandler::Handler(handle_sigint),
			signal::SaFlags::empty(),
			signal::SigSet::empty()
		));

		// TODO: SIGSEGV?
	}

	#[cfg(target_os = "linux")]
	unsafe {
		// if Ctrl-Alt-Del is pressed, `kill -INT 1` instead of hard-rebooting the system
		syscall::syscall!(REBOOT, 0xfee1dead, 0x20112000, 0);
	}

	let conf = read_config();

	match conf.get("boot") {
		Some(s) => {
			let _ = Command::new(s).status(); // XXX should we do something if anything goes wrong?
		},
		None => {}
	};

	loop {
		let mut cmd;
		match conf.get("main") {
			Some(s) => {
				cmd = Command::new(s);
			},
			None => {
				cmd = Command::new("/sbin/agetty");
				let _ = cmd.arg("tty1");
			}
		};
		let _ = cmd.before_exec(|| { let _ = unistd::setsid(); Ok(()) });
		let _ = cmd.status(); // XXX should we keep spawning the process no matter what?
	}
}
