use tauri_plugin_notification::NotificationExt;

pub async fn check_for_updates(
    app: tauri::AppHandle,
    should_show_no_update_notification: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    use tauri_plugin_updater::UpdaterExt;

    if let Some(update) = app.updater()?.check().await? {
        let mut downloaded = 0;

        app.notification()
            .builder()
            .title("Rune Update")
            .body(&format!("Downloading update {}...", update.version))
            .show()?;

        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    log::info!("Downloaded {} from {:?}", downloaded, content_length);
                },
                || {
                    log::info!("Download finished");

                    if let Err(e) = app
                        .notification()
                        .builder()
                        .title("Rune Update")
                        .body("Update downloaded. Restarting application...")
                        .show()
                    {
                        log::error!("Failed to show notification: {}", e);
                    }
                },
            )
            .await?;

        log::info!("Update installed, restarting application");
        app.restart();
    } else {
        log::info!("No updates available");
        if should_show_no_update_notification {
            app.notification()
                .builder()
                .title("Rune")
                .body("You're already on the latest version")
                .show()
                .unwrap_or_else(|e| log::error!("Failed to show notification: {}", e));
        }
        Ok(false)
    }
}
