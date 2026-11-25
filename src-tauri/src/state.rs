use crate::models::AppState;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct StateStore {
    path: PathBuf,
    state: Mutex<AppState>,
}

impl StateStore {
    pub fn open(path: PathBuf) -> Self {
        let state = read_state(&path).unwrap_or_default();
        Self {
            path,
            state: Mutex::new(state),
        }
    }

    pub fn default_path() -> PathBuf {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .unwrap_or_else(|| PathBuf::from(".local/share"));
        base.join("quelldock/state.json")
    }

    pub fn snapshot(&self) -> Result<AppState, String> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| "Application state lock was poisoned".into())
    }

    pub fn update<F>(&self, change: F) -> Result<AppState, String>
    where
        F: FnOnce(&mut AppState),
    {
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Application state lock was poisoned".to_string())?;
        let mut next = state.clone();
        change(&mut next);
        persist_state(&self.path, &next)?;
        *state = next;
        Ok(state.clone())
    }

    pub fn reset(&self) -> Result<AppState, String> {
        self.update(|state| *state = AppState::default())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn read_state(path: &Path) -> Result<AppState, String> {
    let data = fs::read(path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&data).map_err(|error| error.to_string())
}

fn persist_state(path: &Path, state: &AppState) -> Result<(), String> {
    let parent = path.parent().ok_or("State path has no parent directory")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Cannot create state directory: {error}"))?;
    let temp_path = path.with_extension("json.tmp");
    let data = serde_json::to_vec_pretty(state).map_err(|error| error.to_string())?;
    let mut file =
        fs::File::create(&temp_path).map_err(|error| format!("Cannot write state: {error}"))?;
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("Cannot flush state: {error}"))?;
    fs::rename(&temp_path, path).map_err(|error| format!("Cannot replace state file: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn test_path() -> PathBuf {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("quelldock-state-{id}/state.json"))
    }

    #[test]
    fn missing_or_corrupt_state_uses_defaults() {
        let path = test_path();
        let store = StateStore::open(path.clone());
        assert!(store
            .snapshot()
            .unwrap()
            .settings
            .appimage_directory
            .ends_with("/Applications"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{").unwrap();
        assert!(StateStore::open(path).snapshot().is_ok());
    }

    #[test]
    fn updates_are_atomically_persisted() {
        let path = test_path();
        let store = StateStore::open(path.clone());
        store
            .update(|state| state.settings.aur_no_confirm = true)
            .unwrap();
        assert!(
            StateStore::open(path.clone())
                .snapshot()
                .unwrap()
                .settings
                .aur_no_confirm
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
