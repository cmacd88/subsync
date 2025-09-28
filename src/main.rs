mod cli;
mod framerate_detector;
mod subtitle_parser;

use anyhow::{anyhow, Result};
use cli::{Cli, Commands};
use framerate_detector::{FramerateDetector, FramerateDetection};
use subtitle_parser::SubtitleFile;
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse_args();

    match cli.command {
        Commands::Convert {
            input,
            output,
            from_fps,
            to_fps,
            force,
            verbose,
        } => {
            handle_convert(input, output, from_fps, to_fps, force, verbose)?;
        }
        Commands::Analyze { input, verbose } => {
            handle_analyze(input, verbose)?;
        }
        Commands::Info => {
            cli::show_framerate_info();
        }
    }

    Ok(())
}

fn handle_convert(
    input: std::path::PathBuf,
    output: Option<std::path::PathBuf>,
    from_fps: Option<f32>,
    to_fps: f32,
    force: bool,
    verbose: bool,
) -> Result<()> {
    // Load subtitle file
    if verbose {
        println!("Loading subtitle file: {}", input.display());
    }
    
    let mut subtitle_file = SubtitleFile::from_file(&input)?;
    
    // Validate subtitle file
    let warnings = subtitle_file.validate()?;
    if !warnings.is_empty() && verbose {
        println!("Validation warnings:");
        for warning in &warnings {
            println!("  ⚠️  {}", warning);
        }
        println!();
    }

    // Determine source framerate
    let source_fps = if let Some(fps) = from_fps {
        if verbose {
            println!("Using specified source framerate: {} fps", fps);
        }
        fps
    } else {
        if verbose {
            println!("Detecting source framerate...");
        }
        
        let detection = detect_framerate(&subtitle_file, verbose)?;
        
        if detection.confidence < 0.5 && !force {
            return Err(anyhow!(
                "Low confidence framerate detection ({:.1}% confidence for {} fps). \
                Use --force to proceed anyway, or specify --from-fps manually.",
                detection.confidence * 100.0,
                detection.framerate
            ));
        }
        
        if verbose {
            println!(
                "Detected framerate: {} fps (confidence: {:.1}%, method: {})",
                detection.framerate,
                detection.confidence * 100.0,
                detection.method
            );
        } else {
            println!(
                "Detected source framerate: {} fps ({:.1}% confidence)",
                detection.framerate,
                detection.confidence * 100.0
            );
        }
        
        detection.framerate
    };

    // Check if conversion is needed
    if (source_fps - to_fps).abs() < 0.001 {
        println!("Source and target framerates are the same. No conversion needed.");
        return Ok(());
    }

    // Perform conversion
    if verbose {
        println!("Converting from {} fps to {} fps...", source_fps, to_fps);
    }
    
    subtitle_file.convert_framerate(source_fps, to_fps)?;

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        cli::generate_output_filename(&input, source_fps, to_fps)
    });

    // Save converted file
    subtitle_file.save_to_file(&output_path)?;
    
    println!("✅ Conversion complete!");
    println!("   Input:  {} ({} fps)", input.display(), source_fps);
    println!("   Output: {} ({} fps)", output_path.display(), to_fps);
    
    // Show post-conversion validation
    let post_warnings = subtitle_file.validate()?;
    if !post_warnings.is_empty() && verbose {
        println!("\nPost-conversion validation:");
        for warning in &post_warnings {
            println!("  ⚠️  {}", warning);
        }
    }

    Ok(())
}

fn handle_analyze(input: std::path::PathBuf, verbose: bool) -> Result<()> {
    println!("Analyzing subtitle file: {}", input.display());
    
    let subtitle_file = SubtitleFile::from_file(&input)?;
    
    // Basic file info
    println!("\n📊 File Information:");
    println!("   Subtitle entries: {}", subtitle_file.entries.len());
    
    if let (Some(first), Some(last)) = (subtitle_file.entries.first(), subtitle_file.entries.last()) {
        println!("   First subtitle: {}", first.start_time);
        println!("   Last subtitle:  {}", last.end_time);
        
        let start_ms = subtitle_parser::timestamp_to_milliseconds(&first.start_time)?;
        let end_ms = subtitle_parser::timestamp_to_milliseconds(&last.end_time)?;
        let duration_ms = end_ms - start_ms;
        let duration_min = duration_ms as f32 / 60000.0;
        
        println!("   Total duration: {:.1} minutes", duration_min);
    }

    // Framerate detection
    println!("\n🔍 Framerate Analysis:");
    let detection = detect_framerate(&subtitle_file, verbose)?;
    
    println!("   Detected framerate: {} fps", detection.framerate);
    println!("   Confidence: {:.1}%", detection.confidence * 100.0);
    println!("   Detection method: {}", detection.method);
    
    if detection.confidence < 0.7 {
        println!("   ⚠️  Low confidence detection - consider manual specification");
    }

    // Validation
    let warnings = subtitle_file.validate()?;
    if !warnings.is_empty() {
        println!("\n⚠️  Validation Issues:");
        for warning in &warnings {
            println!("   {}", warning);
        }
    } else {
        println!("\n✅ No validation issues found");
    }

    // Detailed statistics if verbose
    if verbose {
        let mut detector = FramerateDetector::new();
        let content = std::fs::read_to_string(&input)?;
        detector.analyze_srt_content(&content)?;
        let stats = detector.get_statistics();
        
        println!("\n📈 Detailed Statistics:");
        for (key, value) in stats {
            println!("   {}: {:.2}", key, value);
        }
    }

    Ok(())
}

fn detect_framerate(subtitle_file: &SubtitleFile, verbose: bool) -> Result<FramerateDetection> {
    let mut detector = FramerateDetector::new();
    
    // Convert subtitle timing info to detector format
    for entry in &subtitle_file.entries {
        let start_ms = subtitle_parser::timestamp_to_milliseconds(&entry.start_time)?;
        let end_ms = subtitle_parser::timestamp_to_milliseconds(&entry.end_time)?;
        
        detector.timings.push(framerate_detector::SubtitleTiming {
            start_ms,
            end_ms,
            duration_ms: end_ms - start_ms,
        });
    }
    
    let detection = detector.detect_framerate()?;
    
    if verbose {
        let stats = detector.get_statistics();
        println!("Detection statistics:");
        for (key, value) in stats {
            println!("  {}: {:.2}", key, value);
        }
    }
    
    Ok(detection)
}
