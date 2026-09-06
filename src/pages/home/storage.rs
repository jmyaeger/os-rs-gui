//! Browser persistence for the calculator. No-ops outside the browser.

#[cfg(target_arch = "wasm32")]
pub fn load<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    let storage = web_sys::window()?.local_storage().ok()??;
    let json = storage.get_item(key).ok()??;
    match serde_json::from_str(&json) {
        Ok(value) => Some(value),
        Err(error) => {
            log::warn!("Ignoring stored {key}: {error}");
            None
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn save<T: serde::Serialize>(key: &str, value: &T) {
    let Some(storage) = web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    else {
        return;
    };
    match serde_json::to_string(value) {
        Ok(json) => {
            if let Err(error) = storage.set_item(key, &json) {
                log::warn!("Could not store {key}: {error:?}");
            }
        }
        Err(error) => log::warn!("Could not serialize {key}: {error}"),
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load<T: serde::de::DeserializeOwned>(_key: &str) -> Option<T> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
pub fn save<T: serde::Serialize>(_key: &str, _value: &T) {}
