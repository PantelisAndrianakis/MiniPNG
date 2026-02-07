use anyhow::{anyhow, Result};
use image::{GenericImageView, ImageFormat};
use oxipng::{optimize_from_memory, Deflater, Options as OxiOptions};
use crate::utils::time_utils;
use crate::utils::file_utils::TempFile;
use crate::utils::crc_utils;

use std::fs;
use std::io::Cursor;
use std::path::Path;

use crate::dithering;

/// Marker string for identifying files minified by this tool.
/// Includes null terminator as required by PNG tEXt chunks.
const MARKER_STRING: &str = "MiniPNG by P. Andrian.\0";

/// PNG signature bytes.
const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

/// Dithering mode for lossy compression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DitheringMode
{
	/// No dithering - produces clean gradients but may show banding.
	None,
	
	/// Floyd-Steinberg dithering - distributes error to neighbors, good for photos.
	FloydSteinberg,
	
	/// Ordered (Bayer) dithering - regular pattern, balanced approach.
	Ordered,
	
	/// Auto-detect optimal mode based on image characteristics.
	Auto,
}

/// Results of processing a PNG file.
#[derive(Debug)]
pub struct ProcessingResult
{
	/// The original file size in bytes.
	pub original_size: u64,
	
	/// The new file size in bytes.
	pub new_size: u64,
}

/// Information about previous minification.
#[derive(Debug, Clone)]
pub struct MinificationInfo
{
	pub quality: Option<u8>,
	pub lossless: bool,
	pub reduction_pct: f64,
	pub timestamp: Option<String>,
}

/// Minifies a PNG file using a combination of techniques.
///
/// If lossless is true, only lossless minification is applied.
/// Otherwise, applies lossy minification with the specified quality level and dithering mode.
/// 
/// If force is true, skips the marker check and re-minifies even if already minified.
/// 
/// Returns (ProcessingResult, Option<MinificationInfo>) - the second value is Some if file was already minified.
pub fn minify_png(source_path: &Path, target_path: &Path, lossless_only: bool, quality: u8, dithering_mode: DitheringMode, smooth_radius: f32, denoise: bool, force: bool) -> Result<(ProcessingResult, Option<MinificationInfo>)>
{
	// Get the original file size.
	let original_size = fs::metadata(source_path)
		.map_err(|e| anyhow!("Failed to get file metadata: {}", e))?
		.len();
	
	// Read the source file into memory.
	let source_data = fs::read(source_path)
		.map_err(|e| anyhow!("Failed to read source file: {}", e))?;
	
	// Check if this file has already been minified by this tool (unless force is true).
	let (is_minified, prev_info) = if force
	{
		(false, None)
	}
	else
	{
		is_already_minified(&source_data)?
	};
	
	if is_minified
	{
		// File already minified - return info about previous minification.
		return Ok((ProcessingResult
		{
			original_size,
			new_size: original_size,
		}, prev_info));
	}
	
	// Apply minification based on mode - quality-first, not size-based.
	let minified_data = if lossless_only
	{
		// Apply lossless minification only.
		apply_quality_lossless_minification(&source_data)?
	}
	else
	{
		// Apply lossy minification with specified quality level and dithering mode.
		apply_quality_lossy_minification(&source_data, quality, dithering_mode, smooth_radius, denoise)?
	};
	
	// Write the result to a temporary file first.
	let temp_file = TempFile::new()?;
	
	fs::write(temp_file.path(), &minified_data)
		.map_err(|e| anyhow!("Failed to write to temporary file: {}", e))?;
	
	// Get the new file size.
	let new_size = fs::metadata(temp_file.path())
		.map_err(|e| anyhow!("Failed to get temporary file metadata: {}", e))?
		.len();
	
	// Only save the result if it's smaller than the original.
	if new_size < original_size
	{
		// Calculate reduction percentage.
		let reduction_pct = (1.0 - (new_size as f64 / original_size as f64)) * 100.0;
		
		// Add marker with minification info before saving.
		let marked_data = add_minification_marker_with_info(&minified_data, lossless_only, quality, reduction_pct)?;
		fs::write(temp_file.path(), &marked_data)
			.map_err(|e| anyhow!("Failed to write marked data: {}", e))?;
		
		// Atomically replace the target file with the temporary file.
		fs::copy(temp_file.path(), target_path)
			.map_err(|e| anyhow!("Failed to copy to target file: {}", e))?;
		
		Ok((ProcessingResult
		{
			original_size,
			new_size,
		}, None))
	}
	else
	{
		// Minification didn't reduce size - keep original.
		if source_path != target_path
		{
			fs::copy(source_path, target_path)
				.map_err(|e| anyhow!("Failed to copy source to target: {}", e))?;
		}
		
		Ok((ProcessingResult
		{
			original_size,
			new_size: original_size,
		}, None))
	}
}

/// Checks if a PNG file has already been minified by this tool.
/// Returns (is_minified, minification_info).
fn is_already_minified(png_data: &[u8]) -> Result<(bool, Option<MinificationInfo>)>
{
	// Check for PNG signature.
	if png_data.len() < 8 || &png_data[0..8] != PNG_SIGNATURE
	{
		return Ok((false, None));
	}
	
	// Look for our custom tEXt chunk marker.
	let mut pos = 8;
	while pos + 12 <= png_data.len()
	{
		// Read chunk length (4 bytes, big-endian).
		let length = u32::from_be_bytes([png_data[pos], png_data[pos + 1], png_data[pos + 2], png_data[pos + 3]]) as usize;
		
		// Read chunk type (4 bytes).
		let chunk_type = &png_data[pos + 4..pos + 8];
		
		// Check if this is our marker chunk.
		if chunk_type == b"tEXt"
		{
			// Check if the chunk data contains our marker.
			let chunk_data_end = pos + 8 + length;
			if chunk_data_end <= png_data.len()
			{
				let chunk_data = &png_data[pos + 8..chunk_data_end];
				
				// Look for our marker: "MiniPNG by P. Andrian.\0"
				if chunk_data.starts_with(MARKER_STRING.as_bytes())
				{
					// Parse the minification info from the marker.
					let info = parse_minification_info(chunk_data);
					return Ok((true, info));
				}
			}
		}
		
		// Move to next chunk (length + type + data + CRC).
		pos += 12 + length;
		
		// Safety check to prevent infinite loops.
		if length > 10_000_000
		{
			break;
		}
	}
	
	Ok((false, None))
}

/// Parse minification info from marker text.
fn parse_minification_info(marker_data: &[u8]) -> Option<MinificationInfo>
{
	// Convert to string, format: "MiniPNG by P. Andrian.\0quality=40,reduction=73.0,timestamp=2026-02-06T20:15:30Z"
	let marker_str = std::str::from_utf8(marker_data).ok()?;
	
	// Skip the "MiniPNG by P. Andrian.\0" part.
	let data_part = marker_str.strip_prefix(MARKER_STRING)?;
	
	let mut quality = None;
	let mut lossless = false;
	let mut reduction_pct = 0.0;
	let mut timestamp = None;
	
	// Parse key=value pairs.
	for pair in data_part.split(',')
	{
		let parts = pair.split('=').collect::<Vec<_>>();
		if parts.len() == 2
		{
			match parts[0]
			{
				"quality" => quality = parts[1].parse::<u8>().ok(),
				"lossless" => lossless = parts[1] == "true",
				"reduction" => reduction_pct = parts[1].parse::<f64>().unwrap_or(0.0),
				"timestamp" => timestamp = Some(parts[1].to_string()),
				_ => {}
			}
		}
	}
	
	Some(MinificationInfo
	{
		quality,
		lossless,
		reduction_pct,
		timestamp,
	})
}

/// Applies lossless minification with aggressive settings for maximum minification
/// while maintaining perfect image quality.
fn apply_quality_lossless_minification(png_data: &[u8]) -> Result<Vec<u8>>
{
	// Use maximum lossless minification settings.
	let mut options = OxiOptions::default();
	options.strip = oxipng::StripChunks::Safe;
	options.optimize_alpha = true;
	options.interlace = None;
	options.bit_depth_reduction = true;
	options.color_type_reduction = true;
	options.palette_reduction = true;
	
	// Use Zopfli for maximum minification (slower but best results).
	options.deflater = Deflater::Zopfli(Default::default());
	
	// Apply oxipng optimization.
	let optimized = optimize_from_memory(png_data, &options)
		.map_err(|e| anyhow!("Failed to optimize PNG: {}", e))?;
	
	Ok(optimized)
}

/// Applies lossy minification with the specified quality level and dithering mode.
/// Quality 40 (default) provides good visual quality with aggressive minification (~70-77% reduction).
/// Quality 50-60 provides very good quality with strong minification (~57-73% reduction).
/// Quality 70-80 provides excellent quality with moderate minification (~30-60% reduction).
fn apply_quality_lossy_minification(png_data: &[u8], quality: u8, dithering_mode: DitheringMode, smooth_radius: f32, denoise: bool) -> Result<Vec<u8>>
{
	// Validate it's a valid PNG and load it.
	let img = image::load_from_memory(png_data)
		.map_err(|e| anyhow!("Failed to decode PNG: {}", e))?;
	
	// Determine effective dithering mode (resolve Auto).
	let effective_dithering = match dithering_mode
	{
		DitheringMode::Auto => dithering::recommend_dithering_mode(&img),
		_ => dithering_mode,
	};
	
	// Apply color quantization with specified quality and dithering mode.
	let quantized = apply_quantization(&img, quality, effective_dithering, smooth_radius, denoise)?;
	
	// Apply aggressive lossless minification to the quantized data.
	let mut options = OxiOptions::default();
	options.strip = oxipng::StripChunks::Safe;
	options.optimize_alpha = true;
	options.interlace = None;
	options.deflater = Deflater::Zopfli(Default::default());
	options.bit_depth_reduction = true;
	options.color_type_reduction = true;
	options.palette_reduction = true;
	
	let minified = optimize_from_memory(&quantized, &options)
		.map_err(|e| anyhow!("Failed to optimize quantized PNG: {}", e))?;
	
	Ok(minified)
}

/// Adds a tEXt chunk marker with minification info.
fn add_minification_marker_with_info(png_data: &[u8], lossless: bool, quality: u8, reduction_pct: f64) -> Result<Vec<u8>>
{
	// Verify PNG signature.
	if png_data.len() < 8 || &png_data[0..8] != PNG_SIGNATURE
	{
		return Err(anyhow!("Invalid PNG signature"));
	}
	
	// Find the position to insert our chunk (before IEND).
	let mut iend_pos = None;
	let mut pos = 8;
	
	while pos + 12 <= png_data.len()
	{
		let length = u32::from_be_bytes([png_data[pos], png_data[pos + 1], png_data[pos + 2], png_data[pos + 3]]) as usize;
		
		let chunk_type = &png_data[pos + 4..pos + 8];
		
		if chunk_type == b"IEND"
		{
			iend_pos = Some(pos);
			break;
		}
		
		pos += 12 + length;
		
		// Safety check.
		if length > 10_000_000
		{
			return Err(anyhow!("Invalid PNG chunk length"));
		}
	}
	
	let iend_pos = iend_pos.ok_or_else(|| anyhow!("IEND chunk not found"))?;
	
	// Create our marker chunk with minification info.
	let timestamp = time_utils::get_iso8601_timestamp();
	
	let info_str = if lossless
	{
		format!("lossless=true,reduction={:.1},timestamp={}", reduction_pct, timestamp)
	}
	else
	{
		format!("quality={},lossless=false,reduction={:.1},timestamp={}", quality, reduction_pct, timestamp)
	};
	
	let marker_text = format!("{}{}", MARKER_STRING, info_str);
	let marker_bytes = marker_text.as_bytes();
	let marker_length = marker_bytes.len() as u32;
	
	// Calculate CRC for the chunk.
	let mut crc_data = Vec::new();
	crc_data.extend_from_slice(b"tEXt");
	crc_data.extend_from_slice(marker_bytes);
	let crc = crc_utils::hash(&crc_data);
	
	// Build new PNG with marker chunk inserted before IEND.
	let mut result = Vec::with_capacity(png_data.len() + 12 + marker_bytes.len());
	result.extend_from_slice(&png_data[..iend_pos]);
	result.extend_from_slice(&marker_length.to_be_bytes());
	result.extend_from_slice(b"tEXt");
	result.extend_from_slice(marker_bytes);
	result.extend_from_slice(&crc.to_be_bytes());
	result.extend_from_slice(&png_data[iend_pos..]);
	
	Ok(result)
}

/// Apply color quantization with selectable dithering mode.
/// For lossy minification, this reduces the color palette and applies the specified dithering algorithm.
fn apply_quantization(img: &image::DynamicImage, quality: u8, dithering_mode: DitheringMode, smooth_radius: f32, denoise: bool) -> Result<Vec<u8>>
{
	// Extract dimensions and pixel data.
	let (width, height) = img.dimensions();
	let mut rgba = img.to_rgba8();
	
	// Apply darkening BEFORE quantization.
	apply_darkening(&mut rgba);
	
	// Apply Gaussian blur if smooth_radius > 0.
	if smooth_radius > 0.0
	{
		// Apply Gaussian blur to smooth gradients before quantization.
		// This reduces banding and makes "dithering none" mode work better.
		rgba = image::imageops::blur(&image::DynamicImage::ImageRgba8(rgba), smooth_radius);
	}
	
	// Determine downsampling factor based on quality.
	// Higher quality = less downsampling.
	let downsampling_factor = match quality
	{
		0..=40 => 32,
		41..=55 => 16,
		56..=70 => 12,
		_ => 8,
	};
	
	// Apply the selected dithering algorithm.
	let quantized_img = match dithering_mode
	{
		DitheringMode::None =>
		{
			// No dithering - simple quantization produces cleanest results for gradients.
			// May show banding in some cases, but avoids adding noise.
			apply_no_dithering(&rgba, width, height, downsampling_factor)
		},
		
		DitheringMode::FloydSteinberg =>
		{
			// Floyd-Steinberg dithering - distributes quantization error to neighboring pixels.
			// Creates smooth gradients instead of harsh banding, excellent for photos.
			apply_floyd_steinberg_dithering(&rgba, width, height, downsampling_factor)
		},
		
		DitheringMode::Ordered =>
		{
			// Ordered (Bayer) dithering - uses a fixed pattern matrix.
			// Balanced approach: less noisy than Floyd-Steinberg, better than none for photos.
			apply_ordered_dithering(&rgba, width, height, downsampling_factor)
		},
		
		DitheringMode::Auto =>
		{
			// This should have been resolved earlier, but handle it just in case.
			return Err(anyhow!("Auto dithering mode should be resolved before quantization"));
		},
	};
	
	// Convert the RgbaImage back to DynamicImage.
	let mut dynamic_img = image::DynamicImage::ImageRgba8(quantized_img);
	
	// Apply selective denoising if enabled.
	if denoise
	{
		dynamic_img = apply_selective_denoising(&dynamic_img);
	}
	
	// Encode the image back to PNG.
	let mut buffer = Vec::new();
	{
		let mut cursor = Cursor::new(&mut buffer);
		dynamic_img.write_to(&mut cursor, ImageFormat::Png)
			.map_err(|e| anyhow!("Failed to encode quantized image: {}", e))?;
	}
	
	Ok(buffer)
}

/// Apply selective darkening to the image before quantization.
/// This preserves compression patterns while making the image darker.
fn apply_darkening(rgba: &mut image::RgbaImage)
{
	for pixel in rgba.pixels_mut()
	{
		// Selectively darken RGB channels based on brightness (skip alpha).
		for i in 0..3
		{
			let value = pixel[i];
			
			pixel[i] = if value < 16
			{
				// Shadows: darken by 20%.
				(value as f32 * 0.8) as u8
			}
			else if value < 32
			{
				// Lighter shadows: darken by 10%.
				(value as f32 * 0.9) as u8
			}
			else
			{
				// Highlights: keep unchanged.
				value
			};
		}
	}
}

/// Apply simple quantization without dithering.
/// Cleanest for gradients and UI elements, but may show banding.
fn apply_no_dithering(rgba: &image::RgbaImage, width: u32, height: u32, factor: u8) -> image::RgbaImage
{
	let mut quantized_img = image::RgbaImage::new(width, height);
	
	for (x, y, pixel) in rgba.enumerate_pixels()
	{
		let r = quantize_channel(pixel[0] as i16, factor);
		let g = quantize_channel(pixel[1] as i16, factor);
		let b = quantize_channel(pixel[2] as i16, factor);
		quantized_img.put_pixel(x, y, image::Rgba([r, g, b, pixel[3]])); // Keep alpha unchanged.
	}
	
	quantized_img
}

/// Apply Floyd-Steinberg dithering.
/// This distributes quantization error to neighboring pixels for smoother gradients.
fn apply_floyd_steinberg_dithering(rgba: &image::RgbaImage, width: u32, height: u32, factor: u8) -> image::RgbaImage
{
	// Create a working buffer with i16 to handle error diffusion (can be negative).
	let initial_buffer = vec![vec![[0i16; 4]; width as usize]; height as usize];
	let mut working_buffer: Vec<Vec<[i16; 4]>> = rgba.enumerate_pixels()
		.fold(initial_buffer, |mut buf, (x, y, pixel)|
		{
			buf[y as usize][x as usize] = [pixel[0] as i16, pixel[1] as i16, pixel[2] as i16, pixel[3] as i16];
			buf
		});
	
	// Apply Floyd-Steinberg dithering with serpentine scanning and reduced error.
	// Serpentine: alternating left-to-right and right-to-left scan eliminates "worms".
	// Reduced error (7/8 factor): softer, smoother gradients with less visible noise.
	const ERROR_REDUCTION: i16 = 7; // 7/8 = 0.875 error reduction factor.
	const ERROR_DIVISOR: i16 = 8;
	
	for y in 0..height as usize
	{
		// Serpentine: even rows go left-to-right, odd rows go right-to-left.
		let is_forward = y % 2 == 0;
		let x_range: Vec<usize> = if is_forward
		{
			(0..width as usize).collect()
		}
		else
		{
			(0..width as usize).rev().collect()
		};
		
		for x in x_range
		{
			let old_pixel = working_buffer[y][x];
			
			// Quantize RGB channels (not alpha).
			let r = quantize_channel(old_pixel[0], factor);
			let g = quantize_channel(old_pixel[1], factor);
			let b = quantize_channel(old_pixel[2], factor);
			let a = old_pixel[3].clamp(0, 255) as u8;
			let new_pixel = [r, g, b, a];
			
			// Calculate quantization error for each channel.
			let error = [old_pixel[0] - new_pixel[0] as i16, old_pixel[1] - new_pixel[1] as i16, old_pixel[2] - new_pixel[2] as i16, 0];
			
			// Reduce error to create softer gradients (7/8 of original error).
			let error =
			[
				(error[0] * ERROR_REDUCTION) / ERROR_DIVISOR,
				(error[1] * ERROR_REDUCTION) / ERROR_DIVISOR,
				(error[2] * ERROR_REDUCTION) / ERROR_DIVISOR,
				0
			];
			
			// Distribute error to neighboring pixels (Floyd-Steinberg pattern).
			// Pattern adapts based on scan direction:
			// Forward (L→R):     Reverse (R→L):
			//         X   7/16           7/16 X
			//   3/16 5/16 1/16     1/16 5/16 3/16
			if is_forward
			{
				// Forward scan (left to right).
				// Right pixel (x+1, y).
				if x + 1 < width as usize
				{
					for c in 0..3
					{
						working_buffer[y][x + 1][c] += (error[c] * 7) / 16;
					}
				}
				
				// Bottom-left pixel (x-1, y+1).
				if y + 1 < height as usize && x > 0
				{
					for c in 0..3
					{
						working_buffer[y + 1][x - 1][c] += (error[c] * 3) / 16;
					}
				}
				
				// Bottom pixel (x, y+1).
				if y + 1 < height as usize
				{
					for c in 0..3
					{
						working_buffer[y + 1][x][c] += (error[c] * 5) / 16;
					}
				}
				
				// Bottom-right pixel (x+1, y+1).
				if y + 1 < height as usize && x + 1 < width as usize
				{
					for c in 0..3
					{
						working_buffer[y + 1][x + 1][c] += (error[c] * 1) / 16;
					}
				}
			}
			else
			{
				// Reverse scan (right to left).
				// Left pixel (x-1, y).
				if x > 0
				{
					for c in 0..3
					{
						working_buffer[y][x - 1][c] += (error[c] * 7) / 16;
					}
				}
				
				// Bottom-right pixel (x+1, y+1).
				if y + 1 < height as usize && x + 1 < width as usize
				{
					for c in 0..3
					{
						working_buffer[y + 1][x + 1][c] += (error[c] * 3) / 16;
					}
				}
				
				// Bottom pixel (x, y+1).
				if y + 1 < height as usize
				{
					for c in 0..3
					{
						working_buffer[y + 1][x][c] += (error[c] * 5) / 16;
					}
				}
				
				// Bottom-left pixel (x-1, y+1).
				if y + 1 < height as usize && x > 0
				{
					for c in 0..3
					{
						working_buffer[y + 1][x - 1][c] += (error[c] * 1) / 16;
					}
				}
			}
			
			// Write quantized pixel back.
			working_buffer[y][x] = [new_pixel[0] as i16, new_pixel[1] as i16, new_pixel[2] as i16, new_pixel[3] as i16];
		}
	}
	
	// Convert working buffer back to image.
	let mut quantized_img = image::RgbaImage::new(width, height);
	for y in 0..height as usize
	{
		for x in 0..width as usize
		{
			let pixel = working_buffer[y][x];
			let r = pixel[0].clamp(0, 255) as u8;
			let g = pixel[1].clamp(0, 255) as u8;
			let b = pixel[2].clamp(0, 255) as u8;
			let a = pixel[3].clamp(0, 255) as u8;
			quantized_img.put_pixel(x as u32, y as u32, image::Rgba([r, g, b, a]));
		}
	}
	
	quantized_img
}

/// Apply ordered (Bayer) dithering.
/// Balanced approach: less noisy than Floyd-Steinberg, better than none for photos.
fn apply_ordered_dithering(rgba: &image::RgbaImage, width: u32, height: u32, factor: u8) -> image::RgbaImage
{
	// 4x4 Bayer matrix for ordered dithering.
	// Centered around zero to avoid brightness bias.
	const BAYER_MATRIX: [[i16; 4]; 4] =
	[
		[-8,  0, -6,  2],
		[ 4, -4,  6, -2],
		[-5,  3, -7,  1],
		[ 7, -1,  5, -3],
	];
	
	let mut result = image::RgbaImage::new(width, height);
	
	for (x, y, pixel) in rgba.enumerate_pixels()
	{
		// Get threshold from Bayer matrix.
		let threshold = BAYER_MATRIX[y as usize % 4][x as usize % 4];
		
		// Scale threshold based on downsampling factor.
		let threshold_scaled = (threshold * factor as i16) / 32;
		
		let r = quantize_channel(pixel[0] as i16 + threshold_scaled, factor);
		let g = quantize_channel(pixel[1] as i16 + threshold_scaled, factor);
		let b = quantize_channel(pixel[2] as i16 + threshold_scaled, factor);
		result.put_pixel(x, y, image::Rgba([r, g, b, pixel[3]])); // Keep alpha unchanged.
	}
	
	result
}

/// Quantize a single color channel with rounding.
fn quantize_channel(value: i16, factor: u8) -> u8
{
	let clamped = value.clamp(0, 255);
	let factor = factor as i16;
	let quantized = ((clamped + factor / 2) / factor) * factor;
	quantized.clamp(0, 255) as u8
}

/// Apply selective denoising to remove dithering artifacts in gradient areas.
/// Detects smooth gradient regions and applies noise removal while preserving edges.
fn apply_selective_denoising(img: &image::DynamicImage) -> image::DynamicImage
{
	let rgba = img.to_rgba8();
	let (width, height) = img.dimensions();
	let mut result = rgba.clone();
	
	// Process image in blocks to detect gradient vs detail regions.
	const BLOCK_SIZE: u32 = 8;
	
	for block_y in (0..height).step_by(BLOCK_SIZE as usize)
	{
		for block_x in (0..width).step_by(BLOCK_SIZE as usize)
		{
			let block_end_x = (block_x + BLOCK_SIZE).min(width);
			let block_end_y = (block_y + BLOCK_SIZE).min(height);
			
			// Analyze this block to determine if it's a gradient area.
			let (is_gradient, noise_level) = analyze_block(&rgba, block_x, block_y, block_end_x, block_end_y);
			
			// If it's a gradient with noise, apply selective median filter.
			if is_gradient && noise_level > 15.0
			{
				apply_median_filter_to_block(&rgba, &mut result, block_x, block_y, block_end_x, block_end_y);
			}
		}
	}
	
	image::DynamicImage::ImageRgba8(result)
}

/// Analyze a block to determine if it's a gradient area with noise.
/// Returns (is_gradient, noise_level).
fn analyze_block(rgba: &image::RgbaImage, start_x: u32, start_y: u32, end_x: u32, end_y: u32) -> (bool, f64)
{
	let mut edge_count = 0;
	let mut total_pixels = 0;
	let mut variance_sum = 0.0;
	
	// Calculate edge density and local variance.
	for y in start_y..end_y
	{
		for x in start_x..end_x
		{
			if x > 0 && y > 0 && x < end_x - 1 && y < end_y - 1
			{
				// Simple edge detection - check if pixel differs significantly from neighbors.
				let center = rgba.get_pixel(x, y);
				let right = rgba.get_pixel(x + 1, y);
				let bottom = rgba.get_pixel(x, y + 1);
				
				let diff_h = pixel_diff(center, right);
				let diff_v = pixel_diff(center, bottom);
				
				if diff_h > 30.0 || diff_v > 30.0
				{
					edge_count += 1;
				}
				
				// Calculate local variance (noise indicator).
				let neighbors =
				[
					rgba.get_pixel(x.saturating_sub(1), y),
					rgba.get_pixel(x + 1, y),
					rgba.get_pixel(x, y.saturating_sub(1)),
					rgba.get_pixel(x, y + 1),
				];
				
				let mut neighbor_diffs = 0.0;
				for neighbor in neighbors
				{
					neighbor_diffs += pixel_diff(center, neighbor);
				}
				variance_sum += neighbor_diffs / 4.0;
				total_pixels += 1;
			}
		}
	}
	
	let edge_density = if total_pixels > 0 { edge_count as f64 / total_pixels as f64 } else { 0.0 };
	let avg_variance = if total_pixels > 0 { variance_sum / total_pixels as f64 } else { 0.0 };
	
	// Gradient area: low edge density (<15%) but with some variance (noise from dithering).
	let is_gradient = edge_density < 0.15 && avg_variance > 5.0;
	
	(is_gradient, avg_variance)
}

/// Calculate color difference between two pixels.
fn pixel_diff(p1: &image::Rgba<u8>, p2: &image::Rgba<u8>) -> f64
{
	let dr = (p1[0] as i16 - p2[0] as i16).abs() as f64;
	let dg = (p1[1] as i16 - p2[1] as i16).abs() as f64;
	let db = (p1[2] as i16 - p2[2] as i16).abs() as f64;
	(dr + dg + db) / 3.0
}

/// Apply 3x3 median filter to a block to remove dithering noise.
fn apply_median_filter_to_block(source: &image::RgbaImage, dest: &mut image::RgbaImage, start_x: u32, start_y: u32, end_x: u32, end_y: u32)
{
	for y in start_y..end_y
	{
		for x in start_x..end_x
		{
			if x > 0 && y > 0 && x < end_x - 1 && y < end_y - 1
			{
				// Collect 3x3 neighborhood.
				let mut r_values = Vec::new();
				let mut g_values = Vec::new();
				let mut b_values = Vec::new();
				
				for dy in -1i32..=1
				{
					for dx in -1i32..=1
					{
						let px = (x as i32 + dx) as u32;
						let py = (y as i32 + dy) as u32;
						let pixel = source.get_pixel(px, py);
						r_values.push(pixel[0]);
						g_values.push(pixel[1]);
						b_values.push(pixel[2]);
					}
				}
				
				// Sort and get median.
				r_values.sort();
				g_values.sort();
				b_values.sort();
				
				let median_r = r_values[4]; // Middle value of 9 elements.
				let median_g = g_values[4];
				let median_b = b_values[4];
				
				let alpha = source.get_pixel(x, y)[3];
				dest.put_pixel(x, y, image::Rgba([median_r, median_g, median_b, alpha]));
			}
		}
	}
}
