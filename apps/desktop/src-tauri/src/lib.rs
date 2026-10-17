mod commands;
mod events;
mod state;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  // Los servicios del core usan `tracing`; nunca registran tokens (ver redaccion en launch).
  let _ = tracing_subscriber::fmt().with_target(false).try_init();

  tauri::Builder::default()
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }

      let data_dir = app.path().app_data_dir()?;
      app.manage(state::AppState::new(data_dir, app.handle().clone()));
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      commands::accounts_list,
      commands::accounts_default,
      commands::accounts_create_local,
      commands::accounts_set_default,
      commands::instances_list,
      commands::instances_create,
      commands::instances_launch,
      commands::instances_stop,
      commands::instances_repair,
      commands::minecraft_list_versions,
    ])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
