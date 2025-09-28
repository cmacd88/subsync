use anyhow::Result;
use regex::Regex;
use std::collections::HashMap;

/// Common video framerates to test against
const COMMON_FRAMERATES: &[f32] = &[
    23.976, 24.0, 25.0, 29.97, 30.0, 50.0, 59.94, 60.0
];

/// Represents timing information extracted from subtitles
#[derive(Debug, Clone)]
pub struct SubtitleTiming {
    pub start_ms: i32,
    pub end_ms: i32,
    pub duration_ms: i32,
}

/// Framerate detection result with confidence score
#[derive(Debug, Clone)]
pub struct FramerateDetection {
    pub framerate: f32,
    pub confidence: f32,
    pub method: String,
}

pub struct FramerateDetector {
    timings: Vec<SubtitleTiming>,
}

impl FramerateDetector {
    pub fn new() -> Self {
        Self {
            timings: Vec::new(),
        }
    }

    /// Extract timing information from SRT content
    pub fn analyze_srt_content(&mut self, content: &str) -> Result<()> {
        let re = Regex::new(r"(\d{2}):(\d{2}):(\d{2}),(\d{3}) --> (\d{2}):(\d{2}):(\d{2}),(\d{3})")?;
        
        for caps in re.captures_iter(content) {
            let start_ms = self.parse_timestamp(&caps, 1)?;
            let end_ms = self.parse_timestamp(&caps, 5)?;
            let duration_ms = end_ms - start_ms;
            
            self.timings.push(SubtitleTiming {
                start_ms,
                end_ms,
                duration_ms,
            });
        }
        
        Ok(())
    }

    /// Parse timestamp from regex captures
    fn parse_timestamp(&self, caps: &regex::Captures, start_group: usize) -> Result<i32> {
        let hours: i32 = caps.get(start_group).unwrap().as_str().parse()?;
        let minutes: i32 = caps.get(start_group + 1).unwrap().as_str().parse()?;
        let seconds: i32 = caps.get(start_group + 2).unwrap().as_str().parse()?;
        let milliseconds: i32 = caps.get(start_group + 3).unwrap().as_str().parse()?;
        
        Ok((hours * 3600000) + (minutes * 60000) + (seconds * 1000) + milliseconds)
    }

    /// Detect framerate using multiple methods and return best guess
    pub fn detect_framerate(&self) -> Result<FramerateDetection> {
        if self.timings.is_empty() {
            return Ok(FramerateDetection {
                framerate: 29.97,
                confidence: 0.0,
                method: "default".to_string(),
            });
        }

        let mut detections = Vec::new();

        // Method 1: Interval analysis
        if let Some(detection) = self.detect_by_intervals()? {
            detections.push(detection);
        }

        // Method 2: Duration pattern analysis
        if let Some(detection) = self.detect_by_duration_patterns()? {
            detections.push(detection);
        }

        // Method 3: Common framerate testing
        if let Some(detection) = self.detect_by_common_framerates()? {
            detections.push(detection);
        }

        // Return detection with highest confidence
        detections.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(detections.into_iter().next().unwrap_or(FramerateDetection {
            framerate: 29.97,
            confidence: 0.1,
            method: "fallback".to_string(),
        }))
    }

    /// Detect framerate by analyzing intervals between subtitles
    fn detect_by_intervals(&self) -> Result<Option<FramerateDetection>> {
        if self.timings.len() < 10 {
            return Ok(None);
        }

        let mut intervals = Vec::new();
        for i in 1..self.timings.len() {
            let interval = self.timings[i].start_ms - self.timings[i-1].end_ms;
            if interval > 0 && interval < 10000 { // Reasonable interval (0-10 seconds)
                intervals.push(interval);
            }
        }

        if intervals.is_empty() {
            return Ok(None);
        }

        // Find most common interval
        let mut interval_counts: HashMap<i32, usize> = HashMap::new();
        for &interval in &intervals {
            // Round to nearest 100ms for grouping
            let rounded = (interval / 100) * 100;
            *interval_counts.entry(rounded).or_insert(0) += 1;
        }

        let most_common_interval = interval_counts
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(&interval, _)| interval);

        if let Some(interval) = most_common_interval {
            // Try to match interval to known framerate patterns
            for &fps in COMMON_FRAMERATES {
                let frame_duration_ms = 1000.0 / fps;
                let expected_intervals = [
                    frame_duration_ms as i32,
                    (frame_duration_ms * 2.0) as i32,
                    (frame_duration_ms * 3.0) as i32,
                ];

                for expected in expected_intervals {
                    if (interval - expected).abs() < 50 { // 50ms tolerance
                        let confidence = 1.0 - (interval - expected).abs() as f32 / 50.0;
                        return Ok(Some(FramerateDetection {
                            framerate: fps,
                            confidence: confidence * 0.7, // Lower confidence for interval method
                            method: "interval_analysis".to_string(),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Detect framerate by analyzing subtitle duration patterns
    fn detect_by_duration_patterns(&self) -> Result<Option<FramerateDetection>> {
        if self.timings.len() < 20 {
            return Ok(None);
        }

        // Analyze if durations align with frame boundaries
        let mut best_match = None;
        let mut best_score = 0.0;

        for &fps in COMMON_FRAMERATES {
            let frame_duration_ms = 1000.0 / fps;
            let mut aligned_count = 0;
            let mut total_count = 0;

            for timing in &self.timings {
                total_count += 1;
                let frames = timing.duration_ms as f32 / frame_duration_ms;
                let rounded_frames = frames.round();
                
                // Check if duration is close to a whole number of frames
                if (frames - rounded_frames).abs() < 0.1 {
                    aligned_count += 1;
                }
            }

            let alignment_ratio = aligned_count as f32 / total_count as f32;
            if alignment_ratio > best_score && alignment_ratio > 0.6 {
                best_score = alignment_ratio;
                best_match = Some(FramerateDetection {
                    framerate: fps,
                    confidence: alignment_ratio * 0.8,
                    method: "duration_pattern".to_string(),
                });
            }
        }

        Ok(best_match)
    }

    /// Test against common framerates using statistical analysis
    fn detect_by_common_framerates(&self) -> Result<Option<FramerateDetection>> {
        if self.timings.len() < 5 {
            return Ok(None);
        }

        // Simple heuristic: check if timestamps align well with common framerates
        let total_duration = self.timings.last().unwrap().end_ms - self.timings.first().unwrap().start_ms;
        
        // For longer content, prefer certain framerates
        let confidence_boost = if total_duration > 3600000 { 0.1 } else { 0.0 }; // 1 hour+

        // Default to 29.97 for NTSC content, 25 for PAL
        let default_fps = if self.has_ntsc_indicators() { 29.97 } else { 25.0 };
        
        Ok(Some(FramerateDetection {
            framerate: default_fps,
            confidence: 0.5 + confidence_boost,
            method: "common_framerate_heuristic".to_string(),
        }))
    }

    /// Check for NTSC indicators (rough heuristic)
    fn has_ntsc_indicators(&self) -> bool {
        // Very basic heuristic - could be improved with more sophisticated analysis
        if let Some(first) = self.timings.first() {
            if let Some(last) = self.timings.last() {
                let total_duration = last.end_ms - first.start_ms;
                let subtitle_count = self.timings.len();
                
                // Rough estimate of subtitle density
                let density = subtitle_count as f32 / (total_duration as f32 / 60000.0); // per minute
                
                // NTSC content often has different subtitle timing patterns
                // This is a very rough heuristic and could be improved
                return density > 10.0; // Arbitrary threshold
            }
        }
        false
    }

    /// Get statistics about the analyzed content
    pub fn get_statistics(&self) -> HashMap<String, f32> {
        let mut stats = HashMap::new();
        
        if !self.timings.is_empty() {
            let total_duration: i32 = self.timings.iter().map(|t| t.duration_ms).sum();
            let avg_duration = total_duration as f32 / self.timings.len() as f32;
            
            let first_start = self.timings.first().unwrap().start_ms;
            let last_end = self.timings.last().unwrap().end_ms;
            let total_span = last_end - first_start;
            
            stats.insert("subtitle_count".to_string(), self.timings.len() as f32);
            stats.insert("average_duration_ms".to_string(), avg_duration);
            stats.insert("total_span_ms".to_string(), total_span as f32);
            stats.insert("density_per_minute".to_string(), 
                        self.timings.len() as f32 / (total_span as f32 / 60000.0));
        }
        
        stats
    }
}
