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
  loader: Option<instance_manager::LoaderKind>,
  loader_version: Option<String>,
) -> Result<Instance, String> {
  state
    .launcher
    .instances_create(&name, &version, loader.unwrap_or(instance_manager::LoaderKind::Vanilla), loader_version)
    .await
    .map_err(user_message)
}

/// Loaders disponibles (siempre incluye `vanilla`).
#[tauri::command]
pub async fn loaders_supported(state: State<'_, AppState>) -> Result<Vec<instance_manager::LoaderKind>, String> {
  Ok(state.launcher.supported_loaders())
}

/// Versiones del loader para una version de Minecraft.
#[tauri::command]
pub async fn loaders_list_versions(
  state: State<'_, AppState>,
  loader: instance_manager::LoaderKind,
  minecraft_version: String,
) -> Result<Vec<loaders::LoaderVersion>, String> {
  state.launcher.loader_versions(loader, &minecraft_version).await.map_err(user_message)
}

#[tauri::command]
pub async fn instances_update(
  state: State<'_, AppState>,
  id: String,
  changes: instance_manager::InstanceUpdate,
) -> Result<Instance, String> {
  state.launcher.instances_update(&id, changes).await.map_err(user_message)
}

#[tauri::command]
pub async fn instances_delete(state: State<'_, AppState>, id: String) -> Result<(), String> {
  state.launcher.instances_delete(&id).await.map_err(user_message)
}

#[tauri::command]
pub async fn instances_duplicate(state: State<'_, AppState>, id: String) -> Result<Instance, String> {
  state.launcher.instances_duplicate(&id).await.map_err(user_message)
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

/// Verifica y repara la instalacion; el avance llega por `repair://progress`.
#[tauri::command]
pub async fn instances_repair(
  state: State<'_, AppState>,
  id: String,
) -> Result<launcher_core::RepairSummary, String> {
  state.launcher.repair_instance(&id).await.map_err(user_message)
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

#[tauri::command]
pub async fn settings_get(state: State<'_, AppState>) -> Result<launcher_core::SettingsSnapshot, String> {
  Ok(state.launcher.settings_snapshot())
}

#[tauri::command]
pub async fn settings_update(
  state: State<'_, AppState>,
  settings: launcher_core::Settings,
) -> Result<launcher_core::SettingsSnapshot, String> {
  state.launcher.settings_update(settings).await.map_err(user_message)?;
  Ok(state.launcher.settings_snapshot())
}

#[tauri::command]
pub async fn mods_list(state: State<'_, AppState>, id: String) -> Result<launcher_core::ModsOverview, String> {
  state.launcher.mods_list(&id).await.map_err(user_message)
}

#[tauri::command]
pub async fn mods_set_enabled(
  state: State<'_, AppState>,
  id: String,
  file: String,
  enabled: bool,
) -> Result<(), String> {
  state.launcher.mods_set_enabled(&id, &file, enabled).await.map_err(user_message)
}

#[tauri::command]
pub async fn mods_remove(state: State<'_, AppState>, id: String, file: String) -> Result<(), String> {
  state.launcher.mods_remove(&id, &file).await.map_err(user_message)
}

/// Abre el selector de archivos del sistema; el frontend nunca elige rutas por su cuenta.
/// Devuelve `None` si el usuario cancela.
#[tauri::command]
pub async fn mods_add_local(
  app: tauri::AppHandle,
  state: State<'_, AppState>,
  id: String,
) -> Result<Option<mods::ModEntry>, String> {
  use tauri_plugin_dialog::DialogExt;
  let picked = app
    .dialog()
    .file()
    .add_filter("Mods de Minecraft", &["jar"])
    .blocking_pick_file();
  let Some(file) = picked else { return Ok(None) };
  let path = file.into_path().map_err(|_| "No se pudo leer el archivo elegido.".to_string())?;
  state.launcher.mods_add_local(&id, &path).await.map(Some).map_err(user_message)
}

#[tauri::command]
pub async fn mods_search(
  state: State<'_, AppState>,
  id: String,
  query: String,
  page: Option<u32>,
) -> Result<Vec<mods::ProjectHit>, String> {
  state.launcher.mods_search(&id, &query, page.unwrap_or(0)).await.map_err(user_message)
}

#[tauri::command]
pub async fn mods_install(
  state: State<'_, AppState>,
  id: String,
  project_id: String,
) -> Result<Vec<mods::InstalledMod>, String> {
  state.launcher.mods_install(&id, &project_id).await.map_err(user_message)
}

#[tauri::command]
pub async fn mods_check_updates(state: State<'_, AppState>, id: String) -> Result<Vec<mods::UpdateInfo>, String> {
  state.launcher.mods_check_updates(&id).await.map_err(user_message)
}

#[tauri::command]
pub async fn mods_update(state: State<'_, AppState>, id: String, file: String) -> Result<mods::InstalledMod, String> {
  state.launcher.mods_update(&id, &file).await.map_err(user_message)
}

#[tauri::command]
pub async fn mods_rollback(state: State<'_, AppState>, id: String, file: String) -> Result<(), String> {
  state.launcher.mods_rollback(&id, &file).await.map_err(user_message)
}

/// Abre el selector de archivos y crea una instancia a partir del `.mrpack` elegido.
/// Devuelve `None` si el usuario cancela. El avance llega por `install://progress` (fase `modpack`).
#[tauri::command]
pub async fn modpacks_import(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Option<Instance>, String> {
  use tauri_plugin_dialog::DialogExt;
  let picked = app
    .dialog()
    .file()
    .add_filter("Modpack de Modrinth", &["mrpack"])
    .blocking_pick_file();
  let Some(file) = picked else { return Ok(None) };
  let path = file.into_path().map_err(|_| "No se pudo leer el archivo elegido.".to_string())?;
  state.launcher.modpack_import(&path).await.map(Some).map_err(user_message)
}

/// Pide dónde guardar el `.mrpack` y exporta la instancia. `None` si el usuario cancela.
#[tauri::command]
pub async fn modpacks_export(
  app: tauri::AppHandle,
  state: State<'_, AppState>,
  id: String,
) -> Result<Option<launcher_core::ExportSummary>, String> {
  use tauri_plugin_dialog::DialogExt;
  let name = state
    .launcher
    .instances_list()
    .await
    .map_err(user_message)?
    .into_iter()
    .find(|i| i.id == id)
    .map(|i| i.name)
    .unwrap_or_else(|| "modpack".to_string());
  let picked = app
    .dialog()
    .file()
    .add_filter("Modpack de Modrinth", &["mrpack"])
    .set_file_name(format!("{name}.mrpack"))
    .blocking_save_file();
  let Some(file) = picked else { return Ok(None) };
  let path = file.into_path().map_err(|_| "No se pudo usar esa ubicación.".to_string())?;
  state.launcher.modpack_export(&id, &path).await.map(Some).map_err(user_message)
}

#[tauri::command]
pub async fn themes_list(state: State<'_, AppState>) -> Result<Vec<themes::ThemeEntry>, String> {
  Ok(state.launcher.themes_list().await)
}

/// Abre el selector de archivos, valida el `.json` del tema y lo guarda. `None` si se cancela.
#[tauri::command]
pub async fn themes_import(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<Option<themes::Theme>, String> {
  use tauri_plugin_dialog::DialogExt;
  let picked = app
    .dialog()
    .file()
    .add_filter("Tema de DisorderLauncher", &["json"])
    .blocking_pick_file();
  let Some(file) = picked else { return Ok(None) };
  let path = file.into_path().map_err(|_| "No se pudo leer el archivo elegido.".to_string())?;
  // Se limita la lectura: un tema legitimo pesa pocos KB.
  let meta = std::fs::metadata(&path).map_err(|_| "No se pudo leer el archivo elegido.".to_string())?;
  if meta.len() as usize > themes::MAX_THEME_BYTES {
    return Err("Ese archivo es demasiado grande para ser un tema.".to_string());
  }
  let data = std::fs::read(&path).map_err(|_| "No se pudo leer el archivo elegido.".to_string())?;
  state.launcher.themes_import(&data).await.map(Some).map_err(user_message)
}

#[tauri::command]
pub async fn themes_remove(state: State<'_, AppState>, id: String) -> Result<(), String> {
  state.launcher.themes_remove(&id).await.map_err(user_message)
}

#[tauri::command]
pub async fn microsoft_status(state: State<'_, AppState>) -> Result<launcher_core::MicrosoftStatus, String> {
  Ok(state.launcher.microsoft_status())
}

/// Abre el navegador del sistema en la pagina de inicio de sesion de Microsoft. El resultado
/// llega por `auth://state`; los tokens nunca pasan por el frontend.
#[tauri::command]
pub async fn accounts_microsoft_begin_login(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
  use tauri_plugin_opener::OpenerExt;
  let start = state.launcher.microsoft_begin_login().await.map_err(user_message)?;
  // Solo se abre una URL HTTPS construida por el backend.
  if !start.auth_url.starts_with("https://") {
    state.launcher.microsoft_cancel_login();
    return Err("No se pudo abrir el inicio de sesion de forma segura.".to_string());
  }
  if app.opener().open_url(&start.auth_url, None::<&str>).is_err() {
    state.launcher.microsoft_cancel_login();
    return Err("No se pudo abrir tu navegador.".to_string());
  }
  Ok(())
}

#[tauri::command]
pub async fn accounts_microsoft_cancel_login(state: State<'_, AppState>) -> Result<(), String> {
  state.launcher.microsoft_cancel_login();
  Ok(())
}

/// Estado de una cuenta (para Microsoft puede consultar la red).
#[tauri::command]
pub async fn accounts_auth_state(state: State<'_, AppState>, id: String) -> Result<identity_api::AuthState, String> {
  state.launcher.accounts_auth_state(&id).await.map_err(user_message)
}

#[tauri::command]
pub async fn accounts_remove(state: State<'_, AppState>, id: String) -> Result<(), String> {
  state.launcher.accounts_remove(&id).await.map_err(user_message)
}
