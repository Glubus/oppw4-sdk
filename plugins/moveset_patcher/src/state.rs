use std::{
    path::{Component, Path, PathBuf},
    sync::{Mutex, OnceLock},
};

use plugin_sdk::{
    linkdata::{LinkDataEntryId, LinkDataFile},
    HostApi, OwnedHostApi,
};
use serde::Deserialize;

use crate::{constants::PLUGIN_ID, log};

static ENTRIES: OnceLock<Mutex<usize>> = OnceLock::new();
static HOST: OnceLock<OwnedHostApi> = OnceLock::new();

pub(crate) fn initialize(host: HostApi<'_>) -> Result<(), String> {
    let _ = HOST.set(host.owned());
    let _ = ENTRIES.get_or_init(|| Mutex::new(0));
    Ok(())
}

pub(crate) fn edit_count() -> usize {
    ENTRIES
        .get_or_init(|| Mutex::new(0))
        .lock()
        .map(|entries| *entries)
        .unwrap_or(0)
}

pub(crate) fn replace_from_registry(function_name: &str, args_json: &str) -> Result<String, i32> {
    if function_name != "replace" {
        return Err(-42);
    }
    let request = parse_replace_request(args_json)?;
    let caller = request.caller.ok_or(-47)?;
    if caller.is_zip {
        return Err(-48);
    }
    let root = PathBuf::from(caller.root);
    let payload_path = resolve_mod_relative_path(&root, &request.payload_file).ok_or(-49)?;
    let payload = std::fs::read(&payload_path).map_err(|_| -50)?;
    let host = host_api().ok_or(-51)?;
    host.linkdata()
        .replace_entry(
            PLUGIN_ID,
            LinkDataFile::A,
            LinkDataEntryId::new(request.entry),
            &payload,
        )
        .map_err(|_| -52)?;

    let patches = ENTRIES.get_or_init(|| Mutex::new(0));
    let patch_count = {
        let mut guard = patches.lock().map_err(|_| -53)?;
        *guard += 1;
        *guard
    };
    log::write(
        host,
        format!(
            "moveset patch registered mod={} entry={} payload={} bytes={} patches={}",
            caller.mod_id,
            request.entry,
            payload_path.display(),
            payload.len(),
            patch_count
        ),
    );
    Ok(serde_json::json!({
        "ok": true,
        "entry": request.entry,
        "bytes": payload.len(),
        "patches": patch_count,
    })
    .to_string())
}

fn host_api<'api>() -> Option<HostApi<'api>> {
    Some(HOST.get()?.as_ref())
}

fn parse_replace_request(args_json: &str) -> Result<ReplaceRequest, i32> {
    let mut args = serde_json::from_str::<Vec<serde_json::Value>>(args_json).map_err(|_| -43)?;
    let caller = args
        .pop()
        .and_then(|value| serde_json::from_value::<CallerContext>(value).ok())
        .filter(|caller| caller.is_caller);
    let request = if args.len() >= 2 {
        let character = args.first().ok_or(-44)?;
        let payload = args.get(1).ok_or(-44)?;
        let entry = character
            .get("movesetLinkdataEntry")
            .and_then(serde_json::Value::as_u64)
            .ok_or(-45)? as u32;
        let payload_file = match payload {
            serde_json::Value::String(value) => value.clone(),
            serde_json::Value::Object(_) => payload
                .get("payloadFile")
                .or_else(|| payload.get("payload_file"))
                .and_then(serde_json::Value::as_str)
                .ok_or(-45)?
                .to_string(),
            _ => return Err(-45),
        };
        ReplaceRequest {
            entry,
            payload_file,
            caller,
        }
    } else {
        let value = args.into_iter().next().ok_or(-44)?;
        let mut request = serde_json::from_value::<ReplaceRequest>(value).map_err(|_| -45)?;
        request.caller = caller;
        request
    };
    if request.entry == 0 || request.payload_file.trim().is_empty() {
        return Err(-46);
    }
    Ok(request)
}

fn resolve_mod_relative_path(root: &Path, payload_file: &str) -> Option<PathBuf> {
    let payload = Path::new(payload_file);
    if payload.is_absolute() {
        return None;
    }
    if payload
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(root.join(payload))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceRequest {
    entry: u32,
    #[serde(alias = "payload_file")]
    payload_file: String,
    #[serde(skip)]
    caller: Option<CallerContext>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CallerContext {
    #[serde(rename = "__oppw4Caller")]
    is_caller: bool,
    mod_id: String,
    root: String,
    #[allow(dead_code)]
    zip_root: String,
    is_zip: bool,
}
