// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2025 rtldg <rtldg@protonmail.com>

#![allow(non_snake_case)]
// TODO: Bleh, static muts...
#![allow(static_mut_refs)]

use std::ffi::c_char;
use std::ffi::c_void;
use std::io::Write;
use std::ptr::NonNull;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;
use std::thread::JoinHandle;

use extshared::ICellArray::ICellArray;
use extshared::cpp_add_frame_action;
use extshared::cpp_extension_log_error;
use extshared::cpp_forward_execute;
use extshared::cpp_forward_push_cell;
use extshared::cpp_forward_push_string;
use extshared::strxx;

extshared::smext_conf_boilerplate_extension_info!(description, version, author, datestring, url, logtag, license, load);
#[unsafe(no_mangle)]
pub extern "C" fn rust_conf_name() -> *const u8 {
	"srcwr💾\0".as_ptr()
}
extshared::smext_conf_boilerplate_load_funcs!();

static mut SENDER: Option<Sender<Msg>> = None;
static mut THREAD: Option<JoinHandle<()>> = None;

#[derive(Debug)]
struct Msg {
	forward:          NonNull<c_void>,
	value:            i32,
	wrpath:           String,
	copypath:         String,
	header:           Vec<u8>,
	playerrecording:  *mut ICellArray,
	totalframes:      usize,
	sm_friendly_path: String,
}
unsafe impl Send for Msg {} // so we can store the pointers...

struct Callbacker {
	forward: NonNull<c_void>,
	//saved: bool,
	value:   i32,
	path:    String,
}
unsafe impl Send for Callbacker {} // so we can store the pointers...

#[unsafe(no_mangle)]
pub extern "C" fn rust_setup_replay_thread() {
	let (send, recv) = channel();
	unsafe {
		SENDER = Some(send);
		THREAD = Some(
			std::thread::Builder::new()
				.name("srcwrfloppy replay thread".to_string())
				.spawn(move || replay_thread(recv))
				.unwrap(),
		)
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_KILL_replay_thread() {
	unsafe {
		SENDER = None; // closes channel
		THREAD.take().unwrap().join().unwrap();
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_post_to_replay_thread(
	forward: NonNull<c_void>,
	value: i32,
	wrpath: *const c_char,
	copypath: *const c_char,
	header: *const u8,
	headersize: usize,
	playerrecording: *mut ICellArray,
	totalframes: usize,
	sm_friendly_path: *const c_char,
) {
	let wrpath = strxx(wrpath, false, 0).unwrap_or_default().to_string();
	let copypath = strxx(copypath, false, 0).unwrap_or_default().to_string();
	let sm_friendly_path = strxx(sm_friendly_path, false, 0).unwrap().to_string();

	let header = unsafe { std::slice::from_raw_parts(header, headersize).to_vec() };

	//println!("hello from poster!");
	unsafe {
		if let Some(sender) = &SENDER {
			sender
				.send(Msg {
					forward,
					value,
					wrpath,
					copypath,
					header,
					playerrecording,
					totalframes,
					sm_friendly_path,
				})
				.unwrap();
			//println!("posted!");
		}
	}
}

fn replay_thread(recv: Receiver<Msg>) {
	while let Ok(msg) = recv.recv() {
		//println!("received {msg:?}");

		let mut fcopy = None;
		let mut fwr = None;

		if !msg.copypath.is_empty() {
			if let Ok(f) = std::fs::File::create(&msg.copypath).map(std::io::BufWriter::new) {
				fcopy = Some(f);
			} else {
				log_error(format!("Failed to open 'copy' replay file for writing. ('{}')", msg.copypath));
			}
		}

		if !msg.wrpath.is_empty() {
			if let Ok(f) = std::fs::File::create(&msg.wrpath).map(std::io::BufWriter::new) {
				fwr = Some(f);
			} else {
				log_error(format!("Failed to open WR replay file for writing. ('{}')", msg.wrpath));
			}
		}

		if fcopy.is_none() && fwr.is_none() {
			continue;
		}

		let cellarray = unsafe { &mut *msg.playerrecording };
		let frames =
			unsafe { std::slice::from_raw_parts(cellarray.data as *const u8, cellarray.blocksize * 4 * msg.totalframes) };

		if let Some(f) = &mut fwr {
			let _ = f.write_all(&msg.header);
			let _ = f.write_all(frames);
		}
		if let Some(f) = &mut fcopy {
			let _ = f.write_all(&msg.header);
			let _ = f.write_all(frames);
		}

		if let Some(mut f) = fcopy {
			let _ = f.flush();
		}
		if let Some(mut f) = fwr {
			let _ = f.flush();
		}

		unsafe {
			cpp_add_frame_action(
				do_callback,
				Box::leak(Box::new(Callbacker {
					forward: msg.forward,
					//saved: ?
					value:   msg.value,
					path:    msg.sm_friendly_path,
				})) as *mut _ as *mut c_void,
			);
		}
	}
}

unsafe extern "C" fn do_callback(data: *mut c_void) {
	unsafe {
		let mut data = Box::from_raw(data as *mut Callbacker);
		cpp_forward_push_cell(data.forward, 1); // saved -- TODO
		//println!("data.value: {:x}", data.value);
		cpp_forward_push_cell(data.forward, data.value);
		data.path.push('\0');
		cpp_forward_push_string(data.forward, data.path.as_ptr());
		cpp_forward_execute(data.forward, &mut 0);
	}
}

unsafe extern "C" fn log_error_frame_action(error: *mut c_void) {
	unsafe {
		let mut error = Box::from_raw(error as *mut String);
		error.push('\0');
		cpp_extension_log_error(error.as_ptr());
	}
}

fn log_error(error: String) {
	unsafe {
		cpp_add_frame_action(log_error_frame_action, Box::leak(Box::new(error)) as *mut _ as *mut c_void);
	}
}
