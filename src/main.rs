use regex::Regex;
use std::fs::File;
use std::io::prelude::*;

/*
Create the main function, allowing us to run the program from the command line. The program will take four arguments:

    -i = input_file.srt - Mandatory

    -o = output_file.srt - Optional, defaults to Output.srt
    -if = input framerate - Optional, defaults to 29.97
    -of = output framerate - Optional, defaults to 29.97

    -h Display this help.

*/

// Create a function to convert a hh:mm:ss,mmm string to miliseconds as an integer.
fn convert_to_miliseconds(time: &str) -> i32 {
    let re = Regex::new(r"(\d{2}):(\d{2}):(\d{2}),(\d{3})").unwrap();
    let caps = re.captures(time).unwrap();
    let hours = caps.get(1).unwrap().as_str().parse::<i32>().unwrap();
    let minutes = caps.get(2).unwrap().as_str().parse::<i32>().unwrap();
    let seconds = caps.get(3).unwrap().as_str().parse::<i32>().unwrap();
    let miliseconds = caps.get(4).unwrap().as_str().parse::<i32>().unwrap();
    let total_miliseconds = (hours * 3600000) + (minutes * 60000) + (seconds * 1000) + miliseconds;
    total_miliseconds
}

// Create a function to convert miliseconds to a hh:mm:ss,mmm string.
fn convert_to_time(miliseconds: i32) -> String {
    let hours = miliseconds / 3600000;
    let minutes = (miliseconds - (hours * 3600000)) / 60000;
    let seconds = (miliseconds - (hours * 3600000) - (minutes * 60000)) / 1000;
    let miliseconds = miliseconds - (hours * 3600000) - (minutes * 60000) - (seconds * 1000);
    let time = format!(
        "{:02}:{:02}:{:02},{:03}",
        hours, minutes, seconds, miliseconds
    );
    time
}

// Create a function to convert a timecode to a new framerate.
fn convert_timecode(timecode: &str, input_framerate: f32, output_framerate: f32) -> String {
    let miliseconds = convert_to_miliseconds(timecode);
    let new_miliseconds = (miliseconds as f32 * input_framerate / output_framerate) as i32;
    let new_timecode = convert_to_time(new_miliseconds);
    new_timecode
}
// Create a function to regex replace all timecodes with converted timecodes in an input string.
fn convert_timecodes(input: &str, input_framerate: f32, output_framerate: f32) -> String {
    let re = Regex::new(r"(\d{2}):(\d{2}):(\d{2}),(\d{3})").unwrap();
    let output = re.replace_all(input, |caps: &regex::Captures| {
        let timecode = caps.get(0).unwrap().as_str();
        let new_timecode = convert_timecode(timecode, input_framerate, output_framerate);
        new_timecode
    });
    output.to_string()
}

// Create a function that Reads the input file, converts the timecodes, and writes the output file.
fn convert_file(input_file: &str, output_file: &str, input_framerate: f32, output_framerate: f32) {
    let mut file = File::open(input_file).expect("Unable to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Unable to read file");
    let output = convert_timecodes(&contents, input_framerate, output_framerate);
    let mut output_file = File::create(output_file).expect("Unable to create file");
    output_file
        .write_all(output.as_bytes())
        .expect("Unable to write file");
}

// Create the main function, which parses and validates arguments, and calls the convert_file function on the input file.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut input_file = String::new();
    let mut output_file = String::new();
    let mut input_framerate = 29.97;
    let mut output_framerate = 29.97;
    let mut help = false;
    for i in 0..args.len() {
        if args[i] == "-i" {
            input_file = args[i + 1].clone();
        } else if args[i] == "-o" {
            output_file = args[i + 1].clone();
        } else if args[i] == "-if" {
            input_framerate = args[i + 1].parse::<f32>().unwrap();
        } else if args[i] == "-of" {
            output_framerate = args[i + 1].parse::<f32>().unwrap();
        } else if args[i] == "-h" {
            help = true;
        }
    }
    if help == true {
        println!("
    -i = input file path. Expect a string denoting a path to an .srt file.
    -o = output file path. This is optional. If not provided, the program will write to a file named output.srt in the same directory as the input file.
    -if = input framerate. Optional float, defaults to 29.97
    -of = output framerate. Optional float, defaults to 29.97
    -h Display help.
    ");
    } else if input_file == "" {
        println!("No input file provided. Use -h for help.");
    } else {
        if output_file == "" {
            let re = Regex::new(r"(.*)\.srt").unwrap();
            let caps = re.captures(&input_file).unwrap();
            let output_file_name = caps.get(1).unwrap().as_str();
            output_file = format!(
                "{}-{}-{}.srt",
                output_file_name, input_framerate, output_framerate
            );
        }
        convert_file(&input_file, &output_file, input_framerate, output_framerate);
    }
}
