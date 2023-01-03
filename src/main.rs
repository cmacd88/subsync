use chrono::{DateTime, Duration, TimeZone, Utc};
use clap::{App, Arg};
use regex::Regex;
use std::env;
use std::fs;

// Set up the command-line arguments using the clap library
let matches = App::new("timecode-converter")
    .version("1.0")
    .author("Your Name")
    .about("Converts timecodes between different framerates")
    .arg(
        Arg::with_name("input")
            .short("i")
            .long("input")
            .value_name("INPUT")
            .help("The input subtitle file")
            .takes_value(true)
            .required(true),
    )
    .arg(
        Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("OUTPUT")
            .help("The output subtitle file")
            .takes_value(true)
            .default_value("Output.srt"),
    )
    .arg(
        Arg::with_name("input_framerate")
            .short("if")
            .long("input-framerate")
            .value_name("INPUT_FRAMERATE")
            .help("The framerate of the input file")
            .takes_value(true)
            .default_value("29.97"),
    )
    .arg(
        Arg::with_name("output_framerate")
            .short("of")
            .long("output-framerate")
            .value_name("OUTPUT_FRAMERATE")
            .help("The framerate of the output file")
            .takes_value(true)
            .default_value("29.97"),
    )
    .get_matches();

// Get the values of the command-line arguments
let input_file = matches.value_of("input").unwrap();
let output_file = matches.value_of("output").unwrap();
let input_framerate = matches.value_of("input_framerate").unwrap().parse::<f32>().unwrap();
let output_framerate = matches.value_of("output_framerate").unwrap().parse::<f32>().unwrap();

// Read the input file
let input = fs::read_to_string(input_file).unwrap();

// Convert the timecodes in the input string
let re = Regex::new(r"(\d{2}):(\d{2}):(\d{2}),(\d{3})").unwrap();
let output = re.replace_all(&input, |caps: &regex::Captures| {
    let timecode = caps.get(0).unwrap().as_str();
    let miliseconds = Utc.timestamp_millis_str(timecode).unwrap();
    let new_miliseconds = (miliseconds.timestamp_millis() as f32 * input_framerate / output_framerate) as i32;
    let new_timecode = Utc.timestamp_millis(new_miliseconds as i64).format("%H:%M:%S,%3f").to_string();
    new_timecode
});

// Write the output file
fs::write(output_file, output).unwrap();
