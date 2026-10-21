//! Backend del IDE. No enlaza el intérprete acm: lo lanza como proceso y habla con él
//! mediante su salida `--json`, igual que lo haría cualquier otro cliente.

mod engine;
mod files;
mod launch;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let launch = match launch::from_args() {
        launch::Start::Window(l) => l,
        launch::Start::Exit(code) => std::process::exit(code),
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(engine::Engine::default())
        .manage(launch)
        .setup(|app| {
            launch::open_initial(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            engine::engine_locate,
            engine::engine_check,
            engine::engine_info,
            engine::engine_run,
            engine::engine_stop,
            engine::engine_input,
            files::fs_list,
            files::fs_read,
            files::fs_write,
            files::fs_create,
            files::fs_rename,
            files::fs_delete,
            launch::launch_info,
            launch::take_open_request,
            launch::open_in_ide,
            launch::acmx_associated,
            launch::associate_acmx,
        ])
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar acm IDE");
}
