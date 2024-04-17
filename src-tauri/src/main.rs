#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::HashMap, sync::atomic::{self, AtomicU64}};

use base64::Engine;
use crabgrab::{capturable_content::{CapturableContent, CapturableContentFilter, CapturableWindow, CapturableWindowFilter}, capture_stream::{CaptureConfig, CapturePixelFormat, CaptureStream, StreamEvent}, feature::{bitmap::{FrameBitmap, FrameBitmapBgraUnorm8x4, VideoFrameBitmap}, screenshot::take_screenshot}};
use lazy_static::lazy_static;
use parking_lot::Mutex;
use tauri::{AppHandle, Manager};
use tokio::time::{timeout, Duration};
use serde::Serialize;

lazy_static! {
	static ref WINDOW_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
	static ref WINDOW_MAP: Mutex<HashMap<CapturableWindow, u64>> = Mutex::new(HashMap::new());
	static ref ACTIVE_STREAM: Mutex<Option<CaptureStream>> = Mutex::new(None);
}

const THUMBNAIL_MAX: usize = 200;

fn make_base64_png_from_bitmap(bitmap: FrameBitmapBgraUnorm8x4) -> String {
	let size = if bitmap.width > bitmap.height {
		let w = bitmap.width.min(THUMBNAIL_MAX);
		let h = ((w as f64 / bitmap.width as f64) * bitmap.height as f64).ceil() as usize;
		(w, h)
	} else {
		let h = bitmap.height.min(THUMBNAIL_MAX);
		let w = ((h as f64 / bitmap.height as f64) * bitmap.width as f64).ceil() as usize;
		(w, h)
	};
	let mut write_vec = vec![0u8; 0];
	{
		let mut encoder = png::Encoder::new(&mut write_vec, size.0 as u32, size.1 as u32);
		encoder.set_color(png::ColorType::Rgba);
		encoder.set_depth(png::BitDepth::Eight);
		encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
		encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));     // 1.0 / 2.2, unscaled, but rounded
		let source_chromaticities = png::SourceChromaticities::new(     // Using unscaled instantiation here
			(0.31270, 0.32900),
			(0.64000, 0.33000),
			(0.30000, 0.60000),
			(0.15000, 0.06000)
		);
		encoder.set_source_chromaticities(source_chromaticities);
		let mut writer = encoder.write_header().unwrap();
		let mut image_data = vec![0u8; size.0 * size.1 * 4];
		for y in 0..size.1 {
			let sample_y = (bitmap.height * y) / size.1;
			for x in 0..size.0 {
				let sample_x = (bitmap.width * x) / size.0;
				let [b, g, r, a] = bitmap.data[sample_x + sample_y * bitmap.width];
				image_data[(x + y * size.0) * 4 + 0] = r;
				image_data[(x + y * size.0) * 4 + 1] = g;
				image_data[(x + y * size.0) * 4 + 2] = b;
				image_data[(x + y * size.0) * 4 + 3] = a;
			}
		}
		writer.write_image_data(&image_data).unwrap();
	}
	base64::prelude::BASE64_STANDARD.encode(write_vec)
}

#[derive(Serialize)]
#[derive(Clone)]
struct Item {
    id: String,
    thumbnail: String,
    title: String,
	req: i32,
}

#[tauri::command]
async fn get_windows(app: AppHandle, req: i32) -> String {
	let filter = CapturableContentFilter {
		windows: Some(CapturableWindowFilter {
			desktop_windows: false,
			onscreen_only: true
		}),
		displays: false,
	};
	let content = CapturableContent::new(filter).await.unwrap();
	let window_list: Vec<_> = {
		let mut window_map = WINDOW_MAP.lock();
		for window in content.windows() {
			if !window_map.contains_key(&window) {
				let id = WINDOW_ID_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);
				window_map.insert(window, id);
			}
		};
		window_map.iter().map(|(window, id)| (window.clone(), *id)).collect()
	};
	let mut window_list_json = "[".to_string();
	let mut is_first = true;
	for (window, id) in window_list.iter() {
		let screenshot_config = CaptureConfig::with_window(window.clone(), CapturePixelFormat::Bgra8888).unwrap();
		let screenshot_task = take_screenshot(screenshot_config);
		let screenshot_result = timeout(Duration::from_millis(250), screenshot_task).await;

		let screenshot = match screenshot_result {
			Ok(output) => output,
			_ => continue
		};

		if let Ok(Ok(FrameBitmap::BgraUnorm8x4(image_bitmap_bgra8888))) = screenshot.map(|frame| frame.get_bitmap()) {
			let image_base64 = make_base64_png_from_bitmap(image_bitmap_bgra8888);
			if !is_first {
				window_list_json += ",\n";
			}
			is_first = false;
			let item = Item {
				id: format!("{}", id),
				thumbnail: image_base64,
				title: window.title(),
				req
			};
			let item_json = serde_json::to_string(&item).unwrap();
			app.emit_all("window_found", item).unwrap();
			window_list_json += &item_json;
		}
	}
	window_list_json += "]";
	window_list_json
}

#[tauri::command]
fn begin_capture(app_handle: tauri::AppHandle, window_id: u64) -> Result<(), String> {
	let mut active_stream = ACTIVE_STREAM.lock();
	let window_map = WINDOW_MAP.lock();
	for (window, id) in window_map.iter() {
		if *id == window_id {
			let config = CaptureConfig::with_window(window.clone(), CapturePixelFormat::Bgra8888)
				.map_err(|error| error.to_string())?;
			let stream = CaptureStream::new(config, |event_result| {
				match event_result {
					Ok(StreamEvent::Video(frame)) => println!("video frame: {}", frame.frame_id()),
					_ => {}
				}
			}).map_err(|error| error.to_string())?;
			*active_stream = Some(stream);
			let app_window = app_handle.get_window("main").unwrap();
			app_window.eval("window.location.replace('recording.html')").unwrap();
			return Ok(());
		}
	}
	Err("Unknown window".to_string())
}

#[tauri::command]
fn end_capture(app_handle: tauri::AppHandle) -> Result<(), String> {
	{
		let mut active_stream = ACTIVE_STREAM.lock();
		if let Some(mut stream) = active_stream.take() {
			// todo... finish recording
			let _ = stream.stop();
		}
	}
	let app_window = app_handle.get_window("main").unwrap();
	app_window.eval("window.location.replace('main.html')").unwrap();
	Ok(())
}

fn main() {
	tauri::Builder::default()
		.invoke_handler(tauri::generate_handler![
			get_windows,
			begin_capture,
			end_capture
		])
		.setup(|app| {
			let main_window = app.get_window("main").expect("Expected app to have main window");
			main_window.open_devtools();
			Ok(())
		})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
