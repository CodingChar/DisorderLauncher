//! Comandos IPC. Son finos a proposito: delegan en `launcher-core` y devuelven
//! mensajes ya pensados para el usuario final (nada de terminos internos).

use crate::state::AppState;
use identity_api::AccountRecord;
use instance_manager::Instance;
use minecraft_meta::VersionEntry;
use tauri::State;

fn user_message(e: launcher_core::LauncherError) -> String {
  e.to_string()
}

#[tauri::command]
pub async fn accounts_list(state: State<'_, AppState>) -> Result<Vec<AccountRecord>, String> {
  state.launcher.accounts_list().await.map_err(user_message)
}

#[tauri::command]
pub async fn accounts_default(state: State<'_, AppState>) -> Result<Option<String>, String> {
  state.launcher.accounts_default().await.map_err(user_message)
}

#[tauri::command]
pub async fn accounts_create_local(
  state: State<'_, AppState>,
  name: String,
) -> Result<AccountRecord, String> {
  state.launcher.accounts_create_local(&name).await.map_err(user_message)
}

#[tauri::command]
pub async fn accounts_set_default(state: State<'_, AppState>, id: String) -> Result<(), String> {
  state.launcher.accounts_set_default(&id).await.map_err(user_message)
}

#[tauri::command]
pub async fn instances_list(state: State<'_, AppState>) -> Result<Vec<Instance>, String> {
  state.launcher.instances_list().await.map_err(user_message)
}

#[tauri::command]
pub async fn instances_create(
  state: State<'_, AppState>,
  name: String,
  version: String,
) -> Result<Instance, String> {
  state.launcher.instances_create(&name, &version).await.map_err(user_message)
}

/// Inicia en segundo plano; el avance llega por `install://progress` y `launch://state`.
#[tauri::command]
pub async fn instances_launch(state: State<'_, AppState>, id: String) -> Result<(), String> {
  state.launcher.launch(&id).await.map_err(user_message)
}

#[tauri::command]
pub async fn instances_stop(state: State<'_, AppState>, id: String) -> Result<(), String> {
  state.launcher.stop(&id).map_err(user_message)
}

/// Por defecto solo versiones estables; `include_all` agrega snapshots y antiguas.
#[tauri::command]
pub async fn minecraft_list_versions(
  state: State<'_, AppState>,
  include_all: Option<bool>,
) -> Result<Vec<VersionEntry>, String> {
  state
    .launcher
    .list_versions(include_all.unwrap_or(false))
    .await
    .map_err(user_message)
}
