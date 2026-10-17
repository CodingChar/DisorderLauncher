mod commands;
mod events;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Los servicios del core usan `tracing`; nunca registran tokens (ver redaccion en launch).
  // `set_global_default` (y no `try_init`) para no registrar el puente tracing->log, que chocaria con
  // `tauri_plugin_log` en modo desarrollo (dos loggers globales = panico al iniciar).
  let _ = tracing::subscriber::set_global_default(tracing_subscriber::fmt().with_target(false).finish());

  tauri::Builder::default()
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
      commands::modpacks_import,
      commands::modpacks_export,
      commands::mods_list,
      commands::mods_set_enabled,
      commands::mods_remove,
      commands::mods_add_local,
      commands::mods_search,
      commands::mods_install,
      commands::mods_check_updates,
      commands::mods_update,
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
