# Coding Style Guide

This document describes the coding conventions for this project.

## Core Principles

- **Clarity over cleverness**: Code should be immediately understandable.
- **Consistent formatting**: Use the same patterns throughout the codebase.
- **Explicit over implicit**: Prefer explicit type annotations and error handling.
- **Simplicity over forced symmetry**: Don't force vertical alignment when a simple line works fine.
- **Allman style**: Opening braces on new lines for visual symmetry and easy scanning.
- **Performance-conscious**: Write efficient code but prioritize readability first.

---

## Formatting

### Indentation
- **Use tabs for indentation** (not spaces).
- Configure your editor to use tabs.

### Braces and Brackets Placement
- **Place opening braces `{` and brackets `[` on a new line** for:
  - Function definitions.
  - Struct definitions.
  - Enum definitions.
  - Impl blocks.
  - Match expressions.
  - If/else blocks.
  - For/while loops.
  - Multi-line closures.
  - Multi-line array/slice initializations.

**Examples:**

```rust
// Function definition.
fn process_file(source_path: &Path, target_path: &Path, options: &ProcessingOptions) -> Result<ProcessingResult>
{
	// Function body.
}

// Struct definition.
pub struct FileInfo
{
	pub source_path: PathBuf,
	pub target_path: PathBuf,
}

// Enum definition.
enum Mode
{
	Directory(Option<PathBuf>),
	Files(Vec<PathBuf>),
}

// Match expression.
let status = match args.priority
{
	1..=3 => "high",
	4..=7 => "medium",
	_ => "low",
};

// If/else blocks.
if args.force && args.skip
{
	return Err(anyhow!("Cannot use --force and --skip together"));
}

// For loops.
for file in &files
{
	println!("Processing: {}", file.source_path.display());
}
```

**Exception - Single-line closures:**
Single-line closures can have braces on the same line:

```rust
.filter(|entry| entry.file_type().is_file())
.map(|entry| entry.path().to_path_buf())
```

**Exception - Inline multi-line closures in method chains:**
When closures span multiple lines in method chains:

```rust
.filter(|entry|
{
	entry.file_type().is_file() && 
	entry.path().extension()
		.map(|ext| ext.to_string_lossy().to_lowercase() == "txt")
		.unwrap_or(false)
})
```

### Function Signatures
- **Write all parameters on a single line whenever possible**.
- Only break to multiple lines if the signature exceeds ~100-120 characters.
- When breaking, put each parameter on its own line with proper indentation.

**Good:**
```rust
pub fn process_data(source_path: &Path, target_path: &Path, validate: bool, quality: u8, mode: ProcessingMode, force: bool) -> Result<(ProcessingResult, Option<Metadata>)>
```

**When wrapping is necessary:**
```rust
pub fn process_complex_operation(
	source_path: &Path,
	target_path: &Path,
	options: &ProcessingOptions,
	callback: impl Fn(&str) -> Result<()>
) -> Result<ProcessingResult>
```

### Line Length
- Aim for **120 characters maximum** per line.
- Break longer lines at logical boundaries (after commas, operators, etc.).
- Use continuation indentation (one tab) for wrapped lines.

### When to Break Lines
**DO break lines when:**
- Line exceeds 120 characters.
- Readability is significantly improved.
- Complex nested expressions become hard to parse.

**DON'T break lines when:**
- A simple one-liner is perfectly readable.
- Forcing vertical alignment makes code harder to scan.
- Breaking adds no clarity.

**Examples:**

```rust
// Good - simple and clear.
let result = calculate_value(x, y, z);

// Bad - forced vertical for no benefit.
let result = calculate_value(
	x,
	y,
	z
);

// Good - long line broken for readability.
let r = quantize_channel(pixel[0] as i16, factor);
let g = quantize_channel(pixel[1] as i16, factor);
let b = quantize_channel(pixel[2] as i16, factor);
result_buffer.put_pixel(x, y, image::Rgba([r, g, b, pixel[3]]));

// Bad - unreadable single line (140+ chars).
result_buffer.put_pixel(x, y, image::Rgba([quantize_channel(pixel[0] as i16, factor), quantize_channel(pixel[1] as i16, factor), quantize_channel(pixel[2] as i16, factor), pixel[3]]));
```

**Principle:** Simplicity trumps symmetry. Break lines when it helps readability, not to satisfy an arbitrary pattern.

---

## Comments

### General Comment Style
- **Begin all comment sentences with a capital letter**.
- **End all comment sentences with a period**.
- Use complete sentences for clarity.

**Examples:**
```rust
// Calculate the size reduction percentage.
let reduction_pct = (1.0 - (new_size as f64 / original_size as f64)) * 100.0;

// Safety check to prevent infinite loops.
if length > 10_000_000
{
	break;
}
```

### Documentation Comments
Use `///` for public items (functions, structs, modules):

```rust
/// Processes a file using a combination of techniques.
///
/// If validation is enabled, performs additional checks before processing.
/// Otherwise, applies standard processing with the specified quality level.
/// 
/// # Arguments
///
/// * `source_path` - Path to the source file.
/// * `target_path` - Path where the processed file will be saved.
/// * `validate` - Whether to perform validation checks.
pub fn process_file(source_path: &Path, target_path: &Path, validate: bool) -> Result<ProcessingResult>
{
	// Implementation.
}
```

### Inline Comments
- Use `//` followed by a space for inline comments.
- Place inline comments on the same line only when they're brief and clarify specific values.
- When explaining code, place comments on their own line above the code.

**Good:**
```rust
let new_pixel =
[
	quantize_channel(old_pixel[0], factor),
	quantize_channel(old_pixel[1], factor),
	quantize_channel(old_pixel[2], factor),
	old_pixel[3].clamp(0, 255) as u8 // Keep alpha unchanged.
];

// Distribute error to neighboring pixels (Floyd-Steinberg pattern).
if x + 1 < width as usize
{
	for c in 0..3
	{
		working_buffer[y][x + 1][c] += (error[c] * 7) / 16;
	}
}
```

**Avoid:**
```rust
let x = 5; // Set x to 5.
```

---

## Imports and Module Organization

### Import Ordering
Organize imports in the following order, with blank lines between groups:

1. External crates (from dependencies)
2. Standard library (`std::`)
3. Local modules (`crate::`, `super::`, `self::`)

**Example:**
```rust
use anyhow::{anyhow, Result};
use clap::{Parser, ArgAction};
use rayon::prelude::*;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

mod files;
mod processor;
mod utils;

use files::{find_files_in_dir, prepare_file_list};
use processor::{process_file, ProcessingResult, Metadata};
```

### Module Declarations
Place module declarations before use statements:

```rust
mod files;
mod processor;

use files::FileInfo;
```

---

## Naming Conventions

### Variables and Functions
Use **snake_case**:

```rust
let source_path = PathBuf::from("./file.txt");
let total_size: u64 = 0;

fn calculate_statistics(data: &[u8], width: u32, height: u32) -> f64
{
	// Implementation.
}
```

### Types (Structs, Enums, Traits)
Use **PascalCase**:

```rust
struct ProcessingResult
{
	original_size: u64,
	new_size: u64,
}

enum ProcessingMode
{
	Fast,
	Balanced,
	Quality,
}
```

### Constants
Use **SCREAMING_SNAKE_CASE**:

```rust
const APP_NAME: &str = "Application";
const APP_VERSION: &str = "1.0.0";
const FILE_SIGNATURE: &[u8; 4] = b"FILE";
const MAX_BUFFER_SIZE: u64 = 10_000_000;
```

---

## Type Annotations and Declarations

### Explicit Type Annotations
Prefer explicit type annotations for:
- Struct fields (always).
- Function parameters (always).
- Return types (always).
- Local variables when the type isn't immediately obvious.

**Good:**
```rust
let original_size: u64 = fs::metadata(source_path)?.len();
let mut working_buffer: Vec<Vec<[i16; 4]>> = Vec::new();
```

**When type inference is clear, annotation is optional:**
```rust
let results = Arc::new(Mutex::new(Vec::new())); // Type is clear from context.
let source_data = fs::read(source_path)?; // Return type is obvious.
```

### Struct Field Initialization
Use field init shorthand when variable names match field names:

```rust
FileInfo
{
	source_path, // Instead of: source_path: source_path.
	target_path,
}
```

---

## Pattern Matching

### Match Expressions
- Opening brace on a new line.
- Each arm on its own line.
- Comma after each arm (including the last one).
- Use range patterns when appropriate.

**Example:**
```rust
let priority_desc = match args.priority
{
	1..=3 => "High priority - immediate processing",
	4..=7 => "Medium priority - normal processing",
	8..=10 => "Low priority - batch processing",
	_ => "Custom priority level",
};
```

### If Let Expressions
Use `if let` for single pattern matching:

```rust
if let Some(ref info) = prev_info
{
	println!("Previously processed at priority {}", info.priority);
}
```

---

## Error Handling

### Result Type
Use `anyhow::Result` for most error handling:

```rust
use anyhow::{anyhow, Result};

fn process_file(path: &Path) -> Result<ProcessingResult>
{
	let data = fs::read(path)
		.map_err(|e| anyhow!("Failed to read file: {}", e))?;
	
	// Processing logic.
	
	Ok(ProcessingResult { /* fields */ })
}
```

### Error Context
Add context to errors using `.map_err()` or `.context()`:

```rust
use anyhow::Context;

let data = fs::read(source_path)
	.context("Failed to read source file")?;

let parsed = parse_data(&data)
	.map_err(|e| anyhow!("Parse error at line {}: {}", line_num, e))?;
```

### Early Returns
Use early returns with `?` operator for cleaner code:

```rust
fn validate_and_process(path: &Path) -> Result<()>
{
	if !path.exists()
	{
		return Err(anyhow!("File does not exist"));
	}
	
	let metadata = fs::metadata(path)?;
	
	if metadata.len() > MAX_FILE_SIZE
	{
		return Err(anyhow!("File too large"));
	}
	
	// Main processing logic.
	Ok(())
}
```

---

## Iterators and Functional Programming

### Method Chaining
Chain iterator methods for clarity:

```rust
let valid_files: Vec<PathBuf> = read_dir(dir_path)?
	.filter_map(Result::ok)
	.filter(|entry| entry.file_type().is_ok())
	.filter(|entry| entry.file_type().unwrap().is_file())
	.map(|entry| entry.path())
	.collect();
```

### Complex Chains
Break complex chains with meaningful intermediate variables:

```rust
let entries = read_dir(dir_path)?
	.filter_map(Result::ok)
	.collect::<Vec<_>>();

let files = entries
	.into_iter()
	.filter(|entry| entry.file_type().is_ok())
	.filter(|entry| entry.file_type().unwrap().is_file())
	.collect::<Vec<_>>();

let file_info = files
	.into_iter()
	.map(|entry|
	{
		let source_path = entry.path().to_path_buf();
		let target_path = source_path.clone();
		
		FileInfo
		{
			source_path,
			target_path,
		}
	})
	.collect::<Vec<_>>();
```

### Collect with Turbofish
Use turbofish syntax when the target type needs to be explicit:

```rust
.collect::<Vec<_>>();
.collect::<HashSet<_>>();
```

---

## Spacing and Blank Lines

### Between Functions
Use **one blank line** between function definitions:

```rust
fn function_one() -> Result<()>
{
	// Implementation.
}

fn function_two() -> Result<()>
{
	// Implementation.
}
```

### Within Functions
Use blank lines to separate logical sections:

```rust
fn process_data() -> Result<()>
{
	// Section 1: Read data.
	let data = read_file()?;
	let size = data.len();
	
	// Section 2: Process data.
	let processed = transform(data)?;
	let optimized = optimize(processed)?;
	
	// Section 3: Write results.
	write_output(optimized)?;
	
	Ok(())
}
```

### Around Imports
Blank line after imports:

```rust
use std::fs;
use std::path::Path;

fn main()
{
	// Code.
}
```

---

## String Formatting

### Display for Paths
Use `.display()` when printing paths:

```rust
println!("Processing file: {}", source_path.display());
println!("  - {} -> {}", file.source_path.display(), file.target_path.display());
```

### Format Macro
Use `format!()` for string construction:

```rust
let message = format!("Processed: {} ({:.1}% reduction)", filename, reduction_pct);
```

### String Literals
Use double quotes for strings:

```rust
const APP_NAME: &str = "MyApplication";
let msg = "Processing complete.";
```

---

## Numeric Formatting

### Floating Point
Use appropriate precision for display:

```rust
format!("{:.1}%", reduction_pct); // One decimal place.
format!("{:.2} MB", size_mb); // Two decimal places.
```

### Integer Formatting
Use underscores for large number literals:

```rust
const MAX_SIZE: u64 = 10_000_000;
if length > 1_024_000
{
	// Handle large file.
}
```

---

## Closures

### Single-Line Closures
Keep on one line when simple:

```rust
.filter(|entry| entry.file_type().is_file())
.map(|x| x * 2)
```

### Multi-Line Closures
Use opening brace on new line:

```rust
.filter(|entry|
{
	entry.file_type().is_file() && 
	entry.path().extension().is_some()
})

.map(|pixel|
{
	let r = pixel[0];
	let g = pixel[1];
	let b = pixel[2];
	calculate_brightness(r, g, b)
})
```

---

## Specific Patterns

### Arc and Mutex
Use `Arc<Mutex<T>>` for shared mutable state across threads:

```rust
let results = Arc::new(Mutex::new(Vec::new()));
let errors = Arc::new(Mutex::new(Vec::new()));

// In parallel code.
results.lock().unwrap().push(result);
```

### Unwrapping Arc
Use `Arc::try_unwrap()` when taking ownership back:

```rust
let results = Arc::try_unwrap(results)
	.unwrap_or_else(|_| panic!("Failed to unwrap Arc"))
	.into_inner()
	.unwrap();
```

### Array/Slice Initialization
Multi-element arrays on separate lines when it improves readability:

```rust
const LOOKUP_TABLE: [[i16; 4]; 4] =
[
	[ 0,  8,  2, 10],
	[12,  4, 14,  6],
	[ 3, 11,  1,  9],
	[15,  7, 13,  5],
];
```

---

## Testing Conventions

### Test Functions
- Name tests descriptively with `test_` prefix.
- Use `assert!`, `assert_eq!`, `assert!(matches!(...))` for assertions.

**Example:**
```rust
#[test]
fn test_data_validation()
{
	let mut data = vec![1, 2, 3, 4, 5];
	
	let result = validate_data(&data);
	
	assert!(result.is_ok());
	assert_eq!(result.unwrap(), true);
}

#[test]
fn test_mode_selection()
{
	let config = Configuration
	{
		priority: 5,
		quality: 85,
		mode: ProcessingMode::Balanced,
	};
	
	assert!(matches!(select_mode(&config), ProcessingMode::Balanced));
}
```

---

## Anti-Patterns to Avoid

### Don't Use Unnecessary Cloning
Prefer borrowing over cloning when possible:

**Avoid:**
```rust
fn process(data: Vec<u8>) -> Vec<u8> // Takes ownership, forces cloning at call site.
```

**Better:**
```rust
fn process(data: &[u8]) -> Vec<u8> // Borrows, no cloning needed.
```

### Don't Use `unwrap()` in Production Code
Use proper error handling instead:

**Avoid:**
```rust
let data = fs::read(path).unwrap(); // Panics on error.
```

**Better:**
```rust
let data = fs::read(path)
	.map_err(|e| anyhow!("Failed to read file: {}", e))?;
```

### Don't Write Overly Nested Code
Extract complex logic into separate functions:

**Avoid:**
```rust
if condition1
{
	if condition2
	{
		if condition3
		{
			// Deeply nested logic.
		}
	}
}
```

**Better:**
```rust
if !condition1
{
	return early();
}

if !condition2
{
	return early();
}

// Main logic at top level.
```

---

## Quick Reference Checklist

When writing or reviewing code, ensure:

- [ ] Using tabs for indentation.
- [ ] Opening braces/brackets on new lines for functions, structs, enums, blocks, arrays.
- [ ] Function parameters on single line (unless very long).
- [ ] Comments start with capital letter and end with period.
- [ ] Using snake_case for variables/functions.
- [ ] Using PascalCase for types.
- [ ] Using SCREAMING_SNAKE_CASE for constants.
- [ ] Explicit type annotations on struct fields and function signatures.
- [ ] Proper error handling with `anyhow::Result`.
- [ ] Meaningful error messages with context.
- [ ] One blank line between functions.
- [ ] Blank lines to separate logical sections.
- [ ] Using `.display()` for path formatting.
- [ ] Import organization: external -> std -> local.

---

## Summary

The coding style emphasizes:
1. **Readability** - Code should be self-documenting and easy to understand.
2. **Consistency** - Use the same patterns throughout the codebase.
3. **Explicitness** - Prefer explicit types and error handling.
4. **Maintainability** - Write code that's easy to modify and debug.
