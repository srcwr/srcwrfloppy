// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2025 rtldg <rtldg@protonmail.com>

#![allow(non_snake_case)]
// TODO: Bleh, static muts...
#![allow(static_mut_refs)]

use std::ffi::c_void;
use std::io::Write;
use std::ptr::NonNull;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;
use std::thread::JoinHandle;

use extshared::ICellArray::ICellArray;
use extshared::ICellArray::ICellArray_at;
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
	forward:         NonNull<c_void>,
	value:           i32,
	friendly_paths:  Vec<String>,
	header:          Vec<u8>,
	playerrecording: *const ICellArray,
	totalframes:     usize,
}
unsafe impl Send for Msg {} // so we can store the pointers...

struct Callbacker {
	forward: NonNull<c_void>,
	saved:   bool,
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
	pathsarray: &ICellArray,
	header: *const u8,
	headersize: usize,
	playerrecording: *const ICellArray,
	totalframes: usize,
) {
	let mut pathsvec = vec![];

	unsafe {
		let len = pathsarray.size;
		for i in 0..len {
			pathsvec.push(strxx(ICellArray_at(pathsarray, i), false, 0).unwrap_or_default().to_string());
		}
	}

	// the wr replay path is pushed to the end of the arraylist, so I want to make that be the first one that is written....
	pathsvec.reverse();

	let header = unsafe { std::slice::from_raw_parts(header, headersize).to_vec() };

	//println!("hello from poster!");
	unsafe {
		if let Some(sender) = &SENDER {
			sender
				.send(Msg {
					forward,
					value,
					friendly_paths: pathsvec,
					header,
					playerrecording,
					totalframes,
				})
				.unwrap();
			//println!("posted!");
		}
	}
}

fn replay_thread(recv: Receiver<Msg>) {
	while let Ok(msg) = recv.recv() {
		//println!("received {msg:?}");

		let mut writers = vec![];

		// TODO: write to data/replaybot/tmp/ & rename for atomic overwrites?

		for path in &msg.friendly_paths {
			let path = extshared::build_path(path.as_ptr(), extshared::PathType::Path_Game);
			if let Ok(f) = std::fs::File::create(&path).map(std::io::BufWriter::new) {
				writers.push(f);
			} else {
				log_error(format!("Failed to open '{path}' replay file for writing."));
			}
		}

		let saved = !writers.is_empty();

		if saved {
			let cellarray = unsafe { &*msg.playerrecording };
			let frames = unsafe {
				std::slice::from_raw_parts(
					cellarray.data as *const u8,
					cellarray.blocksize * size_of::<i32>() * msg.totalframes,
				)
			};

			for f in writers.iter_mut() {
				let _ = f.write_all(&msg.header);
				let _ = f.write_all(frames);
			}

			for f in writers {
				let _ = f.into_inner(); // consume & flush & drop
			}
		}

		unsafe {
			cpp_add_frame_action(
				do_callback,
				Box::leak(Box::new(Callbacker {
					forward: msg.forward,
					saved,
					value: msg.value,
					// TODO: not necessarily a path that was actually saved to but whatever for now...
					path: msg.friendly_paths[0].clone(),
				})) as *mut _ as *mut c_void,
			);
		}
	}
}

unsafe extern "C" fn do_callback(data: *mut c_void) {
	unsafe {
		let mut data = Box::from_raw(data as *mut Callbacker);
		cpp_forward_push_cell(data.forward, data.saved as i32);
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
