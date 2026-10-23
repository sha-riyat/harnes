#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod attention;
mod budget;
mod commands;
mod crew_ctl;
mod experiments;
mod host;
mod lookout;
mod methods;
mod persist;
mod projects;
mod pump;
mod runs;
mod safety;
mod state;
mod switch;
mod trace;

use tauri::Manager;

fn main() {
    // The cockpit binary is also the native guard. Claude Code hooks invoke
    // `harnes __hook` so a packaged app never depends on curl or a second file.
    if std::env::args().nth(1).as_deref() == Some("__hook") {
        std::process::exit(harnes_guards::hook_main());
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // OS packs ship inside harnes: install them before projects are equipped.
            if let Err(e) = harnes_os::install_bundled(&harnes_os::Library::open_default()) {
                tracing::warn!("OS packs not installed: {e:#}");
            }
            let handle = app.handle().clone();
            let st = tauri::async_runtime::block_on(state::AppState::new(handle))?;
            pump::start(st.clone());
            lookout::start(st.clone());
            crew_ctl::install_control(&st);
            trace::install(&st);
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
            budget::budget_get,
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
            runs::run_view,
            runs::trace_events,
            runs::trace_runs,
            runs::trace_summary,
            runs::run_decide,
            runs::memory_candidates,
            runs::memory_records,
            runs::memory_stats,
            runs::memory_review,
            runs::memory_propose,
            runs::harness_get,
            safety::profile_get,
            safety::profile_set,
            safety::trust_report,
            safety::trust_accept,
            runs::harness_save,
            runs::guards_get,
            runs::guards_save,
            runs::guards_reload,
            runs::eval_list,
            runs::eval_run,
            runs::tune_plan,
            experiments::eval_run_k,
            experiments::experiment_record,
            experiments::experiment_library,
            runs::worktree_list,
            runs::worktree_remove,
            runs::handoffs,
            commands::lookout,
            methods::os_list,
            methods::os_pack,
            methods::os_read,
            methods::os_import,
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
