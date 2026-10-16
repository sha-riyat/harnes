#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod attention;
mod commands;
mod crew_ctl;
mod host;
mod lookout;
mod persist;
mod projects;
mod pump;
mod state;
mod switch;

use tauri::Manager;

fn main() {
    tracing_subscriber::fmt().with_env_filter(tracing_subscriber::EnvFilter::from_default_env()).init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let st = tauri::async_runtime::block_on(state::AppState::new(handle))?;
            pump::start(st.clone());
            lookout::start(st.clone());
            crew_ctl::install_control(&st);
            attention::install(&st);
            app.manage(attention::Prompts::default());
            app.manage(st.clone());
            // Restore the last session, then keep it saved.
            tauri::async_runtime::spawn(async move {
                persist::restore(st.clone()).await;
                persist::start_autosave(st);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            projects::app_info,
            projects::open_project,
            projects::projects_list,
            projects::project_activate,
            projects::project_close,
            commands::pane_spawn,
            commands::pane_write,
            commands::pane_resize,
            commands::pane_scroll,
            commands::pane_close,
            commands::pane_list,
            commands::frames_subscribe,
            commands::pane_refresh,
            commands::crew_start,
            commands::crew_cancel,
            projects::stats,
            commands::lookout,
            attention::permissions_pending,
            attention::permission_answer,
            attention::screen_prompts,
            attention::screen_answer,
            attention::conversation,
            crew_ctl::orchestrator_say,
            crew_ctl::threads_list,
            crew_ctl::thread_read,
            crew_ctl::thread_post,
            crew_ctl::threads_recent,
            switch::providers_list,
            switch::pane_switch_provider,
        ])
        .on_window_event(|win, ev| {
            if let tauri::WindowEvent::Destroyed = ev {
                if let Some(st) = win.app_handle().try_state::<state::Shared>() {
                    persist::save_now(&st);
                    st.panes.close_all();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running harnes");
}
