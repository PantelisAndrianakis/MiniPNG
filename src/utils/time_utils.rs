/// Returns the current timestamp in ISO 8601 format.
pub fn get_iso8601_timestamp() -> String
{
	let now: std::time::SystemTime = std::time::SystemTime::now();
	let datetime: std::time::Duration = now.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default();
	let secs: u64 = datetime.as_secs();
	
	// Convert to UTC datetime components manually.
	let days_since_epoch: u64 = secs / 86400;
	let seconds_in_day: u64 = secs % 86400;
	
	let hours: u64 = seconds_in_day / 3600;
	let minutes: u64 = (seconds_in_day % 3600) / 60;
	let seconds: u64 = seconds_in_day % 60;
	
	// Calculate date components (year, month, day).
	// This uses a simplified algorithm to convert from Unix time to calendar date.
	let (year, month, day): (u32, u32, u32) = convert_days_to_date(days_since_epoch);
	
	format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", year, month, day, hours, minutes, seconds)
}

/// Convert days since Unix epoch (Jan 1, 1970) to (year, month, day).
fn convert_days_to_date(days_since_epoch: u64) -> (u32, u32, u32)
{
	// Base epoch year.
	let mut year: u64 = 1970;
	let mut days_remaining: u64 = days_since_epoch;
	
	// Account for leap years.
	loop
	{
		let days_in_year: u64 = if is_leap_year(year) { 366 } else { 365 };
		if days_remaining < days_in_year
		{
			break;
		}

		days_remaining -= days_in_year;
		year += 1;
	}
	
	// Determine month and day.
	let days_in_month: [u64; 12] =
	[
		31,
		if is_leap_year(year) { 29 } else { 28 },
		31, 30, 31, 30, 31, 31, 30, 31, 30, 31
	];
	
	let mut month: usize = 0;
	for (idx, &days) in days_in_month.iter().enumerate()
	{
		if days_remaining < days
		{
			month = idx + 1;
			break;
		}

		days_remaining -= days;
	}
	
	// Remaining days plus 1 gives us the day of month.
	let day: u64 = days_remaining + 1;
	
	(year as u32, month as u32, day as u32)
}

/// Check if a year is a leap year.
fn is_leap_year(year: u64) -> bool
{
	(year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Format ISO timestamp to human-readable format.
pub fn format_timestamp(iso_timestamp: &str) -> String
{
	// Parse ISO 8601 timestamp (e.g., "2026-02-06T20:15:30Z").
	// Convert to readable format (e.g., "2026-02-06 at 20:15").
	if let Some(dt_part) = iso_timestamp.split('T').next()
	{
		if let Some(time_part) = iso_timestamp.split('T').nth(1)
		{
			let time: &str = time_part.trim_end_matches('Z');
			let hm: String = time.split(':').take(2).collect::<Vec<_>>().join(":");
			return format!("{} at {}", dt_part, hm);
		}
	}
	iso_timestamp.to_string()
}
