use anyhow::{anyhow, Result};
use regex::Regex;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SubtitleEntry {
    pub index: u32,
    pub start_time: String,
    pub end_time: String,
    pub text: Vec<String>,
}

#[derive(Debug)]
pub struct SubtitleFile {
    pub entries: Vec<SubtitleEntry>,
}

impl SubtitleFile {
    /// Parse SRT file from file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        Self::from_content(&content)
    }

    /// Parse SRT content from string
    pub fn from_content(content: &str) -> Result<Self> {
        let mut entries = Vec::new();
        let blocks: Vec<&str> = content.split("\n\n").collect();

        for block in blocks {
            let block = block.trim();
            if block.is_empty() {
                continue;
            }

            if let Some(entry) = Self::parse_subtitle_block(block)? {
                entries.push(entry);
            }
        }

        if entries.is_empty() {
            return Err(anyhow!("No valid subtitle entries found"));
        }

        Ok(SubtitleFile { entries })
    }

    /// Parse a single subtitle block
    fn parse_subtitle_block(block: &str) -> Result<Option<SubtitleEntry>> {
        let lines: Vec<&str> = block.lines().collect();
        
        if lines.len() < 3 {
            return Ok(None); // Skip invalid blocks
        }

        // Parse index
        let index: u32 = lines[0].trim().parse()
            .map_err(|_| anyhow!("Invalid subtitle index: {}", lines[0]))?;

        // Parse timing line
        let timing_regex = Regex::new(r"^(\d{2}:\d{2}:\d{2},\d{3})\s*-->\s*(\d{2}:\d{2}:\d{2},\d{3})$")?;
        let timing_caps = timing_regex.captures(lines[1])
            .ok_or_else(|| anyhow!("Invalid timing format: {}", lines[1]))?;

        let start_time = timing_caps.get(1).unwrap().as_str().to_string();
        let end_time = timing_caps.get(2).unwrap().as_str().to_string();

        // Parse text lines
        let text: Vec<String> = lines[2..].iter().map(|s| s.to_string()).collect();

        Ok(Some(SubtitleEntry {
            index,
            start_time,
            end_time,
            text,
        }))
    }

    /// Convert subtitle file to string format
    pub fn to_string(&self) -> String {
        let mut result = String::new();

        for (i, entry) in self.entries.iter().enumerate() {
            if i > 0 {
                result.push_str("\n\n");
            }

            result.push_str(&format!("{}\n", entry.index));
            result.push_str(&format!("{} --> {}\n", entry.start_time, entry.end_time));
            
            for line in &entry.text {
                result.push_str(&format!("{}\n", line));
            }
        }

        result
    }

    /// Save subtitle file to path
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        fs::write(path, self.to_string())?;
        Ok(())
    }

    /// Apply framerate conversion to all timestamps
    pub fn convert_framerate(&mut self, from_fps: f32, to_fps: f32) -> Result<()> {
        let conversion_ratio = from_fps / to_fps;

        for entry in &mut self.entries {
            entry.start_time = convert_timestamp(&entry.start_time, conversion_ratio)?;
            entry.end_time = convert_timestamp(&entry.end_time, conversion_ratio)?;
        }

        Ok(())
    }

    /// Get all timing information for analysis
    pub fn get_timing_info(&self) -> Vec<(i32, i32)> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let start_ms = timestamp_to_milliseconds(&entry.start_time).ok()?;
                let end_ms = timestamp_to_milliseconds(&entry.end_time).ok()?;
                Some((start_ms, end_ms))
            })
            .collect()
    }

    /// Validate subtitle file integrity
    pub fn validate(&self) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        for (i, entry) in self.entries.iter().enumerate() {
            // Check timing validity
            if let (Ok(start), Ok(end)) = (
                timestamp_to_milliseconds(&entry.start_time),
                timestamp_to_milliseconds(&entry.end_time)
            ) {
                if start >= end {
                    warnings.push(format!("Entry {}: End time is not after start time", entry.index));
                }
                
                if end - start < 100 {
                    warnings.push(format!("Entry {}: Very short duration ({}ms)", entry.index, end - start));
                }
                
                if end - start > 10000 {
                    warnings.push(format!("Entry {}: Very long duration ({}ms)", entry.index, end - start));
                }
            } else {
                warnings.push(format!("Entry {}: Invalid timestamp format", entry.index));
            }

            // Check for empty text
            if entry.text.is_empty() || entry.text.iter().all(|s| s.trim().is_empty()) {
                warnings.push(format!("Entry {}: Empty subtitle text", entry.index));
            }

            // Check timing overlap with next entry
            if i + 1 < self.entries.len() {
                if let (Ok(current_end), Ok(next_start)) = (
                    timestamp_to_milliseconds(&entry.end_time),
                    timestamp_to_milliseconds(&self.entries[i + 1].start_time)
                ) {
                    if current_end > next_start {
                        warnings.push(format!("Entry {}: Overlaps with next subtitle", entry.index));
                    }
                }
            }
        }

        Ok(warnings)
    }
}

/// Convert timestamp string with given ratio
fn convert_timestamp(timestamp: &str, ratio: f32) -> Result<String> {
    let ms = timestamp_to_milliseconds(timestamp)?;
    let new_ms = (ms as f32 * ratio).round() as i32;
    Ok(milliseconds_to_timestamp(new_ms))
}

/// Convert timestamp string to milliseconds
pub fn timestamp_to_milliseconds(timestamp: &str) -> Result<i32> {
    let re = Regex::new(r"^(\d{2}):(\d{2}):(\d{2}),(\d{3})$")?;
    let caps = re.captures(timestamp)
        .ok_or_else(|| anyhow!("Invalid timestamp format: {}", timestamp))?;

    let hours: i32 = caps.get(1).unwrap().as_str().parse()?;
    let minutes: i32 = caps.get(2).unwrap().as_str().parse()?;
    let seconds: i32 = caps.get(3).unwrap().as_str().parse()?;
    let milliseconds: i32 = caps.get(4).unwrap().as_str().parse()?;

    Ok((hours * 3600000) + (minutes * 60000) + (seconds * 1000) + milliseconds)
}

/// Convert milliseconds to timestamp string
pub fn milliseconds_to_timestamp(ms: i32) -> String {
    let hours = ms / 3600000;
    let minutes = (ms - (hours * 3600000)) / 60000;
    let seconds = (ms - (hours * 3600000) - (minutes * 60000)) / 1000;
    let milliseconds = ms - (hours * 3600000) - (minutes * 60000) - (seconds * 1000);

    format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, milliseconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timestamp_conversion() {
        assert_eq!(timestamp_to_milliseconds("01:23:45,678").unwrap(), 5025678);
        assert_eq!(milliseconds_to_timestamp(5025678), "01:23:45,678");
    }

    #[test]
    fn test_framerate_conversion() {
        // Converting from 24fps to 29.97fps should make timestamps shorter
        let ratio = 24.0 / 29.97;
        let converted = convert_timestamp("00:01:00,000", ratio).unwrap();
        let original_ms = timestamp_to_milliseconds("00:01:00,000").unwrap();
        let converted_ms = timestamp_to_milliseconds(&converted).unwrap();
        
        assert!(converted_ms < original_ms);
    }
}
