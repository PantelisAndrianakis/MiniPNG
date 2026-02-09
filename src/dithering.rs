use image::{DynamicImage, GenericImageView};
use crate::minify::DitheringMode;

/// Image characteristics that help determine optimal dithering.
#[derive(Debug)]
pub struct ImageAnalysis
{
	/// Average color gradient magnitude (0-255). Lower = smoother.
	pub gradient_smoothness: f64,
	
	/// Edge density (0-1). Higher = more complex/detailed.
	pub edge_density: f64,
	
	/// Number of unique colors (after some bucketing).
	pub color_diversity: u32,
	
	/// Variance in local color changes. Lower = more uniform.
	pub local_variance: f64,
	
	/// Frequency of high-contrast edges. Higher = more photo-like.
	pub detail_frequency: f64,
}

/// Analyzes an image and recommends the best dithering mode.
/// Returns the optimal mode from: None, Ordered, FloydSteinberg, or MedianCut.
pub fn recommend_dithering_mode(img: &DynamicImage) -> DitheringMode
{
	let analysis = analyze_image(img);
	select_optimal_mode(&analysis)
}

/// Analyze key image characteristics.
fn analyze_image(img: &DynamicImage) -> ImageAnalysis
{
	let rgba = img.to_rgba8();
	let (width, height) = img.dimensions();
	
	if width == 0 || height == 0
	{
		return ImageAnalysis
		{
			gradient_smoothness: 0.0,
			edge_density: 0.0,
			color_diversity: 0,
			local_variance: 0.0,
			detail_frequency: 0.0,
		};
	}
	
	// Calculate gradient smoothness.
	let gradient_smoothness = calculate_gradient_smoothness(&rgba, width, height);
	
	// Calculate edge density.
	let edge_density = calculate_edge_density(&rgba, width, height);
	
	// Calculate color diversity.
	let color_diversity = calculate_color_diversity(&rgba);
	
	// Calculate local variance.
	let local_variance = calculate_local_variance(&rgba, width, height);
	
	// Calculate detail frequency.
	let detail_frequency = calculate_detail_frequency(&rgba, width, height);
	
	ImageAnalysis
	{
		gradient_smoothness,
		edge_density,
		color_diversity,
		local_variance,
		detail_frequency,
	}
}

/// Calculate average gradient magnitude (smoothness indicator).
/// Lower values = smoother gradients.
fn calculate_gradient_smoothness(rgba: &image::RgbaImage, width: u32, height: u32) -> f64
{
	let mut total_gradient = 0.0;
	let mut count = 0;
	
	// Sample every 4th pixel for performance.
	for y in (0..height).step_by(4)
	{
		for x in (0..width).step_by(4)
		{
			if x + 1 < width
			{
				let p1 = rgba.get_pixel(x, y);
				let p2 = rgba.get_pixel(x + 1, y);
				
				let diff = ((p2[0] as i32 - p1[0] as i32).abs() + (p2[1] as i32 - p1[1] as i32).abs() + (p2[2] as i32 - p1[2] as i32).abs()) as f64;
				
				total_gradient += diff;
				count += 1;
			}
		}
	}
	
	if count > 0
	{
		total_gradient / count as f64
	}
	else
	{
		0.0
	}
}

/// Calculate edge density using Sobel operator.
/// Higher values = more detailed/complex image.
fn calculate_edge_density(rgba: &image::RgbaImage, width: u32, height: u32) -> f64
{
	let mut edge_count = 0;
	let mut total_pixels = 0;
	
	// Sample every 4th pixel for performance.
	for y in (1..height-1).step_by(4)
	{
		for x in (1..width-1).step_by(4)
		{
			let gradient = calculate_pixel_gradient(rgba, x, y);
			
			// Threshold for edge detection.
			if gradient > 30.0
			{
				edge_count += 1;
			}
			total_pixels += 1;
		}
	}
	
	if total_pixels > 0
	{
		edge_count as f64 / total_pixels as f64
	}
	else
	{
		0.0
	}
}

/// Calculate gradient magnitude at a pixel using simplified Sobel.
fn calculate_pixel_gradient(rgba: &image::RgbaImage, x: u32, y: u32) -> f64
{
	let get_brightness = |x: u32, y: u32| -> f64
	{
		let p = rgba.get_pixel(x, y);
		
		// Simple brightness calculation.
		p[0] as f64 * 0.299 + p[1] as f64 * 0.587 + p[2] as f64 * 0.114
	};
	
	// Simplified Sobel kernels.
	let gx = -get_brightness(x-1, y-1) - 2.0*get_brightness(x-1, y) - get_brightness(x-1, y+1) + get_brightness(x+1, y-1) + 2.0*get_brightness(x+1, y) + get_brightness(x+1, y+1);
	let gy = -get_brightness(x-1, y-1) - 2.0*get_brightness(x, y-1) - get_brightness(x+1, y-1) + get_brightness(x-1, y+1) + 2.0*get_brightness(x, y+1) + get_brightness(x+1, y+1);
	(gx * gx + gy * gy).sqrt()
}

/// Calculate color diversity (number of unique colors after bucketing).
fn calculate_color_diversity(rgba: &image::RgbaImage) -> u32
{
	use std::collections::HashSet;
	
	let mut colors = HashSet::new();
	
	// Bucket colors to 16 levels per channel for faster comparison.
	for pixel in rgba.pixels()
	{
		let bucketed = (pixel[0] / 16, pixel[1] / 16, pixel[2] / 16);
		colors.insert(bucketed);
	}
	
	colors.len() as u32
}

/// Calculate local variance in color changes.
/// Lower values = more uniform image (gradients, solid colors).
fn calculate_local_variance(rgba: &image::RgbaImage, width: u32, height: u32) -> f64
{
	let mut variances = Vec::new();
	
	// Analyze 8x8 blocks.
	for block_y in (0..height).step_by(8)
	{
		for block_x in (0..width).step_by(8)
		{
			let variance = calculate_block_variance(rgba, block_x, block_y, 8, 8, width, height);
			variances.push(variance);
		}
	}
	
	if variances.is_empty()
	{
		return 0.0;
	}
	
	// Return median variance.
	variances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	variances[variances.len() / 2]
}

/// Calculate variance within a block.
fn calculate_block_variance(rgba: &image::RgbaImage, start_x: u32, start_y: u32, block_width: u32, block_height: u32, img_width: u32, img_height: u32) -> f64
{
	let mut values = Vec::new();
	
	let end_x = (start_x + block_width).min(img_width);
	let end_y = (start_y + block_height).min(img_height);
	
	for y in start_y..end_y
	{
		for x in start_x..end_x
		{
			let p = rgba.get_pixel(x, y);

			// Use brightness as representative value.
			let brightness = p[0] as f64 * 0.299 + p[1] as f64 * 0.587 + p[2] as f64 * 0.114;
			values.push(brightness);
		}
	}
	
	if values.is_empty()
	{
		return 0.0;
	}
	
	let mean = values.iter().sum::<f64>() / values.len() as f64;
	let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
	
	variance
}

/// Calculate detail frequency (high-frequency content).
/// Higher values indicate photo-like content that benefits from Floyd-Steinberg.
fn calculate_detail_frequency(rgba: &image::RgbaImage, width: u32, height: u32) -> f64
{
	let mut high_freq_count = 0;
	let mut total_samples = 0;
	
	// Sample every 4th pixel.
	for y in (2..height-2).step_by(4)
	{
		for x in (2..width-2).step_by(4)
		{
			// Check for high-frequency detail (rapid changes).
			let center = rgba.get_pixel(x, y);
			let neighbors = [rgba.get_pixel(x-1, y), rgba.get_pixel(x+1, y), rgba.get_pixel(x, y-1), rgba.get_pixel(x, y+1)];
			
			let mut changes = 0;
			for neighbor in neighbors
			{
				let diff = (center[0] as i32 - neighbor[0] as i32).abs() + (center[1] as i32 - neighbor[1] as i32).abs() + (center[2] as i32 - neighbor[2] as i32).abs();
				if diff > 20
				{
					changes += 1;
				}
			}
			
			// High-frequency if many neighbors differ significantly.
			if changes >= 2
			{
				high_freq_count += 1;
			}
			total_samples += 1;
		}
	}
	
	if total_samples > 0
	{
		high_freq_count as f64 / total_samples as f64
	}
	else
	{
		0.0
	}
}

/// Select optimal dithering mode based on analysis.
/// Chooses between None, Ordered, FloydSteinberg and MedianCut.
fn select_optimal_mode(analysis: &ImageAnalysis) -> DitheringMode
{
	// Thresholds tuned for optimal results.
	const SMOOTH_GRADIENT_THRESHOLD: f64 = 5.0;
	const LOW_EDGE_THRESHOLD: f64 = 0.15;
	const LOW_VARIANCE_THRESHOLD: f64 = 200.0;
	const LOW_COLOR_DIVERSITY: u32 = 100;
	const MODERATE_COLOR_DIVERSITY: u32 = 300;
	const HIGH_COLOR_DIVERSITY: u32 = 500;
	const HIGH_DETAIL_FREQUENCY: f64 = 0.25;
	const PHOTO_EDGE_THRESHOLD: f64 = 0.35;
	
	// Decision tree based on image characteristics.
	
	// Very smooth gradient with low complexity -> No dithering (cleanest).
	if analysis.gradient_smoothness < SMOOTH_GRADIENT_THRESHOLD && analysis.edge_density < LOW_EDGE_THRESHOLD && analysis.local_variance < LOW_VARIANCE_THRESHOLD
	{
		return DitheringMode::None;
	}
	
	// Simple image with few colors -> No dithering.
	if analysis.color_diversity < LOW_COLOR_DIVERSITY && analysis.edge_density < LOW_EDGE_THRESHOLD
	{
		return DitheringMode::None;
	}
	
	// Many distinct colors but not photo-like -> MedianCut (great for logos, illustrations, UI).
	// High color diversity with moderate edges suggests distinct color regions rather than smooth gradients.
	if analysis.color_diversity > MODERATE_COLOR_DIVERSITY  && analysis.color_diversity < HIGH_COLOR_DIVERSITY && analysis.edge_density > LOW_EDGE_THRESHOLD  && analysis.edge_density < PHOTO_EDGE_THRESHOLD && analysis.detail_frequency < HIGH_DETAIL_FREQUENCY
	{
		return DitheringMode::MedianCut;
	}
	
	// Photo-like with high detail frequency -> Floyd-Steinberg for best quality.
	if analysis.detail_frequency > HIGH_DETAIL_FREQUENCY && analysis.edge_density > PHOTO_EDGE_THRESHOLD && analysis.color_diversity > MODERATE_COLOR_DIVERSITY
	{
		return DitheringMode::FloydSteinberg;
	}
	
	// High complexity photo-like content -> Floyd-Steinberg.
	if analysis.edge_density > 0.4 && analysis.local_variance > 600.0 && analysis.color_diversity > HIGH_COLOR_DIVERSITY
	{
		return DitheringMode::FloydSteinberg;
	}
	
	// Moderately complex but still fairly smooth -> Ordered dithering.
	if analysis.gradient_smoothness < SMOOTH_GRADIENT_THRESHOLD * 2.0 && analysis.edge_density < 0.3
	{
		return DitheringMode::Ordered;
	}
	
	// Default: Ordered dithering as safe middle ground.
	DitheringMode::Ordered
}
