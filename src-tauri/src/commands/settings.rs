// 设置命令

use tauri::State;

use crate::config;
use crate::db::{models::*, Database};
use crate::error::Result;

#[tauri::command]
pub async fn get_settings(db: State<'_, Database>) -> Result<SettingsDto> {
    config::get_settings(db.pool()).await
}

#[tauri::command]
pub async fn update_settings(
    db: State<'_, Database>,
    form: SettingsForm,
) -> Result<()> {
    config::update_settings(db.pool(), &form).await
}
