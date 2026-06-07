use std::env;
use std::path::PathBuf;

pub fn default_registry_path() -> PathBuf {
    if let Ok(path) = env::var("STP_REGISTRY") {
        return PathBuf::from(path);
    }
    state_home().join("sexy-terminal-panel/registry.json")
}

pub fn default_broker_socket_path() -> PathBuf {
    if let Ok(path) = env::var("STP_BROKER_SOCKET") {
        return PathBuf::from(path);
    }
    state_home().join("sexy-terminal-panel/broker.sock")
}

pub fn selected_registry_path(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(default_registry_path)
}

pub fn selected_broker_socket_path(path: Option<PathBuf>) -> PathBuf {
    path.unwrap_or_else(default_broker_socket_path)
}

fn state_home() -> PathBuf {
    env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".local/state")))
        .unwrap_or_else(|_| PathBuf::from("."))
}
