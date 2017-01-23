extern crate std;
extern crate libc;

use std::mem::{uninitialized, transmute};

// SIGCHLD
/*
typedef struct {
	int si_signo;
	int si_errno;
	int si_code;
	union {
		// …
		struct {
			__pid_t si_pid;
			__uid_t si_uid;
			int si_status;
			__sigchld_clock_t si_utime;
			__sigchld_clock_t si_stime;
		} _sigchld;
		// …
	}
} siginfo_t;
*/
// On x86_64 linux, this would be "i32 i32 i32 (i32 padding) | i32 i32 i32 (i32 padding) i64 i64 ..."; union is padded because _sigpoll starts with `long int`
// _sigchld alone, when padded, is also "i32 i32 i32 (i32 padding) i64 i64 ..."
// but for some reason if no `#[repr(packed)]` is used and `_pad1` is omitted, depending on the size of the `_pad2`, it's either 896 or 960 bits and not 928 which is the size of `siginfo_t._pad`
// FIXME? x32 abi: typedef __clock_t __attribute__ ((__aligned__ (4))) __sigchld_clock_t;
#[cfg(target_os = "linux")]
#[repr(C)]
#[repr(packed)]
struct siginfo_pad_sigchld {
	// `siginfo_t` does not have any union alignment since it's defined as a simple bunch of `c_int`s and knows nothing about _sigpoll's `long`
	// so we account for it here
	_siginfo_dummy_pad: libc::c_int,
	si_pid: libc::pid_t,
	si_uid: libc::uid_t,
	si_status: libc::c_int,
	_pad1: libc::c_int,
	si_utime: libc::clock_t,
	si_stime: libc::clock_t,
	_pad2: [libc::c_int; 20]
}

// TODO arguments?
// TODO return Option<Siginfo> or something?
// XXX too much panicking, probably; don't forget we're pid #1
pub fn waitid() -> Result<Option<libc::pid_t>, std::io::Error> {
	// > Applications shall specify at least one of the flags WEXITED, WSTOPPED, or WCONTINUED to be OR'ed in with the options argument.

	unsafe {
		let mut info: libc::siginfo_t = uninitialized();
		match libc::waitid(libc::P_ALL, 0, &mut info, libc::WEXITED | libc::WNOWAIT | libc::WNOHANG) {
			0 => (),
			-1 => return Err(std::io::Error::last_os_error()), // FIXME: std::io::Error isn't quite idiomatic …yet I don't feel like copying-and-pasting or re-implementing `last_os_error()` is worth it
			wtf => panic!("waitid() returned {}", wtf)
		};

		match info.si_signo {
			0 => return Ok(None), // TODO: check if there is WNOHANG in the arguments
			libc::SIGCHLD => (),
			wtf => panic!("waitid() returned information about signal {}, not SIGCHLD", wtf)
		}

		let info_sigchld: siginfo_pad_sigchld = transmute(info._pad);

		Ok(Some(info_sigchld.si_pid))
	}
}
