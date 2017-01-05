use std::process::Command;

fn main() {
	loop {
		let _ = Command::new("/sbin/agetty")
			.arg("tty1")
			.status(); // XXX should we keep spawning the process no matter what?
	}
}
