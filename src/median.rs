use image::RgbaImage;
use std::collections::HashMap;

/// RGB color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color
{
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}

impl Color
{
	fn new(r: u8, g: u8, b: u8, a: u8) -> Self
	{
		Self { r, g, b, a }
	}
}

/// A box in RGB color space containing a range of colors.
#[derive(Clone)]
struct ColorBox
{
	colors: Vec<(Color, u32)>, // Color and frequency count.
	min_r: u8,
	max_r: u8,
	min_g: u8,
	max_g: u8,
	min_b: u8,
	max_b: u8,
}

impl ColorBox
{
	/// Create a new color box from a list of colors with frequencies.
	fn new(colors: Vec<(Color, u32)>) -> Self
	{
		let mut min_r: u8 = 255;
		let mut max_r: u8 = 0;
		let mut min_g: u8 = 255;
		let mut max_g: u8 = 0;
		let mut min_b: u8 = 255;
		let mut max_b: u8 = 0;
		
		for (color, _) in &colors
		{
			min_r = min_r.min(color.r);
			max_r = max_r.max(color.r);
			min_g = min_g.min(color.g);
			max_g = max_g.max(color.g);
			min_b = min_b.min(color.b);
			max_b = max_b.max(color.b);
		}
		
		Self
		{
			colors,
			min_r,
			max_r,
			min_g,
			max_g,
			min_b,
			max_b,
		}
	}
	
	/// Get the range (max - min) for each channel.
	fn get_ranges(&self) -> (u8, u8, u8)
	{
		(
			self.max_r - self.min_r,
			self.max_g - self.min_g,
			self.max_b - self.min_b,
		)
	}
	
	/// Find the channel with the largest range.
	fn find_widest_channel(&self) -> usize
	{
		let (r_range, g_range, b_range): (u8, u8, u8) = self.get_ranges();
		
		if r_range >= g_range && r_range >= b_range
		{
			0 // Red.
		}
		else if g_range >= b_range
		{
			1 // Green.
		}
		else
		{
			2 // Blue.
		}
	}
	
	/// Split this box into two boxes by cutting along the median of the widest channel.
	fn split(&mut self) -> Option<ColorBox>
	{
		if self.colors.len() < 2
		{
			return None;
		}
		
		let channel: usize = self.find_widest_channel();
		
		// Sort by the widest channel.
		self.colors.sort_by_key(|(c, _)| match channel
		{
			0 => c.r,
			1 => c.g,
			_ => c.b,
		});
		
		// Split at median.
		let mid: usize = self.colors.len() / 2;
		let right_colors: Vec<(Color, u32)> = self.colors.split_off(mid);
		
		// Update this box's bounds.
		*self = ColorBox::new(self.colors.clone());
		
		// Return new box.
		Some(ColorBox::new(right_colors))
	}
	
	/// Get the weighted average color (using frequency counts for better quality).
	fn get_average_color(&self) -> Color
	{
		if self.colors.is_empty()
		{
			return Color::new(0, 0, 0, 255);
		}
		
		let mut sum_r: u64 = 0;
		let mut sum_g: u64 = 0;
		let mut sum_b: u64 = 0;
		let mut total_count: u64 = 0;
		
		// Weighted average based on how often each color appears.
		for (color, count) in &self.colors
		{
			let count_u64: u64 = *count as u64;
			sum_r += color.r as u64 * count_u64;
			sum_g += color.g as u64 * count_u64;
			sum_b += color.b as u64 * count_u64;
			total_count += count_u64;
		}
		
		if total_count == 0
		{
			return Color::new(0, 0, 0, 255);
		}
		
		Color::new((sum_r / total_count) as u8, (sum_g / total_count) as u8, (sum_b / total_count) as u8, 255)
	}
}

/// Quantize an image using median cut algorithm.
pub fn quantize_image_with_median(rgba: &RgbaImage, max_colors: usize) -> RgbaImage
{
	use rayon::prelude::*;
	
	let (width, height): (u32, u32) = rgba.dimensions();
	
	// Collect unique colors (sampling for speed on large images).
	let mut color_counts: HashMap<Color, u32> = HashMap::new();
	
	// Sample every 4th pixel for speed (16x fewer pixels, still excellent coverage).
	// Median cut is very tolerant of sampling - we just need color variety, not every pixel.
	let sample_step: usize = 4;
	for y in (0..height).step_by(sample_step)
	{
		for x in (0..width).step_by(sample_step)
		{
			let pixel: &image::Rgba<u8> = rgba.get_pixel(x, y);
			let color: Color = Color::new(pixel[0], pixel[1], pixel[2], pixel[3]);
			*color_counts.entry(color).or_insert(0) += 1;
		}
	}
	
	// Start with all colors in one box (with their frequency counts).
	let mut initial_colors: Vec<(Color, u32)> = Vec::new();
	for (color, count) in color_counts
	{
		initial_colors.push((color, count));
	}
	
	let mut boxes: Vec<ColorBox> = Vec::new();
	boxes.push(ColorBox::new(initial_colors));
	
	// Split boxes until we have desired number of colors.
	while boxes.len() < max_colors
	{
		// Find the box with the largest range.
		let mut largest_idx: usize = 0;
		let mut largest_range: u32 = 0;
		
		for (i, box_) in boxes.iter().enumerate()
		{
			let (r, g, b): (u8, u8, u8) = box_.get_ranges();
			let range: u32 = r as u32 + g as u32 + b as u32;
			if range > largest_range
			{
				largest_range = range;
				largest_idx = i;
			}
		}
		
		// Split the largest box.
		if let Some(new_box) = boxes[largest_idx].split()
		{
			boxes.push(new_box);
		}
		else // Can't split anymore.
		{
			break;
		}
	}
	
	// Extract palette from boxes.
	let mut palette: Vec<Color> = Vec::new();
	palette.reserve(boxes.len());
	
	for b in &boxes
	{
		palette.push(b.get_average_color());
	}
	
	// Create the quantized image using parallel processing with per-row caching.
	let mut quantized: RgbaImage = RgbaImage::new(width, height);
	
	let rows: Vec<(u32, Vec<(u32, image::Rgba<u8>)>)> = (0..height).into_par_iter().map(|y|
	{
		let mut row_pixels: Vec<(u32, image::Rgba<u8>)> = Vec::with_capacity(width as usize);
		let mut color_cache: HashMap<Color, Color> = HashMap::new();
		
		for x in 0..width
		{
			let pixel: &image::Rgba<u8> = rgba.get_pixel(x, y);
			let original_color: Color = Color::new(pixel[0], pixel[1], pixel[2], pixel[3]);
			
			// Check cache first.
			let quantized_color: Color = if let Some(&cached) = color_cache.get(&original_color)
			{
				cached
			}
			else // Find closest palette color.
			{
				let closest: Color = find_closest_palette_color(&original_color, &palette);
				color_cache.insert(original_color, closest);
				closest
			};
			
			row_pixels.push((x, image::Rgba([quantized_color.r, quantized_color.g, quantized_color.b, quantized_color.a])));
		}
		
		(y, row_pixels)
	}).collect();
	
	// Write all rows to the output image.
	for (y, row_pixels) in rows
	{
		for (x, pixel) in row_pixels
		{
			quantized.put_pixel(x, y, pixel);
		}
	}
	
	quantized
}

/// Find the closest color in a palette.
fn find_closest_palette_color(color: &Color, palette: &[Color]) -> Color
{
	let mut best_color: Color = palette[0];
	let mut best_distance: u64 = u64::MAX;
	
	for &palette_color in palette
	{
		let distance: u64 = color_distance(color, &palette_color);
		if distance < best_distance
		{
			best_distance = distance;
			best_color = palette_color;
			
			if distance == 0
			{
				break;
			}
		}
	}
	
	best_color
}

/// Calculate squared Euclidean distance between colors.
fn color_distance(c1: &Color, c2: &Color) -> u64
{
	let dr: i64 = (c1.r as i32 - c2.r as i32) as i64;
	let dg: i64 = (c1.g as i32 - c2.g as i32) as i64;
	let db: i64 = (c1.b as i32 - c2.b as i32) as i64;
	
	(dr * dr + dg * dg + db * db) as u64
}
