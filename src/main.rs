use std::process::Command;
use std::collections::HashMap;
use std::fs;
use serde_derive::Deserialize;
use regex::Regex;
extern crate clap;
use clap::{Parser, Subcommand, ValueEnum};
const XRANDR_PROPS_CMD : &str = "xrandr --props";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the crandr config file
    #[clap(short, long, value_parser)]
    config: String,
}

#[derive(Debug, Clone)]
struct DisplayProperties {
    resolutions : Vec<String>
}
#[derive(Deserialize, Debug)]
struct MonitorSetup {
    monitor_setup: HashMap<String, DisplayConfig>
}
#[derive(Deserialize, Debug, Clone)]
struct DisplayConfig {
    displays : HashMap<String, String>,
    command : String
}

fn get_connected_displays() -> HashMap<String, DisplayProperties>{
    let xrandr_info_raw =  Command::new("sh")
        .arg("-c")
        .arg(XRANDR_PROPS_CMD)
        .output()
        .expect("failed to execute process").stdout;

    let xrandr_info = match std::str::from_utf8(&xrandr_info_raw){
        Ok(x) => x,
        Err(e) => panic!("Invalid UTF-8 sequence: {}", e)
    };

    let xrandr_lines = xrandr_info.split("\n");
    let mut connected_monitors : HashMap<String, DisplayProperties> = HashMap::new();
    let mut fetching_props: bool = false;
    let mut current_display : &str = "";
    let mut current_config : DisplayProperties = DisplayProperties{
        resolutions : Vec::new()
    };
    for line in xrandr_lines{
        let mut space_separated_line : Vec<&str>= line.split(" ").filter(|&x| !x.is_empty()).collect();

        if line.contains(" connected")  {
            if fetching_props {
                connected_monitors.insert(current_display.to_string(), current_config);
            }
            current_config = DisplayProperties{
                resolutions : Vec::new()
            };
            current_display = space_separated_line[0];
            fetching_props = true;

        }else if line.contains(" disconnected") {
            if fetching_props {
                connected_monitors.insert(current_display.to_string(), current_config);
            }
            current_config = DisplayProperties{
                resolutions : Vec::new()
            };
            fetching_props = false;
        }
        else if fetching_props{
            let re = Regex::new(r"^\d{3,4}x\d{3,4}$").unwrap();
            if re.is_match(space_separated_line[0]){
                current_config.resolutions.push(space_separated_line[0].to_string());
            }
        }
    }
    if fetching_props {
        connected_monitors.insert(current_display.to_string(), current_config);
    }
    return connected_monitors
}

fn match_display_config(configs : HashMap<String, DisplayConfig>, connected_monitors : HashMap<String, DisplayProperties>) -> Result<DisplayConfig, String>{
    let mut monitor_names : Vec<String> = connected_monitors.keys().cloned().collect();
    monitor_names.sort();

    for (name, config) in configs{
        let mut found_config : bool = true;
        for (display, resolution) in config.displays.clone(){
            if !connected_monitors.contains_key(&display) || !connected_monitors[&display].resolutions.contains( &resolution){
                found_config = false;
            }
        }
        if found_config{
            return Ok(config);
        }
    }
    return Err("No matching config found".to_string());
}

fn main() {
    let args = Args::parse();
    let connected_monitors : HashMap<String, DisplayProperties> = get_connected_displays();
    let contents = fs::read_to_string(args.config)
        .expect("Something went wrong reading the file");
    let configs: HashMap<String, DisplayConfig> = toml::from_str(&contents).unwrap();
    println!("{:?}", connected_monitors);
    let current_config : DisplayConfig = match_display_config(configs, connected_monitors).unwrap();
    println!("{:?}", current_config);
    let response_raw = Command::new("sh")
        .arg("-c")
        .arg(current_config.command)
        .output()
        .expect("failed to execute process").stdout;
}