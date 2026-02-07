use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::sync::{Arc, Mutex};

mod minify;
mod dithering;
mod utils
{
	pub mod arg_utils;
	pub mod crc_utils;
	pub mod file_utils;
	pub mod time_utils;
}
use utils::arg_utils::{Args, Mode, determine_mode};
use utils::file_utils::{is_png_file, find_png_files_in_dir, prepare_specific_png_files, process_file};
use utils::time_utils::format_timestamp;


/// Minify PNG files with imperceptible quality loss.
fn main() -> Result<()>
{
	// Parse command line arguments.
	let args = Args::parse()?;
	
	// Validate parameters using the centralized validation method.
	args.validate()?;
	
	// Parse dithering mode.
	let dithering_mode = match args.dithering.to_lowercase().as_str()
	{
		"auto" => minify::DitheringMode::Auto,
		"none" => minify::DitheringMode::None,
		"ordered" => minify::DitheringMode::Ordered,
		"floyd" | "floyd-steinberg" => minify::DitheringMode::FloydSteinberg,
		_ => return Err(anyhow!("Invalid dithering mode. Use: auto, none, ordered, or floyd")),
	};
	
	// Determine the mode of operation.
	let mode = determine_mode(&args, is_png_file)?;
	
	// Print the processing settings with logical grouping.
	println!("Settings:");
	println!("----------------------------------------");
	
	// 1. Input/Output Parameters.
	println!("INPUT/OUTPUT:");
	if let Some(dir) = &args.dir
	{
		println!("  - Directory: {}", dir.display());
	}
	let mode_desc = match determine_mode(&args, is_png_file)? {
		Mode::Directory(_) => "Directory Mode",
		Mode::Files(_) => "Specific Files Mode",
	};
	println!("  - Mode: {}", mode_desc);
	println!("  - In-place: {}", if args.inplace { "Yes" } else { "No" });
	
	// 2. Operation Mode Parameters.
	println!("\nOPERATION:");
	println!("  - Lossless: {}", if args.lossless { "Yes" } else { "Off" });
	println!("  - Force re-minify: {}", if args.force { "Yes" } else { "Off" });
	println!("  - Skip already-minified: {}", if args.skip { "Yes" } else { "Off" });
	
	// 3. Image Quality Parameters.
	println!("\nIMAGE QUALITY:");
	if !args.lossless
	{
		println!("  - Quality: {}", args.quality);
		
		// Add quality level description.
		let quality_desc = match args.quality
		{
			1..=40 => "Aggressive minification - smallest files, good quality (DEFAULT)",
			41..=55 => "Balanced minification - small files, very good quality",
			56..=65 => "High quality minification - medium files, excellent quality",
			66..=75 => "Very high quality - larger files, near-perfect quality",
			76..=100 => "Maximum quality - largest files, perfect quality",
			_ => "Custom quality level",
		};
		println!("    ({}", quality_desc);
		
		// Add downsampling factor info.
		let downsampling_factor = match args.quality
		{
			0..=40 => 32,
			41..=55 => 16,
			56..=70 => 12,
			_ => 8,
		};
		println!("     Downsampling: ÷{}, Colors reduced for minification)", downsampling_factor);
		
		// Add dithering mode info.
		let dithering_desc = match dithering_mode
		{
			minify::DitheringMode::Auto => "Auto (analyzes image to select best mode)",
			minify::DitheringMode::None => "None (clean gradients, may show banding)",
			minify::DitheringMode::Ordered => "Ordered/Bayer (balanced pattern)",
			minify::DitheringMode::FloydSteinberg => "Floyd-Steinberg (best for photos)",
		};
		println!("  - Dithering: {}", dithering_desc);
	}
	else
	{
		println!("  - Quality: Perfect (lossless PNG optimization only)");
		println!("  - Dithering: N/A (lossless mode)");
	}
	
	// 4. Advanced Image Processing Parameters.
	println!("\nADVANCED PROCESSING:");
	// Add smoothing info.
	if args.smooth > 0.0
	{
		println!("  - Smoothing: {:.1} (Gaussian blur before quantization)", args.smooth);
	}
	else
	{
		println!("  - Smoothing: Off");
	}
	
	// Add denoising info.
	if args.denoise
	{
		println!("  - Denoising: Yes (removes dithering artifacts in gradients)");
	}
	else
	{
		println!("  - Denoising: Off");
	}
	println!("----------------------------------------");
	
	// Show minification info.
	println!();
	if args.lossless
	{
		println!("Minification mode: Lossless optimization");
		println!("  - Removes unnecessary metadata.");
		println!("  - Optimizes PNG compression (Zopfli algorithm).");
		println!("  - Preserves perfect image quality.");
		println!("  - Expected reduction: 10-30%.");
	}
	else
	{
		println!("Minification mode: Lossy (Quality {})", args.quality);
		println!("  - Reduces color palette through quantization.");
		println!("  - Applies aggressive PNG optimization.");
		println!("  - Maintains excellent visual quality.");
		let expected_reduction = match args.quality
		{
			1..=40 => "70-77%",
			41..=55 => "63-73%",
			56..=65 => "57-70%",
			66..=75 => "43-60%",
			76..=100 => "30-50%",
			_ => "varies",
		};
		println!("  - Expected reduction: {}.", expected_reduction);
	}
	println!("  - Files already minified by this tool will be skipped.");
	println!();
	
	// Discover PNG files to process.
	let (png_files, explicit_files) = match mode
	{
		Mode::Directory(dir) =>
		{
			if let Some(d) = &dir
			{
				println!("Scanning directory '{}' for PNG files...", d.display());
			}
			else
			{
				println!("Scanning current directory for PNG files...");
			}

			(find_png_files_in_dir(dir.as_deref(), args.inplace)?, false)
		},
		Mode::Files(files) =>
		{
			println!("Processing {} specified PNG files...", files.len());
			(prepare_specific_png_files(&files, args.inplace), true)
		},
	};
	
	// Display discovered files.
	println!("Found {} PNG files to process:", png_files.len());
	for file in &png_files
	{
		if file.source_path == file.target_path
		{
			println!("  - {} (in-place)", file.source_path.display());
		}
		else
		{
			println!("  - {} -> {}", file.source_path.display(), file.target_path.display());
		}
	}
	println!();
	
	// Process each file in parallel.
	let results = Arc::new(Mutex::new(Vec::new()));
	let errors = Arc::new(Mutex::new(Vec::new()));
	
	println!("Processing files...");
	
	// Create a progress counter.
	let total_files = png_files.len();
	let processed = Arc::new(Mutex::new(0));
	
	let quality = args.quality;
	let lossless = args.lossless;
	let force_reminify = args.force;
	let skip_without_prompting = args.skip;
	let smooth_radius = args.smooth;
	let denoise = args.denoise;
	
	// Check if quality was explicitly set (not default 60).
	let quality_explicitly_set = Args::is_explicitly_set("--quality") || Args::is_explicitly_set("-q");
	
	// Check if lossless was explicitly set.
	let lossless_explicitly_set = Args::is_explicitly_set("--lossless");
	
	// Determine if we should prompt:
	// 1. NOT if --force or --skip is set
	// 2. Single file explicitly specified (minipng image.png)
	// 3. Single file found + quality explicitly set (minipng --quality 45 when one file in directory)
	// 4. Single file found + lossless explicitly set (minipng --lossless when one file in directory)
	let is_single_file = total_files == 1;
	let parameters_explicitly_set = quality_explicitly_set || lossless_explicitly_set;
	let should_prompt_on_skip = !force_reminify && !skip_without_prompting && is_single_file && (explicit_files || parameters_explicitly_set);
	
	// Process single file separately (non-parallel) to allow prompting or forced re-minification.
	if should_prompt_on_skip || (force_reminify && is_single_file)
	{
		println!("Processing file...");
		
		let file = &png_files[0];
		let file_path_display = file.source_path.display().to_string();
		
		// First check if already minified.
		match process_file(&file.source_path, &file.target_path, lossless, quality, dithering_mode, smooth_radius, denoise, force_reminify)
		{
			Ok((result, prev_info)) =>
			{
				// Check if file was already minified.
				if let Some(ref info) = prev_info
				{
					// File was already minified.
					if force_reminify
					{
						// Force flag set - skip prompting, already re-minified above.
						let size_reduction_pct = if result.original_size > 0
						{
							(1.0 - (result.new_size as f64 / result.original_size as f64)) * 100.0
						}
						else
						{
							0.0
						};
						
						if result.new_size < result.original_size
						{
							println!("Re-minified: {} | {} -> {} ({:.1}% smaller)", file_path_display, format_bytes(result.original_size), format_bytes(result.new_size), size_reduction_pct);
						}
						else
						{
							println!("No reduction: {} (file couldn't be minified further)", file_path_display);
						}
						
						return Ok(());
					}
					
					// Show previous minification details and prompt.
					let prev_settings = if info.lossless
					{
						"Lossless".to_string()
					}
					else
					{
						format!("Quality {}", info.quality.unwrap_or(0))
					};
					
					// Calculate original size.
					let current_size = result.original_size;
					let original_size_before = if info.reduction_pct > 0.0
					{
						(current_size as f64 / (1.0 - info.reduction_pct / 100.0)) as u64
					}
					else
					{
						current_size
					};
					
					// Format sizes.
					println!("\nFile already minified:");
					println!("  Previous mode: {}", prev_settings);
					println!("  Previous reduction: {:.1}%", info.reduction_pct);
					println!("  Original: {} -> Minified: {}", format_bytes(original_size_before), format_bytes(current_size));
					if let Some(ref ts) = info.timestamp
					{
						println!("  Minified on: {}", format_timestamp(ts));
					}
					
					// Prompt user.
					println!("\nCurrent settings:");
					if lossless
					{
						println!("  Mode: Lossless");
					}
					else
					{
						println!("  Mode: Quality {}", quality);
					}
					
					print!("\nRe-minify with current settings? (y/N): ");
					use std::io::{self, Write};
					io::stdout().flush().ok();
					
					let stdin = io::stdin();
					let mut response = String::new();
					stdin.read_line(&mut response).expect("Failed to read user input");
					
					let should_reminify = response.trim().to_lowercase() == "y";
					
					if !should_reminify
					{
						println!("Skipped.");
						return Ok(());
					}
					
					println!("Re-minifing...");
					
					// Re-minify with force=true to bypass marker check.
					match process_file(&file.source_path, &file.target_path, lossless, quality, dithering_mode, smooth_radius, denoise, true)
					{
						Ok((recomp_result, _)) =>
						{
							let size_reduction_pct = if recomp_result.original_size > 0
							{
								(1.0 - (recomp_result.new_size as f64 / recomp_result.original_size as f64)) * 100.0
							}
							else
							{
								0.0
							};
							
							if recomp_result.new_size < recomp_result.original_size
							{
								println!("Re-minified: {} | {} -> {} ({:.1}% smaller)", file_path_display, format_bytes(recomp_result.original_size), format_bytes(recomp_result.new_size), size_reduction_pct);
							}
							else
							{
								println!("No reduction: {} (file couldn't be minified further)", file_path_display);
							}
						},
						Err(err) =>
						{
							eprintln!("Error re-minifing {}: {}", file_path_display, err);
						}
					}
					
					return Ok(());
				}
				
				// File was not previously minified - show normal minification result.
				let size_reduction_pct = if result.original_size > 0
				{
					(1.0 - (result.new_size as f64 / result.original_size as f64)) * 100.0
				}
				else
				{
					0.0
				};
				
				if result.new_size < result.original_size
				{
					println!("Minified: {} | {} -> {} ({:.1}% smaller)", file_path_display, format_bytes(result.original_size), format_bytes(result.new_size), size_reduction_pct);
				}
				else
				{
					println!("No reduction: {} (file couldn't be minified further)", file_path_display);
				}
			},
			Err(err) =>
			{
				eprintln!("Error processing {}: {}", file_path_display, err);
			}
		}
		
		return Ok(());
	}
	
	// Batch mode: process in parallel with auto-skip (unless --force is set).
	png_files.into_par_iter().for_each(|file|
	{
		let file_path_display = file.source_path.display().to_string();
		
		match process_file(&file.source_path, &file.target_path, lossless, quality, dithering_mode, smooth_radius, denoise, force_reminify)
		{
			Ok((result, prev_info)) =>
			{
				// Check if file was already minified.
				if let Some(ref info) = prev_info
				{
					// File was already minified.
					if force_reminify
					{
						// Force mode - file was already re-minified, show result.
						let size_reduction_pct = if result.original_size > 0
						{
							(1.0 - (result.new_size as f64 / result.original_size as f64)) * 100.0
						}
						else
						{
							0.0
						};
						
						let mut count = processed.lock().expect("Processed counter mutex poisoned");
						*count += 1;
						let current = *count;
						
						if result.new_size < result.original_size
						{
							println!("[{}/{}] Re-minified: {} | {} -> {} ({:.1}% smaller)", current, total_files, file_path_display, format_bytes(result.original_size), format_bytes(result.new_size), size_reduction_pct);
						}
						else
						{
							println!("[{}/{}] No reduction: {}", current, total_files, file_path_display);
						}
						
						results.lock().expect("Results mutex poisoned").push(result);
						return;
					}
					
					// Auto-skip mode (batch or --skip flag).
					let mut count = processed.lock().expect("Processed counter mutex poisoned");
					*count += 1;
					let current = *count;
					
					// Show comprehensive previous minification info.
					let prev_settings = if info.lossless
					{
						"Lossless".to_string()
					}
					else
					{
						format!("Quality {}", info.quality.unwrap_or(0))
					};
					
					// Calculate what the original size was before minification.
					let current_size = result.original_size;
					let original_size_before = if info.reduction_pct > 0.0
					{
						(current_size as f64 / (1.0 - info.reduction_pct / 100.0)) as u64
					}
					else
					{
						current_size
					};
					
					// Format sizes.
					println!("[{}/{}] Skipped: {}", current, total_files, file_path_display);
					println!("    Already minified | Mode: {} | Reduction: {:.1}%", prev_settings, info.reduction_pct);
					println!("    Original: {} -> Minified: {}", format_bytes(original_size_before), format_bytes(current_size));
					if let Some(ref ts) = info.timestamp
					{
						println!("    Minified on: {}", format_timestamp(ts));
					}
					
					results.lock().expect("Results mutex poisoned").push(result);
					return;
				}
				
				// File was not previously minified, or user chose to re-minify.
				// Calculate the size reduction percentage.
				let size_reduction_pct = if result.original_size > 0
				{
					(1.0 - (result.new_size as f64 / result.original_size as f64)) * 100.0
				}
				else
				{
					0.0
				};
				
				// Update the progress counter.
				let mut count = processed.lock().expect("Processed counter mutex poisoned");
				*count += 1;
				let current = *count;
				
				// Format sizes for display.

				
				// Show detailed progress.
				if result.original_size == result.new_size && prev_info.is_none()
				{
					println!("[{}/{}] No reduction: {} (file couldn't be minified further)", current, total_files, file_path_display);
				}
				else if result.new_size < result.original_size
				{
					println!("[{}/{}] Minified: {} | {} -> {} ({:.1}% smaller)", current, total_files, file_path_display, format_bytes(result.original_size), format_bytes(result.new_size), size_reduction_pct);
				}
				
				results.lock().expect("Results mutex poisoned").push(result);
			},
			Err(err) =>
			{
				eprintln!("Error processing {}: {}", file_path_display, err);
				errors.lock().expect("Errors mutex poisoned").push((file_path_display, err.to_string()));
			}
		}
	});
	
	// Convert back to a regular Vec.
	let results = Arc::try_unwrap(results)
		.unwrap_or_else(|_| panic!("Failed to unwrap Arc"))
		.into_inner()
		.expect("Mutex poisoned");
	
	let errors = Arc::try_unwrap(errors)
		.unwrap_or_else(|_| panic!("Failed to unwrap Arc"))
		.into_inner()
		.expect("Mutex poisoned");
	
	// Print summary.
	println!("\n========================================");
	println!("MINIFICATION SUMMARY");
	println!("========================================");
	println!("Total files processed successfully: {}", results.len());
	
	if !errors.is_empty()
	{
		println!("Files with errors: {}", errors.len());
		println!("\nErrors:");
		for (file, error) in &errors
		{
			println!("  {}: {}", file, error);
		}
	}
	
	if !results.is_empty()
	{
		// Count skipped vs minified files.
		let minified_count = results.iter().filter(|r| r.new_size < r.original_size).count();
		let skipped_count = results.iter().filter(|r| r.new_size == r.original_size).count();
		
		println!("\nFiles minified: {}", minified_count);
		println!("Files skipped (already minified): {}", skipped_count);
		
		let total_original_size: u64 = results.iter().map(|r| r.original_size).sum();
		let total_new_size: u64 = results.iter().map(|r| r.new_size).sum();
		let total_saved = total_original_size.saturating_sub(total_new_size);
		let total_saved_pct = if total_original_size > 0
		{
			(total_saved as f64 / total_original_size as f64) * 100.0
		}
		else
		{
			0.0
		};
		
		// Format the size in a human-readable way (KB, MB).
		let format_size = |size: u64| -> String
		{
			if size < 1024
			{
				format!("{} bytes", size)
			}
			else if size < 1024 * 1024
			{
				format!("{:.2} KB", size as f64 / 1024.0)
			}
			else
			{
				format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
			}
		};
		
		println!("\n----------------------------------------");
		println!("SIZE STATISTICS");
		println!("----------------------------------------");
		println!("Total original size:  {}", format_size(total_original_size));
		println!("Total final size:     {}", format_size(total_new_size));
		println!("Total space saved:    {} ({:.1}%)", format_size(total_saved), total_saved_pct);
		
		if minified_count > 0
		{
			// Calculate average compression for files that were actually minified.
			let minified_original: u64 = results.iter()
				.filter(|r| r.new_size < r.original_size)
				.map(|r| r.original_size)
				.sum();
			let minified_new: u64 = results.iter()
				.filter(|r| r.new_size < r.original_size)
				.map(|r| r.new_size)
				.sum();
			let avg_compression = if minified_original > 0
			{
				(1.0 - (minified_new as f64 / minified_original as f64)) * 100.0
			}
			else
			{
				0.0
			};
			println!("Average compression (minified files only): {:.1}%", avg_compression);
		}
		
		println!("========================================");
	}
	
	Ok(())
}

/// Formats file size in human-readable format.
fn format_bytes(size: u64) -> String
{
	if size < 1024
	{
		format!("{} B", size)
	}
	else if size < 1024 * 1024
	{
		format!("{:.1} KB", size as f64 / 1024.0)
	}
	else
	{
		format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
	}
}
