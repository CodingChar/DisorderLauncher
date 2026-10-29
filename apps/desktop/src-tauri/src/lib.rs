mod commands;
mod events;
mod state;

use tauri::Manager;

/// Registro en `<logs>/DisorderLauncher.log`. Si pasa de 2 MB se aparta como `.old` (solo se guarda uno).
/// Nunca incluye tokens: los servicios del core no los registran.
fn init_file_logging(dir: &std::path::Path) {
  const MAX_BYTES: u64 = 2 * 1024 * 1024;
  if std::fs::create_dir_all(dir).is_err() {
    return;
  }
  let path = dir.join("DisorderLauncher.log");
  if std::fs::metadata(&path).is_ok_and(|m| m.len() > MAX_BYTES) {
    let _ = std::fs::rename(&path, dir.join("DisorderLauncher.log.old"));
  }
  let Ok(file) = std::fs::OpenOptions::new().create(true).append(true).open(&path) else { return };
  let _ = tracing::subscriber::set_global_default(
    tracing_subscriber::fmt()
      .with_target(false)
      .with_ansi(false)
      .with_writer(std::sync::Mutex::new(file))
      .finish(),
  );
  tracing::info!(version = env!("CARGO_PKG_VERSION"), "DisorderLauncher iniciado");
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Los servicios del core usan `tracing`; nunca registran tokens (ver redaccion en launch).
  // `set_global_default` (y no `try_init`) para no registrar el puente tracing->log, que chocaria con
  // `tauri_plugin_log` en modo desarrollo (dos loggers globales = panico al iniciar).
  // En desarrollo los registros van a la consola; en la version instalada no hay consola, asi que se
  // guardan en un archivo (ver `init_file_logging`) para poder saber por que fallo algo.
  if cfg!(debug_assertions) {
    let _ = tracing::subscriber::set_global_default(tracing_subscriber::fmt().with_target(false).finish());
  }

  tauri::Builder::default()
    // Una sola ventana: abrir el launcher otra vez solo trae al frente la que ya existe
    // (evita lanzar instancias sin control desde varias copias).
    .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
      if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
      }
    }))
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      if !cfg!(debug_assertions) {
        if let Ok(dir) = app.path().app_log_dir() {
          init_file_logging(&dir);
        }
      }

      let data_dir = app.path().app_data_dir()?;
      let state = state::AppState::new(data_dir, app.handle().clone());
      tauri::async_runtime::block_on(state.launcher.load_settings());
      app.manage(state);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::accounts_list,
      commands::accounts_default,
      commands::accounts_create_local,
      commands::accounts_set_default,
      commands::accounts_remove,
      commands::accounts_auth_state,
      commands::accounts_microsoft_begin_login,
      commands::accounts_microsoft_cancel_login,
      commands::microsoft_status,
      commands::instances_list,
      commands::instances_create,
      commands::loaders_supported,
      commands::loaders_list_versions,
      commands::instances_update,
      commands::instances_delete,
      commands::instances_duplicate,
      commands::instances_launch,
      commands::instances_stop,
      commands::instances_repair,
      commands::minecraft_list_versions,
      commands::minecraft_version_catalog,
      commands::external_versions_scan,
      commands::external_versions_pick_dir,
      commands::instances_create_custom,
      commands::instances_optimize_mods,
      commands::instances_jvm_check,
      commands::instances_recommendation,
      commands::modpacks_import,
      commands::modpacks_export,
      commands::dpack_export,
      commands::dpack_import,
      commands::store_summary,
      commands::store_cleanup,
      commands::mods_list,
      commands::mods_set_enabled,
      commands::mods_remove,
      commands::mods_add_local,
      commands::mods_search,
      commands::mods_install,
      commands::mods_check_updates,
      commands::mods_update,
      commands::mods_install_missing,
      commands::mods_rollback,
      commands::skins_list,
      commands::skins_import,
      commands::skins_read,
      commands::skins_rename,
      commands::skins_set_model,
      commands::skins_remove,
      commands::skins_export,
      commands::skins_save,
      commands::accounts_set_skin,
      commands::themes_list,
      commands::themes_import,
      commands::themes_remove,
      commands::startup_notices,
      commands::settings_get,
      commands::settings_update,
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
