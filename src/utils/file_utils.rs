use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

use crate::minify::{minify_png, ProcessingResult, MinificationInfo};

/// Represents a PNG file to process.
#[derive(Clone)]
pub struct PngFile
{
	pub source_path: PathBuf,
	pub target_path: PathBuf,
}

/// Recursively find all files in a directory that match a predicate.
pub fn find_files_recursive<F>(directory: &Path, file_predicate: F) -> Result<Vec<PathBuf>> where F: Fn(&Path) -> bool + Copy
{
	let mut result: Vec<PathBuf> = Vec::new();
	collect_files_recursive(directory, &mut result, file_predicate)?;
	
	if result.is_empty()
	{
		return Err(anyhow!("No matching files found in the directory or subdirectories."));
	}
	
	Ok(result)
}

/// Internal helper function to collect files recursively.
fn collect_files_recursive<F>(dir: &Path, files: &mut Vec<PathBuf>, file_predicate: F) -> Result<()> where F: Fn(&Path) -> bool + Copy
{
	if !dir.is_dir()
	{
		return Err(anyhow!("Not a directory: {}", dir.display()));
	}
	
	for entry in std::fs::read_dir(dir)?
	{
		let entry_result: std::fs::DirEntry = entry?;
		let path: PathBuf = entry_result.path();
		
		if path.is_dir()
		{
			// Recursively process subdirectories.
			if let Err(e) = collect_files_recursive(&path, files, file_predicate)
			{
				// Log error but continue with other directories.
				eprintln!("Error processing directory {}: {}", path.display(), e);
			}
		}
		else if file_predicate(&path)
		{
			// Add matching file to the list.
			files.push(path);
		}
	}
	
	Ok(())
}

/// Find all PNG files in a directory and its subdirectories.
pub fn find_png_files_in_dir(dir: Option<&Path>, _inplace: bool) -> Result<Vec<PngFile>>
{
	let directory: &Path = dir.unwrap_or_else(|| Path::new("."));
	let png_files: Vec<PathBuf> = find_files_recursive(directory, is_png_file)?;
	
	// Convert to PngFile structures.
	let mut result: Vec<PngFile> = Vec::new();
	result.reserve(png_files.len());
	
	for path in png_files
	{
		result.push(PngFile
		{
			source_path: path.clone(),
			target_path: path, // For in-place operations, target is the same as source.
		});
	}
	
	Ok(result)
}

/// Prepare a list of specific PNG files for processing.
pub fn prepare_specific_png_files(files: &[PathBuf], _inplace: bool) -> Vec<PngFile>
{
	let mut result: Vec<PngFile> = Vec::new();
	result.reserve(files.len());
	
	for path in files
	{
		result.push(PngFile
		{
			source_path: path.clone(),
			target_path: path.clone(), // For in-place operations, target is the same as source.
		});
	}
	
	result
}

/// Checks if a file is a PNG file by its extension.
pub fn is_png_file(path: &Path) -> bool
{
	if let Some(ext) = path.extension()
	{
		let ext_lowercase: String = ext.to_string_lossy().to_lowercase();
		return ext_lowercase == "png";
	}
	false
}

/// Process a single PNG file.
pub fn process_file(source_path: &Path, target_path: &Path, lossless: bool, quality: u8, dithering_mode: crate::minify::DitheringMode, smooth_radius: f32, denoise: bool, force: bool) -> Result<(ProcessingResult, Option<MinificationInfo>)>
{
	// Apply the minification pipeline.
	minify_png(source_path, target_path, lossless, quality, dithering_mode, smooth_radius, denoise, force)
}
