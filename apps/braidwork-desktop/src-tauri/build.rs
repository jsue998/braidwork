fn main() {
    #[cfg(feature = "desktop")]
    {
        let attributes = tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "init_project",
                "open_project",
                "close_project",
                "workspace_snapshot",
                "create_resource",
                "create_agent",
                "create_session",
                "create_task",
                "set_task_status",
                "create_assignment",
                "create_delegation",
                "prepare_capsule",
                "render_capsule",
                "mark_dispatched",
                "ingest_text",
                "ingest_file",
                "assignment_detail",
                "result_detail",
                "open_external_reference",
            ]),
        );
        if let Err(error) = tauri_build::try_build(attributes) {
            eprintln!("Tauri configuration failed: {error}");
            std::process::exit(1);
        }
    }
}
