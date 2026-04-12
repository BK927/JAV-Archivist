//! Video frame extraction and duration via Windows Media Foundation.

use std::path::Path;
use windows::core::PCWSTR;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::*;

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// RAII guard: initializes COM (MTA) + MF on creation, shuts down on drop.
struct MfGuard;

impl MfGuard {
    fn init() -> Option<Self> {
        unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            if hr.is_err() {
                return None;
            }
            if MFStartup(MF_VERSION, MFSTARTUP_NOSOCKET).is_err() {
                CoUninitialize();
                return None;
            }
            Some(Self)
        }
    }
}

impl Drop for MfGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = MFShutdown();
            CoUninitialize();
        }
    }
}

fn create_reader(file_path: &str) -> Option<IMFSourceReader> {
    let wide = to_wide(file_path);
    unsafe {
        let mut attrs: Option<IMFAttributes> = None;
        MFCreateAttributes(&mut attrs, 1).ok()?;
        let attrs = attrs?;
        MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), &attrs).ok()
    }
}

/// Get video duration in seconds.
pub fn get_duration(file_path: &str) -> Option<f64> {
    let _guard = MfGuard::init()?;
    let reader = create_reader(file_path)?;
    unsafe {
        // MF_SOURCE_READER_MEDIASOURCE.0 is i32, cast to u32 for the API
        let var: PROPVARIANT = reader
            .GetPresentationAttribute(
                MF_SOURCE_READER_MEDIASOURCE.0 as u32,
                &MF_PD_DURATION,
            )
            .ok()?;
        // Duration is in 100-nanosecond units, stored as i64
        let hns: i64 = i64::try_from(&var).ok()?;
        Some(hns as f64 / 10_000_000.0)
    }
}

/// Seek to `timestamp` seconds, decode one frame, save as JPEG.
pub fn extract_frame(file_path: &str, timestamp: f64, output_path: &Path) -> bool {
    let inner = || -> Option<()> {
        let _guard = MfGuard::init()?;
        let reader = create_reader(file_path)?;

        unsafe {
            let stream_index = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;

            // Request RGB32 output
            let media_type: IMFMediaType = MFCreateMediaType().ok()?;
            media_type
                .SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)
                .ok()?;
            media_type
                .SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)
                .ok()?;
            reader
                .SetCurrentMediaType(stream_index, None, &media_type)
                .ok()?;

            // Seek to timestamp (100-nanosecond units)
            let hns = (timestamp * 10_000_000.0) as i64;
            let pos = PROPVARIANT::from(hns);
            reader
                .SetCurrentPosition(
                    &windows::core::GUID::zeroed(),
                    &pos,
                )
                .ok()?;

            // Read a sample
            let mut flags: u32 = 0;
            let mut _timestamp_out: i64 = 0;
            let mut sample: Option<IMFSample> = None;
            reader
                .ReadSample(
                    stream_index,
                    0,
                    None,
                    Some(&mut flags),
                    Some(&mut _timestamp_out),
                    Some(&mut sample),
                )
                .ok()?;
            let sample = sample?;
            let buffer: IMFMediaBuffer = sample.ConvertToContiguousBuffer().ok()?;

            // Lock buffer and read pixels
            let mut data_ptr: *mut u8 = std::ptr::null_mut();
            let mut max_len: u32 = 0;
            let mut data_len: u32 = 0;
            buffer
                .Lock(&mut data_ptr, Some(&mut max_len), Some(&mut data_len))
                .ok()?;
            let pixel_data =
                std::slice::from_raw_parts(data_ptr, data_len as usize);

            // Get actual frame dimensions from current output type
            let actual_type: IMFMediaType = reader
                .GetCurrentMediaType(stream_index)
                .ok()?;
            // MF_MT_FRAME_SIZE packs width(high32)|height(low32) into a u64
            let packed: u64 = actual_type.GetUINT64(&MF_MT_FRAME_SIZE).ok()?;
            let width = (packed >> 32) as u32;
            let height = (packed & 0xFFFFFFFF) as u32;

            // Convert BGRA bottom-up to RGB top-down
            // Query actual stride (may differ from width*4 due to alignment padding)
            let stride = actual_type.GetUINT32(&MF_MT_DEFAULT_STRIDE)
                .map(|s| s as usize)
                .unwrap_or((width * 4) as usize);
            let mut rgb = Vec::with_capacity((width * height * 3) as usize);
            for y in (0..height as usize).rev() {
                for x in 0..width as usize {
                    let off = y * stride + x * 4;
                    if off + 2 < pixel_data.len() {
                        rgb.push(pixel_data[off + 2]); // R
                        rgb.push(pixel_data[off + 1]); // G
                        rgb.push(pixel_data[off]);      // B
                    }
                }
            }

            let _ = buffer.Unlock();

            // Save JPEG via image crate
            let img = image::RgbImage::from_raw(width, height, rgb)?;
            image::DynamicImage::ImageRgb8(img)
                .save_with_format(output_path, image::ImageFormat::Jpeg)
                .ok()?;
        }
        Some(())
    };
    inner().is_some()
}
