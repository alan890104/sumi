/// Credential storage — platform-specific implementations.
///
/// macOS: uses the `security` CLI which inherits the calling app's keychain
///        access without triggering extra permission dialogs.
/// Other: uses the `keyring` crate (Windows Credential Manager, etc.).

const SERVICE: &str = "sumi";

fn keychain_service(provider: &str) -> String {
    format!("{}-api-key-{}", SERVICE, provider)
}

// ── macOS: `security` CLI ──────────────────────────────────────────

#[cfg(target_os = "macos")]
pub fn save(provider: &str, key: &str) -> Result<(), String> {
    let service = keychain_service(provider);
    // Delete first to avoid "already exists" error
    let _ = std::process::Command::new("security")
        .args(["delete-generic-password", "-s", &service, "-a", SERVICE])
        .output();
    let out = std::process::Command::new("security")
        .args([
            "add-generic-password",
            "-s", &service,
            "-a", SERVICE,
            "-w", key,
            "-U",
        ])
        .output()
        .map_err(|e| format!("Failed to run security CLI: {}", e))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "security add-generic-password failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

#[cfg(target_os = "macos")]
pub fn load(provider: &str) -> Result<String, String> {
    let service = keychain_service(provider);
    let out = std::process::Command::new("security")
        .args(["find-generic-password", "-s", &service, "-a", SERVICE, "-w"])
        .output()
        .map_err(|e| format!("Failed to run security CLI: {}", e))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        // Item not found → return empty string (not an error)
        Ok(String::new())
    }
}

#[cfg(target_os = "macos")]
pub fn delete(provider: &str) -> Result<(), String> {
    let service = keychain_service(provider);
    let out = std::process::Command::new("security")
        .args(["delete-generic-password", "-s", &service, "-a", SERVICE])
        .output()
        .map_err(|e| format!("Failed to run security CLI: {}", e))?;
    if out.status.success() || String::from_utf8_lossy(&out.stderr).contains("could not be found") {
        Ok(())
    } else {
        Err(format!(
            "security delete-generic-password failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ))
    }
}

// ── Non-macOS: `keyring` crate ─────────────────────────────────────

#[cfg(not(target_os = "macos"))]
pub fn save(provider: &str, key: &str) -> Result<(), String> {
    entry(provider)?
        .set_password(key)
        .map_err(|e| format!("Keyring save failed: {}", e))
}

#[cfg(not(target_os = "macos"))]
pub fn load(provider: &str) -> Result<String, String> {
    match entry(provider)?.get_password() {
        Ok(key) => Ok(key),
        Err(keyring::Error::NoEntry) => Ok(String::new()),
        Err(e) => Err(format!("Keyring load failed: {}", e)),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn delete(provider: &str) -> Result<(), String> {
    match entry(provider)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Keyring delete failed: {}", e)),
    }
}

#[cfg(not(target_os = "macos"))]
fn entry(provider: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(&keychain_service(provider), SERVICE)
        .map_err(|e| format!("Keyring error: {}", e))
}
