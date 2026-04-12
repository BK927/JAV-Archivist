use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::SpriteInfo;

/// Resolve the FFmpeg or FFprobe binary path.
/// Checks next to the executable first (production), then system PATH (development).
pub fn resolve_binary(name: &str) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    };

    // 1. Next to the executable (production bundle)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let path = dir.join(&exe_name);
            if path.exists() {
                return Some(path);
            }
        }
    }

    // 2. System PATH (development)
    let check = Command::new(name)
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
    if check.map(|s| s.success()).unwrap_or(false) {
        return Some(PathBuf::from(name));
    }

    None
}

/// Check if FFmpeg is available.
pub fn check(ffmpeg_path: &Option<PathBuf>) -> bool {
    ffmpeg_path.is_some()
}

/// Get video duration in seconds using ffprobe.
fn get_duration(ffprobe_path: &Path, file_path: &str) -> Option<f64> {
    let output = Command::new(ffprobe_path)
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            file_path,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse::<f64>().ok()
}

/// Extract a single frame as JPEG at the given timestamp.
fn extract_frame(ffmpeg_path: &Path, file_path: &str, timestamp: f64, output_path: &Path) -> bool {
    let ts = format!("{:.2}", timestamp);
    let status = Command::new(ffmpeg_path)
        .args([
            "-y",
            "-ss", &ts,
            "-i", file_path,
            "-frames:v", "1",
            "-q:v", "3",
        ])
        .arg(output_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    status.map(|s| s.success()).unwrap_or(false)
}

/// Check if a JPEG file is likely a black frame (< 3KB).
fn is_black_frame(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() < 3_000)
        .unwrap_or(true)
}

/// Extract a thumbnail for a video. Tries 10%, 25%, 50% of duration.
/// Returns the path to the generated thumbnail, or None on failure.
pub fn extract_thumbnail(
    ffmpeg_path: &Path,
    ffprobe_path: &Path,
    file_path: &str,
    video_id: &str,
    thumbnails_dir: &Path,
) -> Option<String> {
    let duration = get_duration(ffprobe_path, file_path)?;
    if duration <= 0.0 {
        return None;
    }

    let output_path = thumbnails_dir.join(format!("{video_id}_local.jpg"));
    let percentages = [0.10, 0.25, 0.50];

    for pct in percentages {
        let timestamp = duration * pct;
        if extract_frame(ffmpeg_path, file_path, timestamp, &output_path) && !is_black_frame(&output_path) {
            return Some(output_path.to_string_lossy().to_string());
        }
    }

    // All attempts were black frames — use the last one anyway
    if output_path.exists() {
        Some(output_path.to_string_lossy().to_string())
    } else {
        None
    }
}

/// Generate a sprite sheet for seek bar preview.
/// Returns SpriteInfo or None on failure.
pub fn generate_sprite_sheet(
    ffmpeg_path: &Path,
    ffprobe_path: &Path,
    file_path: &str,
    video_id: &str,
    part_index: u32,
    sprites_dir: &Path,
) -> Option<SpriteInfo> {
    let duration = get_duration(ffprobe_path, file_path)?;
    if duration <= 0.0 {
        return None;
    }

    let interval = (duration / 100.0).ceil().max(10.0) as u32;
    let total_frames = (duration / interval as f64).ceil() as u32;
    let columns: u32 = 10;
    let rows = (total_frames as f64 / columns as f64).ceil() as u32;

    let sprite_path = sprites_dir.join(format!("{video_id}_part{part_index}.jpg"));
    let meta_path = sprites_dir.join(format!("{video_id}_part{part_index}.json"));

    // Check cache
    if sprite_path.exists() && meta_path.exists() {
        let json = std::fs::read_to_string(&meta_path).ok()?;
        return serde_json::from_str::<SpriteInfo>(&json).ok();
    }

    let vf = format!("fps=1/{interval},scale=160:-1,tile={columns}x{rows}");
    let status = Command::new(ffmpeg_path)
        .args([
            "-y",
            "-i", file_path,
            "-vf", &vf,
            "-q:v", "5",
        ])
        .arg(&sprite_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    if !status.map(|s| s.success()).unwrap_or(false) || !sprite_path.exists() {
        return None;
    }

    // Read actual frame dimensions from the sprite image
    let (sprite_w, sprite_h) = image_dimensions(&sprite_path)?;
    let frame_w = sprite_w / columns;
    let frame_h = sprite_h / rows;

    let info = SpriteInfo {
        url: sprite_path.to_string_lossy().to_string(),
        width: frame_w,
        height: frame_h,
        columns,
        rows,
        interval,
        total_frames,
    };

    // Cache metadata
    if let Ok(json) = serde_json::to_string(&info) {
        let _ = std::fs::write(&meta_path, json);
    }

    Some(info)
}

/// Get image dimensions (width, height) by reading JPEG header.
pub fn image_dimensions(path: &Path) -> Option<(u32, u32)> {
    let data = std::fs::read(path).ok()?;
    let mut i = 0;
    while i + 1 < data.len() {
        if data[i] == 0xFF {
            let marker = data[i + 1];
            if marker == 0xC0 || marker == 0xC2 {
                if i + 8 < data.len() {
                    let height = ((data[i + 5] as u32) << 8) | (data[i + 6] as u32);
                    let width = ((data[i + 7] as u32) << 8) | (data[i + 8] as u32);
                    return Some((width, height));
                }
            }
            if marker != 0x00 && marker != 0xFF {
                if i + 3 < data.len() {
                    let len = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                    i += 2 + len;
                } else {
                    break;
                }
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Check if an image is low quality based on resolution and bits-per-pixel.
/// Returns true if width < 800px OR bpp < 0.1.
pub fn is_low_quality_image(path: &Path) -> bool {
    let (width, height) = match image_dimensions(path) {
        Some(dims) => dims,
        None => return true, // can't read → treat as low quality
    };

    if width < 800 {
        return true;
    }

    // Check bits per pixel (bpp = file_size_bytes / pixel_count)
    if let Ok(meta) = std::fs::metadata(path) {
        let pixels = (width as u64) * (height as u64);
        if pixels > 0 {
            let bpp = meta.len() as f64 / pixels as f64;
            if bpp < 0.1 {
                return true;
            }
        }
    }

    false
}

/// Extract N evenly-spaced frames from a video as JPEG sample images.
/// Returns the paths of successfully extracted frames.
pub fn extract_sample_images(
    ffmpeg_path: &Path,
    ffprobe_path: &Path,
    file_path: &str,
    video_id: &str,
    samples_dir: &Path,
    count: u32,
) -> Vec<String> {
    let duration = match get_duration(ffprobe_path, file_path) {
        Some(d) if d > 0.0 => d,
        _ => return Vec::new(),
    };

    let mut paths = Vec::new();
    for i in 0..count {
        // Evenly distribute: skip first and last 5% to avoid black frames
        let pct = 0.05 + (0.90 * (i as f64 + 0.5) / count as f64);
        let timestamp = duration * pct;
        let filename = format!("{}_sample_{:02}.jpg", video_id, i + 1);
        let output_path = samples_dir.join(&filename);

        if extract_frame(ffmpeg_path, file_path, timestamp, &output_path)
            && !is_black_frame(&output_path)
        {
            paths.push(output_path.to_string_lossy().to_string());
        }
    }

    paths
}
