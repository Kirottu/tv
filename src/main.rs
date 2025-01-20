use std::{
    env,
    ffi::CString,
    fs,
    process::{Command, Output},
    thread,
    time::Duration,
};

use clap::{Parser, Subcommand};
use nix::unistd::{execvp, execvpe};

const STATE_FILE_PATH: &str = "/tmp/tv.state";
const DESKTOP_VIDEO_OUTPUTS: &[&str] = &["DP-1", "DP-2", "DP-3"];
const TV_VIDEO_OUTPUT: &str = "HDMI-A-1";

const DESKTOP_AUDIO_SINK: &str = "alsa_output.pci-0000_09_00.4.analog-stereo";
const TV_AUDIO_SINK: &str = "alsa_output.pci-0000_07_00.1.hdmi-stereo";

#[derive(Parser)]
struct Args {
    /// What action to perform
    #[command(subcommand)]
    command: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Initialize the state file and set correct output
    Init,
    /// Toggle between TV and Desktop modes
    Toggle,
    /// Switch to TV mode
    Tv,
    /// Switch to Desktop mode
    Desktop,
    /// Helper for launching games
    Game {
        /// What gamescope args to use in TV mode
        #[arg(short, long)]
        tv_gamescope_args: Option<String>,
        /// What gamescope args to use in Desktop mode
        #[arg(short, long)]
        desktop_gamescope_args: Option<String>,
        command: String,
    },
}

fn main() {
    let args = Args::parse();

    match args.command {
        Action::Init => {
            fs::write(STATE_FILE_PATH, "desktop").unwrap();
            cmd(&format!("pactl set-default-sink {}", DESKTOP_AUDIO_SINK));
        }
        Action::Toggle => match fs::read_to_string(STATE_FILE_PATH).unwrap().as_str() {
            "tv" => to_desktop(),
            "desktop" => to_tv(),
            _ => unreachable!("Invalid state file content"),
        },
        Action::Tv => {
            if &fs::read_to_string(STATE_FILE_PATH).unwrap() == "desktop" {
                to_tv()
            }
        }
        Action::Desktop => {
            if &fs::read_to_string(STATE_FILE_PATH).unwrap() == "tv" {
                to_desktop()
            }
        }
        Action::Game {
            command,
            tv_gamescope_args,
            desktop_gamescope_args,
        } => match fs::read_to_string(STATE_FILE_PATH).unwrap().as_str() {
            "tv" => {
                if let Some(args) = tv_gamescope_args {
                    let _ = execvp(
                        c"gamescope",
                        &format!("gamescope {} -- {}", args, command)
                            .split(" ")
                            .map(|s| CString::new(s).unwrap())
                            .collect::<Vec<_>>(),
                    );
                    cmd(&format!("gamescope {} -- {}", args, command));
                } else {
                    cmd(&command);
                }
            }
            "desktop" => {
                if let Some(args) = desktop_gamescope_args {
                    cmd(&format!("gamescope {} -- {}", args, command));
                } else {
                    cmd(&command);
                }
            }
            _ => unreachable!("Invalid state file content"),
        },
    }
}

fn to_tv() {
    fs::write(STATE_FILE_PATH, "tv").unwrap();
    for (i, _) in DESKTOP_VIDEO_OUTPUTS.iter().enumerate() {
        cmd(&format!(
            "eww open tv-transition --id {i} --screen {i} --arg text='Switching to TV...' --duration 3s"
        ));
    }
    cmd(&format!("niri msg output {} on", TV_VIDEO_OUTPUT));
    thread::sleep(Duration::from_secs(2));
    for output in DESKTOP_VIDEO_OUTPUTS {
        cmd(&format!("niri msg output {} off", output));
    }
    cmd(&format!("pactl set-default-sink {}", TV_AUDIO_SINK));
}

fn to_desktop() {
    fs::write(STATE_FILE_PATH, "desktop").unwrap();
    cmd("eww open tv-transition --id 0 --screen 0 --arg text='Switching to Desktop...' --duration 3s");
    for output in DESKTOP_VIDEO_OUTPUTS {
        cmd(&format!("niri msg output {} on", output));
    }
    thread::sleep(Duration::from_secs(2));
    cmd(&format!("niri msg output {} off", TV_VIDEO_OUTPUT));
    cmd(&format!("pactl set-default-sink {}", DESKTOP_AUDIO_SINK));
}

fn cmd(cmd: &str) -> Output {
    Command::new("bash").arg("-c").arg(cmd).output().unwrap()
}
