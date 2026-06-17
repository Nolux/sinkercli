use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioSystem {
    PipeWire,
    PulseAudio,
}

impl AudioSystem {
    pub fn label(&self) -> &'static str {
        match self {
            AudioSystem::PipeWire => "PipeWire",
            AudioSystem::PulseAudio => "PulseAudio",
        }
    }
}

pub fn detect() -> AudioSystem {
    let runtime_dir = runtime_dir();
    let pw_socket = PathBuf::from(&runtime_dir).join("pipewire-0");
    if pw_socket.exists() {
        AudioSystem::PipeWire
    } else {
        AudioSystem::PulseAudio
    }
}

fn runtime_dir() -> String {
    if let Ok(dir) = std::env::var("XDG_RUNTIME_DIR") {
        return dir;
    }
    // Fallback: read UID from /proc/self/status
    if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Uid:\t") {
                if let Some(uid) = rest.split_whitespace().next() {
                    return format!("/run/user/{}", uid);
                }
            }
        }
    }
    "/run/user/1000".to_string()
}
