use std::path::Path;
use crate::media;
use crate::models::SpriteInfo;

/// Check if a JPEG file is likely a black frame (< 3KB).
fn is_black_frame(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() < 3_000)
        .unwrap_or(true)
}

/// Extract a thumbnail for a video. Tries 10%, 25%, 50% of duration.
/// Returns the path to the generated thumbnail, or None on failure.
pub fn extract_thumbnail(
    file_path: &str,
    video_id: &str,
    thumbnails_dir: &Path,
) -> Option<String> {
    let duration = media::get_duration(file_path)?;
    if duration <= 0.0 {
        return None;
    }

    let output_path = thumbnails_dir.join(format!("{video_id}_local.jpg"));
    let percentages = [0.10, 0.25, 0.50];

    for pct in percentages {
        let timestamp = duration * pct;
        if media::extract_frame(file_path, timestamp, &output_path) && !is_black_frame(&output_path) {
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
    file_path: &str,
    video_id: &str,
    part_index: u32,
    sprites_dir: &Path,
) -> Option<SpriteInfo> {
    let duration = media::get_duration(file_path)?;
    if duration <= 0.0 {
        return None;
    }

    let interval = (duration / 100.0).ceil().max(10.0) as u32;
    let total_frames = (duration / interval as f64).ceil() as u32;
    let columns: u32 = 10;

    let sprite_path = sprites_dir.join(format!("{video_id}_part{part_index}.jpg"));
    let meta_path = sprites_dir.join(format!("{video_id}_part{part_index}.json"));

    // Check cache
    if sprite_path.exists() && meta_path.exists() {
        let json = std::fs::read_to_string(&meta_path).ok()?;
        return serde_json::from_str::<SpriteInfo>(&json).ok();
    }

    // Extract individual frames to a temp dir
    let temp_dir = std::env::temp_dir().join(format!("sprites_{video_id}_{part_index}"));
    let _ = std::fs::create_dir_all(&temp_dir);

    let mut frame_paths: Vec<std::path::PathBuf> = Vec::new();
    for i in 0..total_frames {
        let timestamp = i as f64 * interval as f64;
        let frame_path = temp_dir.join(format!("frame_{:04}.jpg", i));
        if media::extract_frame(file_path, timestamp, &frame_path) {
            frame_paths.push(frame_path);
        }
    }

    if frame_paths.is_empty() {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return None;
    }

    // Tile frames into a sprite sheet
    let frame_w: u32 = 160;
    let first_img = image::open(&frame_paths[0]).ok()?;
    let aspect = first_img.height() as f64 / first_img.width() as f64;
    let frame_h = (frame_w as f64 * aspect).round() as u32;

    let actual_rows = (frame_paths.len() as f64 / columns as f64).ceil() as u32;
    let sprite_w = frame_w * columns;
    let sprite_h = frame_h * actual_rows;
    let mut sprite = image::RgbImage::new(sprite_w, sprite_h);

    for (idx, path) in frame_paths.iter().enumerate() {
        if let Ok(img) = image::open(path) {
            let resized = img.resize_exact(frame_w, frame_h, image::imageops::FilterType::Triangle);
            let col = (idx as u32) % columns;
            let row = (idx as u32) / columns;
            image::imageops::overlay(
                &mut sprite,
                &resized.to_rgb8(),
                (col * frame_w) as i64,
                (row * frame_h) as i64,
            );
        }
    }

    let dyn_img = image::DynamicImage::ImageRgb8(sprite);
    dyn_img.save_with_format(&sprite_path, image::ImageFormat::Jpeg).ok()?;

    // Clean up temp dir
    let _ = std::fs::remove_dir_all(&temp_dir);

    if !sprite_path.exists() {
        return None;
    }

    let info = SpriteInfo {
        url: sprite_path.to_string_lossy().to_string(),
        width: frame_w,
        height: frame_h,
        columns,
        rows: actual_rows,
        interval,
        total_frames: frame_paths.len() as u32,
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
    file_path: &str,
    video_id: &str,
    samples_dir: &Path,
    count: u32,
) -> Vec<String> {
    let duration = match media::get_duration(file_path) {
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

        if media::extract_frame(file_path, timestamp, &output_path)
            && !is_black_frame(&output_path)
        {
            paths.push(output_path.to_string_lossy().to_string());
        }
    }

    paths
}
