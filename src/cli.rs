use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "subsync")]
#[command(about = "A subtitle synchronization tool for framerate conversion")]
#[command(version = "0.2.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Convert subtitle timestamps between framerates
    Convert {
        /// Input subtitle file (.srt)
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output subtitle file (optional, defaults to input_[from]fps_to_[to]fps.srt)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Source framerate (will attempt to detect if not specified)
        #[arg(long, value_name = "FPS")]
        from_fps: Option<f32>,
        
        /// Target framerate
        #[arg(long, value_name = "FPS")]
        to_fps: f32,
        
        /// Force conversion even with low confidence detection
        #[arg(long)]
        force: bool,
        
        /// Show detailed analysis information
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Analyze subtitle file and detect likely framerate
    Analyze {
        /// Input subtitle file (.srt)
        #[arg(short, long)]
        input: PathBuf,
        
        /// Show detailed statistics
        #[arg(short, long)]
        verbose: bool,
    },
    
    /// Show information about common framerates
    Info,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

/// Generate output filename if not specified
pub fn generate_output_filename(input: &PathBuf, from_fps: f32, to_fps: f32) -> PathBuf {
    let input_stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let input_dir = input.parent().unwrap_or_else(|| std::path::Path::new("."));
    
    let output_name = format!("{}_{}fps_to_{}fps.srt", input_stem, from_fps, to_fps);
    input_dir.join(output_name)
}

/// Display framerate information
pub fn show_framerate_info() {
    println!("Common Video Framerates:");
    println!("  23.976 fps - Film (24fps slowed down for NTSC)");
    println!("  24.000 fps - Cinema standard");
    println!("  25.000 fps - PAL standard (Europe, Australia)");
    println!("  29.970 fps - NTSC standard (North America, Japan)");
    println!("  30.000 fps - Some digital video");
    println!("  50.000 fps - PAL high framerate");
    println!("  59.940 fps - NTSC high framerate");
    println!("  60.000 fps - High framerate digital");
    println!();
    println!("Note: The most common conversion is between 23.976/24fps (film) and 29.97fps (TV)");
}
