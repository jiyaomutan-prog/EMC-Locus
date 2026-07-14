use crate::{render_json, AgentError};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoreLocalFileInput {
    pub original_filename: String,
    pub mime_type: String,
    pub content_base64: String,
}

#[derive(Clone, Copy)]
pub(crate) struct FileStorePolicy {
    pub namespace: &'static str,
    pub invalid_code: &'static str,
    pub too_large_code: &'static str,
    pub store_failed_code: &'static str,
}

const MAX_LOCAL_FILE_SIZE_BYTES: usize = 20 * 1024 * 1024;

pub(crate) fn store_content_addressed_file(
    storage_root: &Path,
    input: StoreLocalFileInput,
    policy: FileStorePolicy,
) -> Result<String, AgentError> {
    let filename = input.original_filename.trim();
    if filename.is_empty()
        || filename.len() > 255
        || filename.contains(['/', '\\', '\0'])
        || filename == "."
        || filename == ".."
    {
        return Err(AgentError::new(
            policy.invalid_code,
            "original_filename must be a safe file name",
        ));
    }
    let mime_type = input.mime_type.trim();
    if mime_type.is_empty()
        || mime_type.len() > 127
        || !mime_type.contains('/')
        || mime_type.chars().any(char::is_whitespace)
    {
        return Err(AgentError::new(
            policy.invalid_code,
            "mime_type must be a valid non-empty media type",
        ));
    }
    let content = BASE64_STANDARD
        .decode(input.content_base64.trim())
        .map_err(|_| AgentError::new(policy.invalid_code, "content_base64 is invalid"))?;
    if content.is_empty() {
        return Err(AgentError::new(
            policy.invalid_code,
            "the uploaded file must not be empty",
        ));
    }
    if content.len() > MAX_LOCAL_FILE_SIZE_BYTES {
        return Err(AgentError::new(
            policy.too_large_code,
            format!(
                "the uploaded file exceeds the {} byte limit",
                MAX_LOCAL_FILE_SIZE_BYTES
            ),
        ));
    }

    let digest = format!("{:x}", Sha256::digest(&content));
    let relative_storage_key = format!("objects/{}/{}/{}", policy.namespace, &digest[..2], digest);
    let object_path = storage_root.join(&relative_storage_key);
    let parent = object_path
        .parent()
        .ok_or_else(|| AgentError::new(policy.store_failed_code, "object path has no parent"))?;
    fs::create_dir_all(parent)
        .map_err(|error| AgentError::new(policy.store_failed_code, error.to_string()))?;
    if !object_path.exists() {
        let temporary_path = parent.join(format!(".{digest}.upload"));
        fs::write(&temporary_path, &content)
            .map_err(|error| AgentError::new(policy.store_failed_code, error.to_string()))?;
        fs::rename(&temporary_path, &object_path)
            .map_err(|error| AgentError::new(policy.store_failed_code, error.to_string()))?;
    }

    Ok(render_json(&json!({
        "file": {
            "object_id": format!("sha256:{digest}"),
            "original_filename": filename,
            "mime_type": mime_type,
            "size_bytes": content.len(),
            "sha256": digest,
            "storage_key": relative_storage_key
        }
    })))
}
