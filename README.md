# MiniPNG

Minify PNG files with imperceptible quality loss.

## What it does

This tool helps you reduce the file size of your PNG images. It:

- Makes your images take up less space.
- Keeps your images looking good.
- Works quickly on many images at once.
- Shows you how much space you saved.

## How to use it

1. Run the program to minify all .png images in the current directory and its subfolders:
   ```
   minipng
   ```
   
2. Specify a directory to process all PNG files within it and its subfolders:
   ```
   minipng --dir path\to\directory
   ```
   
3. Process specific PNG files:
   ```
   minipng file1.png path\file2.png etc
   ```

## Command line options

### Input/Output Options
- `-D, --dir <DIR>` - Directory to scan for PNG files. If not provided, current directory is used.
- `-i, --inplace` - Process files in-place (always overwrites original files). This is the default behavior.

### Operation Mode Options
- `-L, --lossless` - Use lossless compression only.
- `-F, --force` - Force re-minification of already-minified files without prompting.
- `-S, --skip` - Skip already-minified files without prompting (default for batch operations).

### Image Quality Options
- `-q, --quality <QUALITY>` - Quality level for lossy compression (1-100). Higher = better quality, larger file. Default is 40.
  - Common values: 40 (aggressive - default), 50 (balanced), 60 (high quality), 70 (excellent quality).
- `-d, --dithering <MODE>` - Dithering mode for lossy compression:
  - `auto` - Automatic selection based on image analysis (default)
  - `none` - No dithering (cleanest for gradients, may show banding)
  - `ordered` - Bayer dithering (balanced pattern)
  - `floyd` - Floyd-Steinberg error diffusion (best for photos, can be noisy)

### Advanced Image Processing Options
- `-s, --smooth <RADIUS>` - Pre-quantization smoothing radius (0.0-5.0, 0 = off). Applies Gaussian blur before color reduction to smooth gradients.
  - Recommended: 0.5-1.5 for subtle smoothing, 2.0-3.0 for aggressive smoothing.
- `-N, --denoise` - Post-processing denoising to remove dithering artifacts in gradient areas while preserving edges.

### General Options
- `-h, --help` - Display help information.
- `-V, --version` - Display version information.

## Building from source

If you want to build the program yourself:
1. Make sure you have Rust installed
2. Run `cargo build --release`
3. Find the program in the target/release folder
