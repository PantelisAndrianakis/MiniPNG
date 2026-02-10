use std::path::{Path, PathBuf};
use std::env;
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Args
{
	// 1. Input/Output Parameters.
	/// PNG files to process. If not provided, all PNGs in current directory and subdirectories will be processed.
	pub files: Vec<PathBuf>,
	
	/// Directory to scan for PNG files. If not provided, current directory is used.
	pub dir: Option<PathBuf>,
	
	/// Process files in-place (always overwrites original files).
	/// This is the default behavior and the flag is kept for backward compatibility.
	pub inplace: bool,
	
	// 2. Operation Mode Parameters.
	/// Use lossless compression only.
	pub lossless: bool,
	
	/// Force re-minification of already-minified files without prompting.
	pub force: bool,
	
	/// Skip already-minified files without prompting (default for batch operations).
	pub skip: bool,
	
	// 3. Image Quality Parameters.
	/// Quality level for lossy compression (1-100). Higher = better quality, larger file.
	/// Default is 40 which provides good quality with aggressive compression (~700-930KB for 3MB file).
	/// Common values: 40 (aggressive - default), 50 (balanced), 60 (high quality), 70 (excellent quality).
	pub quality: u8,
	
	/// Dithering mode for lossy compression.
	/// auto = automatic selection based on image analysis (default).
	/// none = no dithering (cleanest for gradients, may show banding).
	/// ordered = Bayer dithering (balanced pattern).
	/// floyd = Floyd-Steinberg error diffusion (best for photos, can be noisy).
	/// median = Median cut color quantization (excellent palette quality).
	pub dithering: String,
	
	// 4. Advanced Image Processing Parameters.
	/// Pre-quantization smoothing radius (0.0-5.0, 0 = off).
	/// Applies Gaussian blur before color reduction to smooth gradients.
	/// Recommended: 0.5-1.5 for subtle smoothing, 2.0-3.0 for aggressive smoothing.
	/// Works great with --dithering none to eliminate banding in smooth gradients.
	pub smooth: f32,
	
	/// Post-processing denoising to remove dithering artifacts.
	/// Detects and smooths dithering noise in gradient areas while preserving edges.
	/// Use when you see grainy dots in smooth areas after processing.
	pub denoise: bool,
	
	// 5. Program Metadata.
	/// Program version info.
	pub version: String,
	
	/// Program author info.
	pub author: String,
	
	/// Program description.
	pub about: String,
}

impl Args
{
	/// Create a new Args instance with default values.
	pub fn new() -> Self
	{
		Args
		{
			files: Vec::new(),
			dir: None,
			inplace: true,
			lossless: false,
			quality: 40,
			force: false,
			skip: false,
			dithering: "floyd".to_string(),
			smooth: 0.0,
			denoise: false,
			version: env!("CARGO_PKG_VERSION").to_string(),
			author: env!("CARGO_PKG_AUTHORS").to_string(),
			about: env!("CARGO_PKG_DESCRIPTION").to_string(),
		}
	}
	
	/// Parse command line arguments and return an Args struct.
	pub fn parse() -> Result<Self>
	{
		let mut args: Args = Args::new();
		
		// Get all command line arguments.
		let mut cli_args: Vec<String> = Vec::new();
		for arg in env::args()
		{
			cli_args.push(arg);
		}
		
		// Skip the program name (first argument).
		if !cli_args.is_empty()
		{
			cli_args.remove(0);
		}
		
		// Process arguments.
		let mut i: usize = 0;
		while i < cli_args.len()
		{
			let arg: &String = &cli_args[i];
			
			match arg.as_str()
			{
				// 1. Input/Output Parameters.
				"--dir" | "-D" =>
				{
					if i + 1 < cli_args.len()
					{
						i += 1;
						args.dir = Some(PathBuf::from(&cli_args[i]));
					}
					else
					{
						return Err(anyhow!("Missing value for {} argument", arg));
					}
				}
				"--inplace" | "-i" =>
				{
					args.inplace = true;
				}
				
				// 2. Operation Mode Parameters.
				"--lossless" | "-L" =>
				{
					args.lossless = true;
				}
				"--force" | "-F" =>
				{
					args.force = true;
				}
				"--skip" | "-S" =>
				{
					args.skip = true;
				}
				
				// 3. Image Quality Parameters.
				"--quality" | "-q" =>
				{
					if i + 1 < cli_args.len()
					{
						i += 1;
						let value: u8 = cli_args[i].parse::<u8>().map_err(|_| anyhow!("Invalid quality value: must be an integer between 1 and 100"))?;
						args.quality = value;
					}
					else
					{
						return Err(anyhow!("Missing value for {} argument", arg));
					}
				}
				"--dithering" | "-d" | "-m" => // Keep -m for backward compatibility.
				{
					if i + 1 < cli_args.len()
					{
						i += 1;
						args.dithering = cli_args[i].to_string();
					}
					else
					{
						return Err(anyhow!("Missing value for {} argument", arg));
					}
				}
				
				// 4. Advanced Image Processing Parameters.
				"--smooth" | "-s" | "-r" => // Keep -r for backward compatibility.
				{
					if i + 1 < cli_args.len()
					{
						i += 1;
						let value: f32 = cli_args[i].parse::<f32>().map_err(|_| anyhow!("Invalid smooth value: must be a number between 0.0 and 5.0"))?;
						args.smooth = value;
					}
					else
					{
						return Err(anyhow!("Missing value for {} argument", arg));
					}
				}
				"--denoise" | "-N" =>
				{
					args.denoise = true;
				}
				
				// 5. Program Information.
				"--help" | "-h" =>
				{
					println!("{} - {}", args.about, args.version);
					println!("By {}", args.author);
					println!("\nUSAGE:");
					println!("    minipng [OPTIONS] [FILES...]");
					println!("\nOPTIONS:");
					// Input/Output Parameters.
					println!("  INPUT/OUTPUT:");
					println!("    -D, --dir <DIR>              Directory to scan for PNG files");
					println!("    -i, --inplace                Process files in-place (default)");
					println!("");
					// Operation Mode Parameters.
					println!("  OPERATION MODE:");
					println!("    -L, --lossless               Use lossless compression only");
					println!("    -F, --force                  Force re-minification of already-minified files");
					println!("    -S, --skip                   Skip already-minified files");
					println!("");
					// Image Quality Parameters.
					println!("  IMAGE QUALITY:");
					println!("    -q, --quality <QUALITY>      Quality level (1-100, default: 40)");
					println!("    -d, --dithering <MODE>       Dithering mode (auto, none, ordered, floyd, median)");
					println!("");
					// Advanced Image Processing Parameters.
					println!("  ADVANCED PROCESSING:");
					println!("    -s, --smooth <RADIUS>        Pre-quantization smoothing radius (0.0-5.0)");
					println!("    -N, --denoise                Apply post-processing denoising");
					println!("");
					// General Options.
					println!("  GENERAL:");
					println!("    -h, --help                   Show help information");
					println!("    -V, --version                Display version information");
					
					std::process::exit(0);
				}
				"--version" | "-V" =>
				{
					println!("{} {}", env!("CARGO_PKG_NAME"), args.version);
					std::process::exit(0);
				}
				
				// Anything else is treated as a file path if it doesn't start with "-".
				_ =>
				{
					if !arg.starts_with('-')
					{
						args.files.push(PathBuf::from(arg));
					}
					else
					{
						return Err(anyhow!("Unknown option: {}", arg));
					}
				}
			}
			
			i += 1;
		}
		
		Ok(args)
	}
	
	/// Check if an argument was explicitly provided on the command line.
	pub fn is_explicitly_set(arg_name: &str) -> bool
	{
		let args: Vec<String> = env::args().collect();
		
		// Check if the argument is present in the command line.
		for a in &args
		{
			if a == arg_name
			{
				return true;
			}
		}
		
		false
	}
	
	/// Validate parameter values and relationships.
	/// Returns Ok(()) if all parameters are valid, otherwise returns an error.
	pub fn validate(&self) -> Result<()>
	{
		// Validate quality parameter.
		if self.quality == 0 || self.quality > 100
		{
			return Err(anyhow!("Quality must be between 1 and 100"));
		}
		
		// Validate smooth parameter.
		if self.smooth < 0.0 || self.smooth > 5.0
		{
			return Err(anyhow!("Smooth radius must be between 0.0 and 5.0"));
		}
		
		// Validate that force and skip are not both set.
		if self.force && self.skip
		{
			return Err(anyhow!("Cannot use --force and --skip together"));
		}
		
		// Validate dithering mode.
		match self.dithering.to_lowercase().as_str()
		{
			"auto" | "none" | "ordered" | "floyd" | "floyd-steinberg" | "mediancut" | "median" => {},
			_ => return Err(anyhow!("Invalid dithering mode. Use: auto, none, ordered, floyd, or median")),
		}
		
		// All validations passed.
		Ok(())
	}
}

/// Enumeration representing the mode of operation.
pub enum Mode
{
	Directory(Option<PathBuf>),
	Files(Vec<PathBuf>),
}

/// Determines the mode of operation based on the provided arguments.
pub fn determine_mode(args: &Args, is_png_file: fn(&Path) -> bool) -> Result<Mode>
{
	// If specific files are provided, they take precedence.
	if !args.files.is_empty()
	{
		// Validate each file.
		let mut png_files: Vec<PathBuf> = Vec::new();
		
		for path in &args.files
		{
			if path.is_file() && is_png_file(path)
			{
				png_files.push(path.clone());
			}
			else
			{
				return Err(anyhow!("Input '{}' is not a PNG file.", path.display()));
			}
		}
		
		if png_files.is_empty()
		{
			return Err(anyhow!("No valid PNG files provided."));
		}
		
		Ok(Mode::Files(png_files))
	}
	else // If no files are specified, use directory mode. Use the specified directory or default to current.
	{
		Ok(Mode::Directory(args.dir.clone()))
	}
}
