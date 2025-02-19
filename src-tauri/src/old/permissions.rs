use tauri::command;

// Check Accessibility Permissions.
#[command]
pub async fn check_accessibility_permissions() -> bool {
    #[cfg(target_os = "macos")]
    return macos_accessibility_client::accessibility::application_is_trusted();

    #[cfg(not(target_os = "macos"))]
    return true;
}

// Request Accessibility Permissions.
#[command]
pub async fn request_accessibility_permissions() -> bool {
    #[cfg(target_os = "macos")]
    return macos_accessibility_client::accessibility::application_is_trusted_with_prompt();

    #[cfg(not(target_os = "macos"))]
    return true;
}
