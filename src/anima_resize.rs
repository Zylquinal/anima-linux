use std::path::PathBuf;
use sha2::{Sha256, Digest};
use std::fs::File;
use image::codecs::gif::{GifDecoder, GifEncoder, Repeat};
use image::{AnimationDecoder, Frame, imageops::FilterType, DynamicImage};

/// Convert any supported file into an animated GIF stored at `dest`.
///
/// | Input              | Strategy                                          |
/// |--------------------|---------------------------------------------------|
/// | `.gif`             | Copy verbatim                                     |
/// | `.webp`            | Animated > re-encode frames; static > 1-frame GIF |
/// | image (png/jpg/…)  | Load via `image` crate > 1-frame GIF              |
/// | video (mp4/mkv/…)  | `ffmpeg` subprocess > GIF                         |
pub fn import_as_gif(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let ext = src.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "gif" => std::fs::copy(src, dest)
            .map(|_| ())
            .map_err(|e| format!("Copy failed: {e}")),

        "webp" => import_webp_as_gif(src, dest),

        "mp4" | "mkv" | "webm" | "avi" | "mov" | "flv" | "m4v" | "wmv" | "ts" | "ogv" => {
            import_video_ffmpeg(src, dest)
        }

        _ => import_static_image_as_gif(src, dest),
    }
}

fn import_webp_as_gif(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    use image::codecs::webp::WebPDecoder;

    // Try animated-WebP path first.
    let try_animated = (|| -> Result<Vec<image::Frame>, String> {
        let file = File::open(src).map_err(|e| format!("Open failed: {e}"))?;
        let decoder = WebPDecoder::new(std::io::BufReader::new(file))
            .map_err(|e| format!("WebP decode error: {e}"))?;
        decoder.into_frames()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Frame read error: {e}"))
    })();

    match try_animated {
        Ok(frames) if frames.len() > 1 => {
            // Animated WebP
            let mut out = File::create(dest).map_err(|e| format!("Create failed: {e}"))?;
            let mut encoder = GifEncoder::new(&mut out);
            encoder.set_repeat(Repeat::Infinite)
                .map_err(|e| format!("Set repeat failed: {e}"))?;
            for frame in frames {
                encoder.encode_frame(frame)
                    .map_err(|e| format!("Encode error: {e}"))?;
            }
            Ok(())
        }
        // Single frame or decode error
        _ => import_static_image_as_gif(src, dest),
    }
}


fn import_static_image_as_gif(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let img = image::open(src).map_err(|e| format!("Failed to open image: {e}"))?;
    let rgba = img.to_rgba8();
    let mut out = File::create(dest).map_err(|e| format!("Create failed: {e}"))?;
    let mut encoder = GifEncoder::new(&mut out);
    encoder.set_repeat(Repeat::Infinite)
        .map_err(|e| format!("Set repeat failed: {e}"))?;
    encoder.encode_frame(Frame::new(rgba))
        .map_err(|e| format!("Encode error: {e}"))
}

fn import_video_ffmpeg(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-i", src.to_str().unwrap_or(""),
            "-vf", "fps=15,scale='min(480,iw)':-1:flags=lanczos",
            "-loop", "0",
            "-y",
            dest.to_str().unwrap_or(""),
        ])
        .status()
        .map_err(|e| format!("ffmpeg launch failed: {e}\nMake sure ffmpeg is installed."))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("ffmpeg exited with code {status}.\nCheck that the file is a valid video."))
    }
}



pub fn get_processed_gif_path(
    orig_path: &str,
    scale: f64,
    mirror: bool,
    flip_v: bool,
    roll: f64,
    pitch: f64,
    yaw: f64,
    temp: f64,
    contrast: f64,
    brightness: f64,
    saturation: f64,
    hue: f64,
) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(orig_path.as_bytes());
    hasher.update(&scale.to_le_bytes());
    hasher.update(&[if mirror { 1 } else { 0 }]);
    hasher.update(&[if flip_v { 1 } else { 0 }]);
    hasher.update(&roll.to_le_bytes());
    hasher.update(&pitch.to_le_bytes());
    hasher.update(&yaw.to_le_bytes());
    hasher.update(&temp.to_le_bytes());
    hasher.update(&contrast.to_le_bytes());
    hasher.update(&brightness.to_le_bytes());
    hasher.update(&saturation.to_le_bytes());
    hasher.update(&hue.to_le_bytes());
    let hash = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in hash {
        use std::fmt::Write;
        write!(&mut hex, "{:02x}", b).unwrap();
    }
    let mut name = String::from("cached_");
    name.push_str(&hex);
    name.push_str(".gif");
    crate::db::Db::app_dir().join(name)
}

pub fn clear_cache() -> std::io::Result<()> {
    let dir = crate::db::Db::app_dir();
    if dir.exists() && dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("cached_") && name.ends_with(".gif") {
                        let _ = std::fs::remove_file(path);
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn process_gif_in_memory(
    orig_path: &str,
    scale: f64,
    mirror: bool,
    flip_v: bool,
    roll: f64,
    pitch: f64,
    yaw: f64,
    temp: f64,
    contrast: f64,
    brightness: f64,
    saturation: f64,
    hue: f64,
) -> Vec<u8> {
    let mut out_data = Vec::new();
    process_gif_frames(orig_path, scale, mirror, flip_v, roll, pitch, yaw, temp, contrast, brightness, saturation, hue, &mut out_data);
    out_data
}

pub fn ensure_processed_gif(
    orig_path: &str,
    scale: f64,
    mirror: bool,
    flip_v: bool,
    roll: f64,
    pitch: f64,
    yaw: f64,
    temp: f64,
    contrast: f64,
    brightness: f64,
    saturation: f64,
    hue: f64,
) -> PathBuf {
    let out_path = get_processed_gif_path(orig_path, scale, mirror, flip_v, roll, pitch, yaw, temp, contrast, brightness, saturation, hue);
    if out_path.exists() {
        return out_path;
    }

    if (scale - 1.0).abs() < 0.01 && !mirror && !flip_v && roll.abs() < 0.01 && pitch.abs() < 0.01
        && yaw.abs() < 0.01 && temp.abs() < 0.01 && contrast.abs() < 0.01 && brightness.abs() < 0.01
        && saturation.abs() < 0.01 && hue.abs() < 0.01 {
        return PathBuf::from(orig_path);
    }

    let mut out_file = File::create(&out_path).unwrap();
    process_gif_frames(orig_path, scale, mirror, flip_v, roll, pitch, yaw, temp, contrast, brightness, saturation, hue, &mut out_file);
    out_path
}

fn process_gif_frames<W: std::io::Write>(
    orig_path: &str,
    scale: f64,
    mirror: bool,
    flip_v: bool,
    roll: f64,
    pitch: f64,
    yaw: f64,
    temp: f64,
    contrast: f64,
    brightness: f64,
    saturation: f64,
    hue: f64,
    writer: &mut W,
) {
    // Detect format so we can handle non-GIF sources (e.g. .webp paths that
    // were imported before the normalise-to-GIF logic was added, or any other
    // image format the user managed to get into the DB).
    let ext = std::path::Path::new(orig_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let frames: Vec<Frame> = if ext == "gif" {
        let file = File::open(orig_path).unwrap();
        let buf_reader = std::io::BufReader::new(file);
        match GifDecoder::new(buf_reader) {
            Ok(decoder) => decoder.into_frames()
                .collect::<Result<Vec<_>, _>>()
                .unwrap_or_default(),
            Err(e) => {
                eprintln!("GIF decode failed for {orig_path}: {e}");
                vec![]
            }
        }
    } else {
        // Static / other animated image — load first frame via image crate.
        match image::open(orig_path) {
            Ok(img) => {
                let delay = image::Delay::from_numer_denom_ms(100, 1);
                vec![Frame::from_parts(img.to_rgba8(), 0, 0, delay)]
            }
            Err(e) => {
                eprintln!("Image decode failed for {orig_path}: {e}");
                vec![]
            }
        }
    };

    if frames.is_empty() {
        eprintln!("No frames decoded from {orig_path}, skipping encode.");
        return;
    }

    let mut encoder = GifEncoder::new(writer);
    encoder.set_repeat(Repeat::Infinite).ok();


    for frame in frames {
        let delay = frame.delay();
        let img = frame.into_buffer();
        let mut dyn_img = DynamicImage::ImageRgba8(img);

        // Resize
        if (scale - 1.0).abs() > 0.01 {
            let nwidth = (dyn_img.width() as f64 * scale) as u32;
            let nheight = (dyn_img.height() as f64 * scale) as u32;
            dyn_img = dyn_img.resize_exact(nwidth, nheight, FilterType::Nearest);
        }

        // Mirror & Flip
        if mirror {
            dyn_img = dyn_img.fliph();
        }
        if flip_v {
            dyn_img = dyn_img.flipv();
        }

        // We model the image as a flat plane in 3D space centred on the origin.
        // A virtual camera sits at (0, 0, -f). For every OUTPUT pixel (dx, dy)
        // we:
        //   1. Unproject it through the camera into a 3D ray direction.
        //   2. Apply the INVERSE rotation (transpose of Rz·Rx·Ry) to that ray.
        //   3. Intersect the inverse-rotated ray with the z=0 plane.
        //   4. That intersection is the source texel — bilinear sample it.
        if roll.abs() > 0.01 || pitch.abs() > 0.01 || yaw.abs() > 0.01 {
            let (w, h) = (dyn_img.width(), dyn_img.height());
            let mut new_img = image::RgbaImage::new(w, h);
            let img_rgba = dyn_img.to_rgba8();

            let cx = w as f32 / 2.0;
            let cy = h as f32 / 2.0;
            // Focal length: larger = less perspective distortion
            let f = w.max(h) as f32 * 1.5;

            // Build forward rotation matrix R = Rz(roll) * Rx(pitch) * Ry(yaw)
            // then invert by transposing (orthogonal matrix).
            let (sr, cr) = (roll.to_radians() as f32).sin_cos();
            let (sp, cp) = (pitch.to_radians() as f32).sin_cos();
            let (sy, cy_) = (yaw.to_radians() as f32).sin_cos();

            // R = Rz * Rx * Ry  (column-major, R[col][row])
            // Ry:  [cy_,0,sy; 0,1,0; -sy,0,cy_]
            // Rx:  [1,0,0; 0,cp,-sp; 0,sp,cp]
            // Rz:  [cr,-sr,0; sr,cr,0; 0,0,1]
            // Combined (row-major indexing r[row][col]):
            let r = [
                [cr*cy_ + sr*sp*sy,  -sr*cp,  cr*sy - sr*sp*cy_],
                [sr*cy_ - cr*sp*sy,   cr*cp,  sr*sy + cr*sp*cy_],
                [-cp*sy,              sp,     cp*cy_           ],
            ];
            // Inverse = transpose
            let ri = [
                [r[0][0], r[1][0], r[2][0]],
                [r[0][1], r[1][1], r[2][1]],
                [r[0][2], r[1][2], r[2][2]],
            ];

            for dy in 0..h {
                for dx in 0..w {
                    // Unproject output pixel to a 3D ray in camera space
                    let ox = dx as f32 - cx;
                    let oy = dy as f32 - cy;
                    // Ray direction in camera space (camera at z = -f looking +z)
                    let rd = [ox, oy, f];

                    // Rotate ray by inverse rotation
                    let rx = ri[0][0]*rd[0] + ri[0][1]*rd[1] + ri[0][2]*rd[2];
                    let ry = ri[1][0]*rd[0] + ri[1][1]*rd[1] + ri[1][2]*rd[2];
                    let rz = ri[2][0]*rd[0] + ri[2][1]*rd[1] + ri[2][2]*rd[2];

                    // Intersect with z=0 plane: t = -camera_z / rz = f / rz
                    // Camera origin in object space is inv_R * (0,0,-f)
                    let cam_ox = ri[0][2] * (-f);
                    let cam_oy = ri[1][2] * (-f);
                    let cam_oz = ri[2][2] * (-f);

                    if rz.abs() < 1e-5 { continue; }
                    let t = -cam_oz / rz;
                    if t < 0.0 { continue; }

                    let src_x = cam_ox + t * rx + cx;
                    let src_y = cam_oy + t * ry + cy;

                    if src_x >= 0.0 && src_x < w as f32 - 1.0
                       && src_y >= 0.0 && src_y < h as f32 - 1.0
                    {
                        let fx = src_x.floor() as u32;
                        let fy = src_y.floor() as u32;
                        let tx = src_x - fx as f32;
                        let ty = src_y - fy as f32;

                        let px00 = img_rgba.get_pixel(fx, fy);
                        let px10 = img_rgba.get_pixel(fx + 1, fy);
                        let px01 = img_rgba.get_pixel(fx, fy + 1);
                        let px11 = img_rgba.get_pixel(fx + 1, fy + 1);

                        let mut out_p = [0u8; 4];
                        for c in 0..4 {
                            out_p[c] = (px00[c] as f32 * (1.0-tx) * (1.0-ty)
                                      + px10[c] as f32 * tx        * (1.0-ty)
                                      + px01[c] as f32 * (1.0-tx) * ty
                                      + px11[c] as f32 * tx        * ty) as u8;
                        }
                        new_img.put_pixel(dx, dy, image::Rgba(out_p));
                    }
                }
            }
            dyn_img = DynamicImage::ImageRgba8(new_img);
        }

        // Brightness
        if brightness.abs() > 0.01 {
            dyn_img = dyn_img.brighten(brightness as i32);
        }

        // Contrast
        if contrast.abs() > 0.01 {
            dyn_img = dyn_img.adjust_contrast(contrast as f32);
        }

        // Hue
        if hue.abs() > 0.01 {
            dyn_img = dyn_img.huerotate(hue as i32);
        }

        // Saturation and Temperature
        let mut rgba_img = dyn_img.into_rgba8();
        if temp.abs() > 0.01 || saturation.abs() > 0.01 {
            for pixel in rgba_img.pixels_mut() {
                let [r, g, b, a] = pixel.0;
                let mut fr = r as f32;
                let mut fg = g as f32;
                let mut fb = b as f32;

                // Temperature
                if temp.abs() > 0.01 {
                    fr += (temp * 0.5) as f32;
                    fb -= (temp * 0.5) as f32;
                }

                // Saturation
                if saturation.abs() > 0.01 {
                    let gray = 0.299 * fr + 0.587 * fg + 0.114 * fb;
                    let factor = 1.0 + (saturation / 100.0) as f32;
                    fr = gray + (fr - gray) * factor;
                    fg = gray + (fg - gray) * factor;
                    fb = gray + (fb - gray) * factor;
                }

                pixel.0 = [
                    fr.clamp(0.0, 255.0) as u8,
                    fg.clamp(0.0, 255.0) as u8,
                    fb.clamp(0.0, 255.0) as u8,
                    a,
                ];
            }
        }

        let processed_frame = Frame::from_parts(rgba_img, 0, 0, delay);
        encoder.encode_frame(processed_frame).unwrap();
    }
}
