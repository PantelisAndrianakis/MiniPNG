# Coding Style Guide

This document describes the Rust-specific coding conventions for this project.

**⚠️ RUST PHILOSOPHY:** We use Rust for systems programming, not functional programming. Enums over traits, pattern matching over chains, explicitness over cleverness.

---

## Core Principles

These are the core principles that define how we write code.

### 1. TYPE INFERENCE - CONTROLLED USE

Type inference is **tightly controlled** but not forbidden.

**Core Principle:** Code must be understandable without IDE assistance. The reader is more important than the writer.

**Allowed - Type is obvious from right-hand side:**
```rust
// ALLOWED - Constructor/initializer visible.
let buffer = Vec::<u8>::new();
let players = Vec::<Player>::with_capacity(100);

// ALLOWED - Generic type parameters make intent clear.
let map = HashMap::<String, Player>::new();
```

**Forbidden - Meaning is hidden:**
```rust
// WRONG - Return type unclear.
let result = calculate_value(x, y, z);
let data = get_data();

// CORRECT - Explicit types are self-documenting.
let result: i32 = calculate_value(x, y, z);
let data: Vec<u8> = get_data();
```

**Rule:** If understanding the type requires jumping to a definition, inference is forbidden.

**Why?** Code must be understandable without IDE assistance. The reader is more important than the writer.

### 2. SINGLE-LINE CODE - NO WRAPPING

**Code must fit in the reader's working memory. If it does not fit on one line, it does not fit in the head either.**

Control flow, conditions, and signatures must stay on single lines. This enforces **locality of understanding**: all required information must be visible in one visual frame.

```rust
// GOOD - all parameters visible, even if line is long.
fn process_data(source_path: &Path, target_path: &Path, validate: bool, quality: u8, mode: ProcessingMode) -> Result<()>
{
	// You can see everything. No hidden coupling. No indirection.
}

// WRONG - wrapping hides complexity.
fn process_data(
	source_path: &Path,
	target_path: &Path,
	validate: bool
) -> Result<()>
{
	// Now you have to scan vertically. Context is distributed.
}

// ALSO WRONG - abstraction hides parameters.
struct ProcessingConfig { /* ... */ }
fn process_data(config: &ProcessingConfig) -> Result<()>
{
	// Now you can't see what the function needs.
	// Coupling is hidden. Debugging is harder.
}

// CORRECT - condition visible.
if cursor[1] == '!' && cursor[2] == '[' && cursor[3] == 'C' && cursor[4] == 'D'
{
	handle_cdata();
}
```

**Why single-line?**
- **Visibility over abstraction** - You can see all parameters/conditions directly. No indirection. No hidden coupling.
- Your brain has ~7±2 working memory slots. Single-line keeps everything in one frame.
- Wrapping distributes complexity vertically - makes you scan and reconstruct context
- Creating parameter structs to "fix" long lines makes things WORSE: hidden coupling, loss of transparency, harder debugging
- A long single line is honest. It shows the real complexity. That's good.

**Don't wrap. Don't hide. If it's long, it's long. That's the truth.**

**Exception - Error handling chains may wrap when linear:**
```rust
// Allowed - error chain is linear and obvious.
let data: Vec<u8> = fs::read(path)
	.map_err(|e| anyhow!("Failed to read: {}", e))?;
```

### 3. ALLMAN BRACES - ALWAYS ON NEW LINE
Opening braces `{` ALWAYS go on a new line. No exceptions (except single-line closures).

```rust
// WRONG.
if condition {
	do_something();
}

// CORRECT.
if condition
{
	do_something();
}
```

**Why?** Visual symmetry makes code easier to scan and spot errors.

### 4. TABS FOR INDENTATION - NOT SPACES
Use tabs, period. Configure your editor properly.

Why? Because a single tab character is the true, unambiguous representation of a single indentation level. Spaces are a visual approximation; tabs are the logical unit.

### 5. COMPLETE SENTENCES IN COMMENTS
Comments start with capital letter, end with period.

```rust
// WRONG.
// calculate average value

// CORRECT.
// Calculate the average value.
```

**Why?** Professional code looks professional. We're not writing text messages.

---

## Rust-Specific Alignment Rules

**⚠️ CRITICAL:** These rules ensure Rust is used as a systems language, not a functional language.

### Prefer Enums With Data Over Traits

Use enums with associated data instead of inheritance or trait objects.

```rust
// CORRECT - Enum with data.
enum Packet
{
	Login(LoginData),
	Move(MoveData),
	Chat(String),
}

fn handle_packet(packet: Packet)
{
	match packet
	{
		Packet::Login(data) => process_login(data),
		Packet::Move(data) => process_move(data),
		Packet::Chat(msg) => process_chat(msg),
	}
}

// WRONG - Trait objects and dynamic dispatch.
trait Packet
{
	fn process(&self);
}

fn handle_packet(packet: &dyn Packet) // Forbidden in hot paths.
{
	packet.process();
}
```

This replaces:
- Polymorphism → Static enum matching
- Virtual dispatch → Direct branching
- Downcasting → Pattern matching
- Heap allocation → Stack allocation

### Prefer Pattern Matching Over If Chains

Use `match` for 3+ comparisons instead of `if/else` chains.

```rust
// CORRECT - Pattern matching.
let status: &str = match priority
{
	1..=3 => "High priority",
	4..=7 => "Medium priority",
	8..=10 => "Low priority",
	_ => "Custom priority",
};

// WRONG - If chain for multiple cases.
let status: &str = if priority >= 1 && priority <= 3
{
	"High priority"
}
else if priority >= 4 && priority <= 7
{
	"Medium priority"
}
else if priority >= 8 && priority <= 10
{
	"Low priority"
}
else
{
	"Custom priority"
};
```

### Iterators: Traversal Only - No Pipelines

Iterators are allowed only for simple traversal, not for chained pipelines.

**Allowed - Simple traversal:**
```rust
for file in files.iter()
{
	process(file);
}

for (index, value) in data.iter().enumerate()
{
	println!("{}: {}", index, value);
}
```

**Forbidden - Chained iterator pipelines:**
```rust
// WRONG - Hides control flow, allocations, and cost.
let results: Vec<_> = files
	.iter()
	.filter(|f| f.size > 1000)
	.map(|f| process(f))
	.collect();
```

**Correct alternative - Explicit loops:**
```rust
// CORRECT - Control flow and allocations are visible.
let mut results: Vec<ProcessedFile> = Vec::new();
results.reserve(files.len());

for file in files.iter()
{
	if file.size > 1000
	{
		let processed: ProcessedFile = process(file);
		results.push(processed);
	}
}
```

### Small Zero-Cost Combinators Are Allowed

Pure, obvious helpers that don't hide semantics are allowed.

```rust
// Allowed - obvious, zero-cost.
let size: usize = data.len().max(1);
let value: i32 = option.unwrap_or(default);
let clamped: i32 = value.clamp(min, max);
```

### Avoid Trait-Heavy Designs

Trait objects and dynamic dispatch are forbidden in hot paths.

Use traits only when:
- Modeling true external polymorphism (e.g., plugin systems).
- Not performance-critical (cold code).
- Behavior cannot be expressed with enums.

```rust
// CORRECT - Enum for known variants.
enum Storage
{
	File(FileStorage),
	Memory(MemoryStorage),
	Network(NetworkStorage),
}

// WRONG - Trait object in hot path.
fn process(storage: &dyn Storage) // Forbidden if hot.
{
	storage.read();
}
```

---

## Data-Oriented Design

**Memory layout and cache behavior matter more than syntax.**

### Prefer Contiguous Memory

Use `Vec`, slices, and arrays. Avoid deep object graphs and pointer chasing.

```rust
// CORRECT - Contiguous memory, cache-friendly.
struct PlayerData
{
	positions: Vec<Position>,
	healths: Vec<i32>,
	levels: Vec<u8>,
}

// WRONG - Pointer-heavy, cache-hostile.
struct Player
{
	position: Box<Position>,
	inventory: Box<Vec<Item>>,
	stats: Box<Stats>,
}
```

### Flat Data Structures

Avoid excessive indirection and nesting.

```rust
// CORRECT - Flat, direct access.
struct GameState
{
	players: Vec<Player>,
	npcs: Vec<Npc>,
	items: Vec<Item>,
}

// WRONG - Deep nesting.
struct GameState
{
	world: Box<World>,
}

struct World
{
	zones: Vec<Box<Zone>>,
}

struct Zone
{
	entities: Vec<Box<Entity>>,
}
```

### SoA vs AoS in Hot Paths

For hot loops iterating a single field, consider Structure of Arrays.

```rust
// AoS - good for general access.
struct Player
{
	x: f32,
	y: f32,
	health: i32,
	level: u8,
}

let players: Vec<Player> = Vec::new();

// SoA - better for hot loops accessing one field.
struct Players
{
	xs: Vec<f32>,
	ys: Vec<f32>,
	healths: Vec<i32>,
	levels: Vec<u8>,
}

// Hot loop only needs positions - SoA wins.
for i in 0..players.xs.len()
{
	update_position(players.xs[i], players.ys[i]);
}
```

### Avoid Heap Allocation in Loops

Pre-allocate outside loops. Reuse buffers.

```rust
// WRONG - allocates every iteration.
for file in &files
{
	let buffer: Vec<u8> = vec![0; 1024]; // Bad.
	process(file, &buffer);
}

// CORRECT - allocate once.
let mut buffer: Vec<u8> = vec![0; 1024];

for file in &files
{
	process(file, &buffer);
}
```

**This grounds all other rules:** Enums over traits, explicit loops, no pipelines—all serve cache-friendly, contiguous, predictable memory access.

---

## Naming Conventions

Get the names right or the code gets rejected.

### Variables and Functions
Use **snake_case**:

```rust
let total_count: i32 = 0;
let file_name: String = String::from("data.txt");

fn process_file(path: &Path) -> Result<()>
{
	let buffer_size: usize = 1024;
}
```

### Types (Structs, Enums, Traits)
Use **PascalCase**:

```rust
struct FileProcessor
{
	file_path: PathBuf,
	buffer_size: usize,
}

enum ProcessingMode
{
	Fast,
	Balanced,
	Quality,
}

trait DataProcessor
{
	fn process(&self, data: &[u8]) -> Result<Vec<u8>>;
}
```

### Constants
Use **SCREAMING_SNAKE_CASE**:

```rust
const MAX_BUFFER_SIZE: usize = 10_000_000;
const APP_NAME: &str = "Application";
const DEFAULT_TIMEOUT: u64 = 30;
```

---

## Formatting Rules

### Indentation
- **Tabs only** - no spaces for indentation.
- One tab per level.
- Continuation lines get one additional tab.

### Braces Placement
Opening brace `{` on new line for:
- Functions
- Structs, enums, traits, impl blocks
- If/else blocks
- Match expressions
- Loops (for, while)
- Multi-line closures
- Multi-line array/slice initializations

**Examples:**

```rust
fn process_file(path: &Path) -> Result<ProcessingResult>
{
	// Function body.
}

struct FileProcessor
{
	file_path: PathBuf,
	buffer_size: usize,
}

if condition
{
	// If body.
}

for file in &files
{
	// Loop body.
}

let status: &str = match priority
{
	1..=3 => "High",
	4..=7 => "Medium",
	_ => "Low",
};
```

**Exception - Single-line closures:**

```rust
// Simple closure for sorting.
files.sort_by(|a, b| a.len().cmp(&b.len()));
```

### Function Signatures
**All parameters on a single line:**

```rust
pub fn process_data(source_path: &Path, target_path: &Path, validate: bool, quality: u8, mode: ProcessingMode) -> Result<ProcessingResult>
{
	// Implementation.
}
```

If it's too long, you're doing too much - refactor it.

### Spacing Rules

**Between functions - one blank line:**
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

**Within functions - blank lines separate logic:**
```rust
fn process_data() -> Result<()>
{
	// Section 1: Read data.
	let data: Vec<u8> = read_file()?;
	let size: usize = data.len();
	
	// Section 2: Process data.
	let processed: ProcessedData = transform(data)?;
	let optimized: OptimizedData = optimize(processed)?;
	
	// Section 3: Write results.
	write_output(optimized)?;
	
	Ok(())
}
```

**Between independent control structures - blank lines:**
```rust
// CORRECT - blank lines separate independent checks.
if p != usize::MAX
{
	key = attr.key[(p + 1)..].to_string();
}

if self.params.dump_id_attributes_name
{
	out_attr.push_str(&format!("{}={}", key, attr.val));
}
```

**Related if/else stays together - no blank lines:**
```rust
if condition1
{
	do_something();
}
else if condition2
{
	do_something_else();
}
else
{
	do_default();
}
```

**Critical spacing rules:**
- **Never more than one blank line** anywhere.
- **No trailing spaces** at end of lines.
- **No excessive spacing** like `if x == 0   `.

---

## Control Flow

### When to Use If vs Match

**Use if/else for 1-2 comparisons:**
```rust
if priority == 1
{
	process_high_priority();
}
else
{
	process_normal_priority();
}
```

**Use match for 3+ comparisons:**
```rust
let status: &str = match priority
{
	1 | 2 | 3 => "High priority",
	4 | 5 => "Medium priority",
	_ => "Low priority",
};
```

### If-Else Statements
- **Always use braces** - even for single statements.
- **Keep conditions on single line** - no wrapping.

```rust
// Good - simple condition on single line.
if cursor[1] == '!' && cursor[2] == '[' && cursor[3] == 'C' && cursor[4] == 'D'
{
	handle_cdata();
}

// Good - complex condition still on single line (even if long).
if player.is_alive() && !player.is_stunned() && player.has_mana(50) && target.is_visible() && target.is_in_range(&player, 1200)
{
	cast_spell();
}

// WRONG - no braces.
if condition
	do_something();
```

### Match Expressions
- **Opening brace on new line**.
- **Each arm on its own line**.
- **Comma after each arm** (including the last one).
- **Always handle all cases** (use `_` for default).

```rust
let priority_desc: &str = match args.priority
{
	1..=3 => "High priority - immediate processing",
	4..=7 => "Medium priority - normal processing",
	8..=10 => "Low priority - batch processing",
	_ => "Custom priority level",
};
```

### Loops
Use the appropriate loop type:

```rust
// For loops - known iteration.
for i in 0..count
{
	process_item(i);
}

// For loops - iterating collections.
for file in &files
{
	process_file(file);
}

// While loops - condition-based iteration.
while !queue.is_empty()
{
	let item: String = queue.pop_front().unwrap();
	process_item(&item);
}
```

---

## Type Declarations

### The Rule: Controlled Type Inference

Type inference is allowed when the type is obvious, forbidden when meaning is hidden.

**Use inference when type is obvious:**
```rust
// Good - generic parameters make type explicit.
let buffer = Vec::<u8>::new();
let players = Vec::<Player>::with_capacity(100);
let map = HashMap::<String, i32>::new();

// Good - constructor is explicit.
let data = String::from("hello");
```

**Do NOT use inference when type is unclear:**
```rust
// WRONG - type inference hides information.
let result = calculate_value(x, y, z);
let file = open_file(path);
let data = get_data();

// CORRECT - explicit types are self-documenting.
let result: i32 = calculate_value(x, y, z);
let file: File = open_file(path)?;
let data: Vec<u8> = get_data();
```

### Struct Fields
Always explicit:

```rust
pub struct FileProcessor
{
	pub source_path: PathBuf,
	pub target_path: PathBuf,
	pub buffer_size: usize,
	pub mode: ProcessingMode,
}
```

### Function Signatures
Always explicit for parameters and return types:

```rust
pub fn process_file(source_path: &Path, target_path: &Path, validate: bool) -> Result<ProcessingResult>
{
	// Implementation.
}
```

### Local Variables
Always explicit:

```rust
let original_size: u64 = fs::metadata(source_path)?.len();
let mut working_buffer: Vec<Vec<[i16; 4]>> = Vec::new();
let results: Arc<Mutex<Vec<ProcessingResult>>> = Arc::new(Mutex::new(Vec::new()));
let source_data: Vec<u8> = fs::read(source_path)?;
```

---

## Borrowing and Ownership

**Borrowing is fundamental to Rust.** It's not something to avoid - it's the primary way to write safe, efficient Rust code.

### Use Borrowing - Don't Clone Everything

Borrow when you don't need ownership. This is the Rust way.

```rust
// CORRECT - borrow when you don't need ownership.
fn process_data(data: &[u8]) -> usize
{
	data.len() // Just reading, no ownership needed.
}

let data: Vec<u8> = vec![1, 2, 3];
let length: usize = process_data(&data); // Borrow it.
// data is still usable here!

// WRONG - taking ownership when borrowing would work.
fn process_data(data: Vec<u8>) -> usize // Takes ownership unnecessarily.
{
	data.len()
}

let data: Vec<u8> = vec![1, 2, 3];
let length: usize = process_data(data); // Moved - data is gone!
// Can't use data anymore - it was moved!
```

### When to Borrow vs Take Ownership

**Borrow (&T or &mut T) when:**
- You just need to read the data
- You need to modify but not consume
- You want the caller to keep using it afterward

**Take ownership (T) when:**
- The function consumes/destroys the value
- The function stores the value long-term
- The function transforms and returns it

```rust
// Borrow - function just reads.
fn get_first(items: &Vec<String>) -> Option<&String>
{
	items.first()
}

// Borrow mutably - function modifies.
fn add_item(items: &mut Vec<String>, item: String)
{
	items.push(item);
}

// Take ownership - function consumes.
fn process_and_save(data: Vec<u8>) -> Result<()>
{
	save_to_disk(data)?; // data is consumed here.
	Ok(())
}
```

### Prefer & and &mut Over Explicit Iterator Methods

Use implicit borrowing syntax in for loops:

```rust
// CORRECT - implicit borrowing.
for item in &files
{
	process(item);
}

// CORRECT - implicit mutable borrowing.
for item in &mut files
{
	modify(item);
}

// WRONG - explicit .iter() call.
for item in files.iter()
{
	process(item);
}

// WRONG - explicit .iter_mut().
for item in files.iter_mut()
{
	modify(item);
}
```

**Principle:** Prefer borrowing. Only take ownership when you actually need it.

---

## Comments

### General Rules
- **Start with capital letter**.
- **End with period**.
- **Use complete sentences**.
- **Use `//` for single-line** comments.
- **Use `/* */` for multi-line** comments.

```rust
// Calculate the size reduction percentage.
let reduction_pct: f64 = (1.0 - (new_size as f64 / original_size as f64)) * 100.0;

/*
 * This is a multi-line comment explaining
 * a complex algorithm or process flow.
 */
```

### Documentation Comments
Use `///` for public items:

```rust
/// Processes a file using the specified options.
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
Only when clarifying non-obvious code:

```rust
let new_pixel: [u8; 4] =
[
	quantize_channel(old_pixel[0], factor),
	quantize_channel(old_pixel[1], factor),
	quantize_channel(old_pixel[2], factor),
	old_pixel[3] // Keep alpha unchanged.
];
```

**Don't state the obvious:**
```rust
// WRONG - obvious comment.
let x: i32 = 5; // Set x to 5.

// CORRECT - only comment when adding value.
let retry_count: i32 = 5; // Empirically determined optimal retry count.
```

---

## Imports and Modules

### Import Ordering
Three groups, blank line between each:

1. External crates (from dependencies)
2. Standard library (`std::`)
3. Local modules (`crate::`, `super::`, `self::`)

```rust
use anyhow::{anyhow, Result};
use clap::{Parser, ArgAction};

use std::fs;
use std::path::{Path, PathBuf};

mod files;
mod processor;

use files::find_files_in_dir;
use processor::ProcessingResult;
```

---

## Structs and Implementations

### Struct Definition
```rust
pub struct FileProcessor
{
	source_path: PathBuf,
	target_path: PathBuf,
	buffer_size: usize,
	mode: ProcessingMode,
}
```

### Impl Blocks
```rust
impl FileProcessor
{
	pub fn new(source: PathBuf, target: PathBuf) -> Self
	{
		Self
		{
			source_path: source,
			target_path: target,
			buffer_size: 1024,
			mode: ProcessingMode::Balanced,
		}
	}
	
	pub fn process(&self) -> Result<ProcessingResult>
	{
		// Implementation.
	}
	
	pub fn set_mode(&mut self, mode: ProcessingMode)
	{
		self.mode = mode;
	}
}
```

### Field Init Shorthand
Use when variable names match field names:

```rust
let source_path: PathBuf = get_source();
let target_path: PathBuf = get_target();

let processor: FileProcessor = FileProcessor
{
	source_path, // Shorthand.
	target_path, // Shorthand.
	buffer_size: 1024,
	mode: ProcessingMode::Fast,
};
```

---

## Error Handling

### Use Result Type
Prefer `anyhow::Result` for most error handling:

```rust
use anyhow::{anyhow, Result};

pub fn process_file(path: &Path) -> Result<ProcessingResult>
{
	if !path.exists()
	{
		return Err(anyhow!("File does not exist: {}", path.display()));
	}
	
	let metadata: Metadata = fs::metadata(path)?;
	
	if metadata.len() > MAX_FILE_SIZE
	{
		return Err(anyhow!("File too large: {} bytes", metadata.len()));
	}
	
	// Main processing logic.
	Ok(ProcessingResult::default())
}
```

### Error Propagation
Use `?` operator for error propagation:

```rust
pub fn read_and_process(path: &Path) -> Result<ProcessingResult>
{
	let data: Vec<u8> = fs::read(path)
		.map_err(|e| anyhow!("Failed to read file: {}", e))?;
	
	let result: ProcessingResult = process_data(&data)?;
	
	Ok(result)
}
```

### Meaningful Error Messages
```rust
// Good - context-rich error.
fs::read(path)
	.map_err(|e| anyhow!("Failed to read {}: {}", path.display(), e))?;

// Bad - generic error.
fs::read(path)?; // What file? Why did it fail?
```

---

## String Handling

### String vs &str
Use the appropriate string type:

```rust
// Owned string - when you need to modify or own the data.
let mut owned: String = String::from("Hello");
owned.push_str(" World");

// String slice - when borrowing is sufficient.
fn process_text(text: &str)
{
	println!("{}", text);
}

// Convert between them.
let owned: String = String::from("text");
let borrowed: &str = &owned;
```

### String Formatting
Use `.display()` for paths:

```rust
println!("Processing file: {}", source_path.display());
```

Use `format!()` for string construction:

```rust
let message: String = format!("Processed: {} ({:.1}% reduction)", filename, reduction_pct);
```

---

## Numeric Formatting

### Digit Separators
Use underscores for digit separators:

```rust
let large_number: i32 = 1_000_000;
let very_large: i64 = 1_234_567_890;
let precise: f64 = 3.141_592_653;

const BUFFER_SIZE: usize = 10_000_000;
```

### Hexadecimal and Binary
```rust
let hex_value: u32 = 0xFF_FF_FF;
let binary_value: u8 = 0b1111_0000;
```

---

## Iterator Reference (Not Used in This Project)

**These constructs exist in Rust but are intentionally avoided in this project.**

Rust's iterator methods (`map`, `filter`, `fold`, `any`, `all`, `for_each`) hide control flow and allocation cost. They are documented here only for reference when reading external code.

**We do not use:**
- Chained iterator pipelines
- `fold` / `reduce`
- `for_each`
- Complex iterator chains

**Instead, we use explicit loops** that make control flow and allocations visible:

```rust
// NOT THIS (pipeline hides control flow):
let results: Vec<_> = files.iter()
	.filter(|f| f.size > 1000)
	.map(|f| process(f))
	.collect();

// THIS (explicit loop shows control flow):
let mut results: Vec<ProcessedFile> = Vec::new();
results.reserve(files.len());

for file in files.iter()
{
	if file.size > 1000
	{
		let processed: ProcessedFile = process(file);
		results.push(processed);
	}
}
```

**Exception:** Simple, zero-cost combinators are allowed:
```rust
let size: usize = data.len().max(1);
let value: i32 = option.unwrap_or(default);
let clamped: i32 = value.clamp(min, max);
```

---

## Closures

### Closure Syntax
Use closures only when required by APIs (sorting, callbacks, threading).

```rust
// Simple closure for API requirements.
let add_one = |x: i32| -> i32 { x + 1 };
let result: i32 = add_one(5);

// Multi-statement closure.
let process = |data: &[u8]| -> Result<Vec<u8>>
{
	let processed: Vec<u8> = transform(data)?;
	let optimized: Vec<u8> = optimize(&processed)?;
	Ok(optimized)
};
```

### Closure Captures
```rust
let threshold: i32 = 10;

// Borrowing in closure (API requirement).
let add_threshold = |x: i32| -> i32 { x + threshold };
let result: i32 = add_threshold(5);

// Moving ownership into closure.
let buffer: Vec<u8> = vec![1, 2, 3];
let processor = move ||
{
	process_buffer(&buffer);
};
```

**Avoid functional patterns:** Do not use closures to replace explicit loops unless required by the API.

---

## Pattern Matching

### Match with Enums
```rust
enum ProcessingMode
{
	Fast,
	Balanced { quality: u8 },
	Quality { precision: f64, passes: u32 },
}

let result: String = match mode
{
	ProcessingMode::Fast => "Fast mode".to_string(),
	ProcessingMode::Balanced { quality } => format!("Balanced: q={}", quality),
	ProcessingMode::Quality { precision, passes } => format!("Quality: p={}, passes={}", precision, passes),
};
```

### If Let
```rust
// If let for single pattern.
if let Some(value) = optional_value
{
	process(value);
}

// If let with guard.
if let Some(value) = optional_value && value > 10
{
	process(value);
}
```

### Destructuring
```rust
// Tuple destructuring.
let (x, y): (i32, i32) = (10, 20);

// Struct destructuring.
let FileInfo { path, size } = file_info;

// Enum destructuring in match.
match result
{
	Ok(value) => println!("Success: {}", value),
	Err(e) => eprintln!("Error: {}", e),
}
```

---

## Specific Patterns

### Builder Pattern
```rust
pub struct ProcessorBuilder
{
	source: Option<PathBuf>,
	target: Option<PathBuf>,
	buffer_size: usize,
	mode: ProcessingMode,
}

impl ProcessorBuilder
{
	pub fn new() -> Self
	{
		Self
		{
			source: None,
			target: None,
			buffer_size: 1024,
			mode: ProcessingMode::Balanced,
		}
	}
	
	pub fn source(mut self, path: PathBuf) -> Self
	{
		self.source = Some(path);
		self
	}
	
	pub fn target(mut self, path: PathBuf) -> Self
	{
		self.target = Some(path);
		self
	}
	
	pub fn build(self) -> Result<FileProcessor>
	{
		Ok(FileProcessor
		{
			source_path: self.source.ok_or_else(|| anyhow!("Source required"))?,
			target_path: self.target.ok_or_else(|| anyhow!("Target required"))?,
			buffer_size: self.buffer_size,
			mode: self.mode,
		})
	}
}

// Usage.
let processor: FileProcessor = ProcessorBuilder::new()
	.source(PathBuf::from("input.txt"))
	.target(PathBuf::from("output.txt"))
	.build()?;
```

### Newtype Pattern
```rust
// Wrap primitive types for type safety.
pub struct UserId(u64);
pub struct ProductId(u64);

impl UserId
{
	pub fn new(id: u64) -> Self
	{
		Self(id)
	}
	
	pub fn value(&self) -> u64
	{
		self.0
	}
}

// Now you can't accidentally mix up IDs.
fn get_user(id: UserId) -> User
{
	// Implementation.
}
```

---

## Testing Conventions

### Unit Tests
```rust
#[cfg(test)]
mod tests
{
	use super::*;
	
	#[test]
	fn test_data_validation()
	{
		let mut data: Vec<i32> = vec![1, 2, 3, 4, 5];
		
		let result: Result<bool> = validate_data(&data);
		
		assert!(result.is_ok());
		assert_eq!(result.unwrap(), true);
	}
	
	#[test]
	fn test_file_processing()
	{
		let temp_file: PathBuf = create_temp_file();
		
		let result: Result<ProcessingResult> = process_file(&temp_file);
		
		assert!(result.is_ok());
		assert!(result.unwrap().reduction_percent > 0.0);
	}
}
```

### Integration Tests
Create in `tests/` directory:

```rust
// tests/integration_test.rs
use my_crate::FileProcessor;
use std::path::PathBuf;

#[test]
fn test_end_to_end_processing()
{
	let source: PathBuf = PathBuf::from("test_data/input.txt");
	let target: PathBuf = PathBuf::from("test_data/output.txt");
	
	let processor: FileProcessor = FileProcessor::new(source, target);
	let result: Result<ProcessingResult> = processor.process();
	
	assert!(result.is_ok());
}
```

---

## Performance Considerations

### Avoid Cloning
```rust
// Bad - unnecessary cloning.
fn process(data: Vec<u8>) -> Vec<u8> // Takes ownership.
{
	// Forces caller to clone.
}

// Good - borrow when possible.
fn process(data: &[u8]) -> Vec<u8> // Borrows.
{
	// No cloning needed.
}
```

### Use References
```rust
// Good - pass by reference.
fn process_file(path: &Path) -> Result<ProcessingResult>
{
	// Borrow the path.
}

// Avoid - pass by value when unnecessary.
fn process_file(path: PathBuf) -> Result<ProcessingResult>
{
	// Takes ownership - caller must give up the PathBuf.
}
```

### Reserve Capacity
```rust
let mut results: Vec<ProcessingResult> = Vec::new();
results.reserve(1000); // Pre-allocate.

for file in &files
{
	results.push(process_file(file)?);
}
```

---

## Anti-Patterns to Avoid

### ❌ Don't Omit Type Annotations When Type Is Unclear
```rust
// WRONG - unclear return type.
let result = calculate_value(x, y, z);

// CORRECT - explicit type.
let result: i32 = calculate_value(x, y, z);

// ALLOWED - obvious from turbofish.
let buffer = Vec::<u8>::new();
```

### ❌ Don't Declare Multiple Variables on Same Line
```rust
// WRONG - hard to see types.
let (a, b) = (0, 0);
let x = 1; let y = 2; let z;

// CORRECT - one per line.
let a: i32 = 0;
let b: i32 = 0;
let x: i32 = 1;
let y: i32 = 2;
let z: i32 = 0;
```

**Exception:** Tuple destructuring for semantically related values:
```rust
let (width, height): (u32, u32) = (800, 600); // OK - paired values.
```

### ❌ Don't Over-Engineer - Avoid Single-Use Code
```rust
// WRONG - constant used only once.
const BUFFER_SIZE: usize = 1024;
let buffer: Vec<u8> = vec![0; BUFFER_SIZE];

// CORRECT - inline it.
let buffer: Vec<u8> = vec![0; 1024];

// WRONG - helper function called only once.
fn print_separator()
{
	println!("---");
}

// CORRECT - inline it.
println!("---");
```

**Exception:** Create abstractions when used multiple times, improves clarity significantly, or likely to change.

### ❌ Don't Use unwrap() in Production Code
```rust
// WRONG - panics on error.
let data: Vec<u8> = fs::read(path).unwrap();

// CORRECT - handle errors.
let data: Vec<u8> = fs::read(path)
	.map_err(|e| anyhow!("Failed to read file: {}", e))?;
```

### ❌ Don't Clone Unnecessarily
```rust
// WRONG - unnecessary cloning.
fn process(data: Vec<u8>) -> Vec<u8> // Takes ownership.
{
	// Forces caller to clone.
}

// CORRECT - borrow when possible.
fn process(data: &[u8]) -> Vec<u8> // Borrows.
{
	// No cloning needed.
}
```

### ❌ Don't Use Iterator Pipelines (Use Explicit Loops)
```rust
// WRONG - hides control flow, allocations, and cost.
let results: Vec<_> = files
	.iter()
	.filter(|f| f.size > 1000)
	.map(|f| process(f))
	.collect();

// CORRECT - explicit control flow and allocations.
let mut results: Vec<ProcessedFile> = Vec::new();
results.reserve(files.len());

for file in files.iter()
{
	if file.size > 1000
	{
		let processed: ProcessedFile = process(file);
		results.push(processed);
	}
}
```

**Exception:** Simple zero-cost combinators like `.max()`, `.unwrap_or()`, `.clamp()` are allowed.

### ❌ Don't Write Deeply Nested Code
```rust
// WRONG - deeply nested.
if condition1
{
	if condition2
	{
		if condition3
		{
			// Too deep.
		}
	}
}

// CORRECT - early returns.
if !condition1
{
	return Err(anyhow!("Condition 1 failed"));
}

if !condition2
{
	return Err(anyhow!("Condition 2 failed"));
}

if !condition3
{
	return Err(anyhow!("Condition 3 failed"));
}

// Main logic at top level.
```

---

## Quick Reference Checklist

Before submitting code, verify:

- [ ] **Controlled type inference** - only when type is obvious from RHS
- [ ] **Explicit types when unclear** - I can see types without jumping to definitions
- [ ] **Enums over traits** - for known variants and hot paths
- [ ] **Pattern matching over if chains** - use match for 3+ cases
- [ ] **No iterator pipelines** - use explicit loops for traversal
- [ ] **Data-oriented design** - contiguous memory, flat structures, no deep nesting
- [ ] **Single-line code** - all control flow, conditions, signatures on single lines (no wrapping)
- [ ] **Allman braces** - opening `{` on new line
- [ ] **Tabs for indentation** - not spaces
- [ ] **Complete sentences in comments** - capital letter, period
- [ ] **snake_case** for variables/functions
- [ ] **PascalCase** for types (structs/enums/traits)
- [ ] **SCREAMING_SNAKE_CASE** for constants
- [ ] **One blank line** between functions
- [ ] **Never more than one blank line** anywhere
- [ ] **No trailing spaces**
- [ ] **One variable per line** - no `let (a, b) = (0, 0);` unless semantically paired
- [ ] **Proper error handling** with `anyhow::Result`
- [ ] **No unwrap()** in production code
- [ ] **Borrow instead of clone** when possible
- [ ] **Use `&` and `&mut`** instead of `.iter()` and `.iter_mut()` in for loops
- [ ] **Import order** correct (external, std, local)
- [ ] **.display()** for path formatting

---

## Summary

Remember these core principles:

1. **Systems, Not Functional** - Explicit loops over pipelines. Enums over traits. No hidden control flow.
2. **Data-Oriented** - Contiguous memory, flat structures, cache-friendly access patterns.
3. **Controlled Type Inference** - Use inference only when type is obvious from RHS.
4. **Pattern Matching** - Use match for 3+ cases, not if/else chains.
5. **Single-Line Code** - All control flow, conditions, signatures on single lines. Wrapping hides complexity instead of reducing it.
6. **Allman Braces** - Opening braces on new lines always.
7. **Complete Sentences** - Comments are documentation.
8. **Don't Over-Engineer** - YAGNI (You Ain't Gonna Need It).
9. **Borrow, Don't Clone** - Use references when possible.
10. **Handle Errors** - Use Result, not unwrap().
