// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2025 rtldg <rtldg@protonmail.com>

#![allow(non_snake_case)]
// TODO: Bleh, static muts...
#![allow(static_mut_refs)]

use std::ffi::c_char;
use std::ffi::c_void;
use std::io::BufWriter;
use std::io::Write;
use std::path::PathBuf;
use std::ptr::NonNull;
use std::str::FromStr;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::mpsc::channel;
use std::thread::JoinHandle;

use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use extshared::cpp_extension_log_error;
use extshared::ICellArray::ICellArray;
use extshared::cpp_add_frame_action;
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

const REPLAY_VERSION: &[u8] = b"9:{SHAVITREPLAYFORMAT}{FINAL}\n";
const FRAME_T_SIZE: usize = 10 * 4;

#[derive(Debug)]
struct Msg {
	forward:          NonNull<c_void>,
	value:            i32,
	replayFolderOrig: String,
	replayFolder:     PathBuf,
	map:              String,
	style:            i32,
	track:            i32,
	time:             f32,
	steamid:          i32,
	preframes:        i32,
	playerrecording:  *mut ICellArray,
	iSize:            i32,
	postframes:       i32,
	timestamp:        i32,
	fZoneOffset:      [f32; 2],
	saveCopy:         bool,
	saveWR:           bool,
	tickrate:         f32,
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
	replayFolderOrig: *const c_char,
	replayFolder: *const c_char,
	map: *const c_char,
	style: i32,
	track: i32,
	time: f32,
	steamid: i32,
	preframes: i32,
	playerrecording: *mut ICellArray,
	iSize: i32,
	postframes: i32,
	timestamp: i32,
	fZoneOffset: *const f32,
	saveCopy: bool,
	saveWR: bool,
	tickrate: f32,
) {
	let replayFolderOrig = strxx(replayFolderOrig, false, 0).unwrap().to_string();
	let replayFolder = PathBuf::from_str(strxx(replayFolder, false, 0).unwrap()).unwrap();
	let map = strxx(map, false, 0).unwrap().to_string();
	//println!("hello from poster!");
	unsafe {
		if let Some(sender) = &SENDER {
			sender
				.send(Msg {
					forward,
					value,
					replayFolderOrig,
					replayFolder,
					map,
					style,
					track,
					time,
					steamid,
					preframes,
					playerrecording,
					iSize,
					postframes,
					timestamp,
					fZoneOffset: [fZoneOffset.read(), fZoneOffset.add(1).read()],
					saveCopy,
					saveWR,
					tickrate,
				})
				.unwrap();
			//println!("posted!");
		}
	}
	//
}

fn write_replay_header(file: &mut BufWriter<std::fs::File>, msg: &Msg) {
	let _ = file.write(REPLAY_VERSION);

	let _ = write!(file, "{}\0", msg.map);
	let _ = file.write_i8(msg.style as i8);
	let _ = file.write_i8(msg.track as i8);
	let _ = file.write_i32::<LittleEndian>(msg.preframes);

	let _ = file.write_i32::<LittleEndian>(msg.iSize - msg.preframes - msg.postframes);
	let _ = file.write_f32::<LittleEndian>(msg.time);
	let _ = file.write_i32::<LittleEndian>(msg.steamid);

	let _ = file.write_i32::<LittleEndian>(msg.postframes);
	let _ = file.write_f32::<LittleEndian>(msg.tickrate);

	let _ = file.write_f32::<LittleEndian>(msg.fZoneOffset[0]);
	let _ = file.write_f32::<LittleEndian>(msg.fZoneOffset[1]);
}

fn replay_thread(recv: Receiver<Msg>) {
	while let Ok(msg) = recv.recv() {
		//println!("received {msg:?}");
		let mut callbackPath = String::new();

		let mut fCopy = if msg.saveCopy {
			callbackPath = format!("copy/{}_{}_{}.replay", msg.timestamp, msg.steamid, msg.map);
			if let Ok(f) = std::fs::File::create(msg.replayFolder.join(&callbackPath)).map(std::io::BufWriter::new) {
				Some(f)
			} else {
				log_error(format!("Failed to open 'copy' replay file for writing. ('{callbackPath}')"));
				None
			}
		} else {
			None
		};
		let mut fWR = if msg.saveWR {
			callbackPath = if msg.track > 0 {
				format!("{}/{}{}.replay", msg.style, msg.map, msg.track)
			} else {
				format!("{}/{}.replay", msg.style, msg.map)
			};
			if let Ok(f) = std::fs::File::create(msg.replayFolder.join(&callbackPath)).map(std::io::BufWriter::new) {
				Some(f)
			} else {
				log_error(format!("Failed to open WR replay file for writing. ('{callbackPath}')"));
				None
			}
		} else {
			None
		};

		if fCopy.is_none() && fWR.is_none() {
			log_error(format!(
				"Failed to open WR and 'copy' replay files for writing [U:1:{}] style={} track={} map={}",
				msg.steamid, msg.style, msg.track, msg.map
			));
			continue;
		}

		if let Some(f) = &mut fWR {
			write_replay_header(f, &msg);
		}

		if let Some(f) = &mut fCopy {
			write_replay_header(f, &msg);
		}

		let cellarray = unsafe { &mut *msg.playerrecording };
		let frames = unsafe { std::slice::from_raw_parts(cellarray.data as *const u8, FRAME_T_SIZE * msg.iSize as usize) };

		if let Some(f) = &mut fWR {
			let _ = f.write(frames);
		}
		if let Some(f) = &mut fCopy {
			let _ = f.write(frames);
		}

		if let Some(mut f) = fCopy {
			let _ = f.flush();
		}
		if let Some(mut f) = fWR {
			let _ = f.flush();
		}

		unsafe {
			cpp_add_frame_action(
				do_callback,
				Box::leak(Box::new(Callbacker {
					forward: msg.forward,
					//saved: ?
					value:   msg.value,
					// note the \0
					path:    format!("{}/{}\0", msg.replayFolderOrig, callbackPath),
				})) as *mut _ as *mut c_void,
			);
		}
	}
}

unsafe extern "C" fn do_callback(data: *mut c_void) {
	unsafe {
		let data = Box::from_raw(data as *mut Callbacker);
		cpp_forward_push_cell(data.forward, 1); // saved -- TODO
		//println!("data.value: {:x}", data.value);
		cpp_forward_push_cell(data.forward, data.value);
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
