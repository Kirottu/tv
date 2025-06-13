use std::{fs, process::Command, thread, time::Duration};

use clap::{Parser, Subcommand};

struct VideoOutput<'a> {
    output: &'a str,
    /// Workspaces will be laid out in this order
    workspaces: &'a [&'a str],
}

const STATE_FILE_PATH: &str = "/tmp/tv.state";
const DESKTOP_VIDEO_OUTPUTS: &[&VideoOutput] = &[
    &VideoOutput {
        output: "DP-1",
        workspaces: &["web-dp1"],
    },
    &VideoOutput {
        output: "DP-2",
        workspaces: &["games", "web-dp2"],
    },
    &VideoOutput {
        output: "DP-3",
        workspaces: &["utils", "chat", "web-dp3"],
    },
];
const TV_VIDEO_OUTPUT: VideoOutput = VideoOutput {
    output: "HDMI-A-1",
    workspaces: &["games", "utils", "chat", "web-dp1", "web-dp2", "web-dp3"],
};

const DESKTOP_AUDIO_SINK: &str = "alsa_output.pci-0000_0a_00.4.analog-stereo";
const TV_AUDIO_SINK: &str = "alsa_output.pci-0000_08_00.1.hdmi-stereo";
const TV_SCALE: &str = "2.0";

macro_rules! cmd {
    ( $( $arg:expr ),* ) => {
        Command::new("sh").arg("-c").arg(format!($($arg,)*)).output().unwrap()
    };
}

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
    /// Toggle TV scaling
    ToggleScaling,
    /// Switch to TV mode
    Tv,
    /// Switch to Desktop mode
    Desktop,
    /// Switch TV to normal scaling
    Scaled,
    /// Switch TV to a scale of 1
    Unscaled,
    /// Fix workspace order
    FixWorkspaceOrder,
}

struct State {
    tv: bool,
    scaled: bool,
}

impl State {
    fn load() -> Self {
        if !fs::exists(STATE_FILE_PATH).unwrap() {
            Self::init();
            return Self::default();
        }

        let string = fs::read_to_string(STATE_FILE_PATH).unwrap();

        let tv = match string.lines().next().unwrap() {
            "tv" => true,
            "desktop" => false,
            _ => unreachable!("Invalid state file content"),
        };
        let scaled = match string.lines().nth(1).unwrap() {
            "scaled" => true,
            "unscaled" => false,
            _ => unreachable!("Invalid state file content"),
        };

        Self { tv, scaled }
    }

    fn save(&self) {
        fs::write(
            STATE_FILE_PATH,
            format!(
                "{}\n{}",
                if self.tv { "tv" } else { "desktop" },
                if self.scaled { "scaled" } else { "unscaled" }
            ),
        )
        .unwrap();
    }

    fn init() {
        Self::default().save()
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            tv: false,
            scaled: true,
        }
    }
}

fn main() {
    let args = Args::parse();

    match args.command {
        Action::Init => {
            State::init();
            cmd!("pactl set-default-sink {}", DESKTOP_AUDIO_SINK);
        }
        Action::Toggle => {
            let state = State::load();
            if state.tv {
                to_desktop(state)
            } else {
                to_tv(state)
            }
        }
        Action::ToggleScaling => {
            let state = State::load();
            if state.tv {
                if state.scaled {
                    to_unscaled(state)
                } else {
                    to_scaled(state)
                }
            }
        }
        Action::Tv => {
            let state = State::load();
            if !state.tv {
                to_tv(state)
            }
        }
        Action::Desktop => {
            let state = State::load();
            if state.tv {
                to_desktop(state)
            }
        }
        Action::Scaled => {
            let state = State::load();
            if state.tv && !state.scaled {
                to_scaled(state)
            }
        }
        Action::Unscaled => {
            let state = State::load();
            if state.tv && state.scaled {
                to_unscaled(state)
            }
        }
        Action::FixWorkspaceOrder => {
            let state = State::load();
            fix_workspace_order(state);
        }
    }
}

fn to_tv(mut state: State) {
    state.tv = true;
    state.save();
    for (i, _) in DESKTOP_VIDEO_OUTPUTS.iter().enumerate() {
        cmd!(
            "eww open tv-transition --id {i} --screen {i} --arg text='Switching to TV...' --duration 3s"
        );
    }
    cmd!("niri msg output {} on", TV_VIDEO_OUTPUT.output);
    thread::sleep(Duration::from_secs(1));
    for workspace in TV_VIDEO_OUTPUT.workspaces.iter() {
        cmd!(
            "niri msg action move-workspace-to-monitor --reference {} {}",
            workspace,
            TV_VIDEO_OUTPUT.output
        );
    }
    fix_workspace_order(state);
    thread::sleep(Duration::from_secs(1));
    for output in DESKTOP_VIDEO_OUTPUTS {
        cmd!("niri msg output {} off", output.output);
    }
    cmd!("pactl set-default-sink {}", TV_AUDIO_SINK);
}

fn fix_workspace_order(state: State) {
    if state.tv {
        for (i, workspace) in TV_VIDEO_OUTPUT.workspaces.iter().enumerate() {
            cmd!(
                "niri msg action move-workspace-to-index --reference {} {}",
                workspace,
                i + 1
            );
        }
    } else {
        for output in DESKTOP_VIDEO_OUTPUTS {
            for (i, workspace) in output.workspaces.iter().enumerate() {
                cmd!(
                    "niri msg action move-workspace-to-index --reference {} {}",
                    workspace,
                    i + 1
                );
            }
        }
    }
}

fn to_desktop(mut state: State) {
    state.tv = false;
    state.scaled = true;
    state.save();
    cmd!(
        "niri msg output {} scale {}",
        TV_VIDEO_OUTPUT.output,
        TV_SCALE
    );
    cmd!("eww open tv-transition --id 0 --screen 0 --arg text='Switching to Desktop...' --duration 3s");
    for output in DESKTOP_VIDEO_OUTPUTS {
        cmd!("niri msg output {} on", output.output);
    }
    thread::sleep(Duration::from_secs(2));
    cmd!("niri msg output {} off", TV_VIDEO_OUTPUT.output);

    for output in DESKTOP_VIDEO_OUTPUTS {
        for workspace in output.workspaces.iter() {
            cmd!(
                "niri msg action move-workspace-to-monitor --reference {} {}",
                workspace,
                output.output
            );
        }
    }
    fix_workspace_order(state);
    cmd!("pactl set-default-sink {}", DESKTOP_AUDIO_SINK);
}

fn to_scaled(mut state: State) {
    state.scaled = true;
    state.save();
    cmd!(
        "niri msg output {} scale {}",
        TV_VIDEO_OUTPUT.output,
        TV_SCALE
    );
}

fn to_unscaled(mut state: State) {
    state.scaled = false;
    state.save();
    cmd!("niri msg output {} scale 1.0", TV_VIDEO_OUTPUT.output);
}
