use anyhow::Result;
use deno_core::ModuleSpecifier;
use deno_runtime::worker::{MainWorker, WorkerOptions};
use deno_runtime::permissions::{Permissions, PermissionsContainer};
use deno_runtime::BootstrapOptions;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde_json::Value;
use url::Url;

pub struct TypeScriptIsolate {
    worker: MainWorker,
}

impl TypeScriptIsolate {
    pub async fn new(script_path: &Path) -> Result<Self> {
        // Create extension with our ops
        deno_core::extension!(
            aish_extension,
            ops = [
                super::ops::op_get_shell_info,
                super::ops::op_get_env,
                super::ops::op_set_env,
                super::ops::op_log,
                super::ops::op_execute_command,
            ]
        );

        // Convert path to module specifier
        let main_module = Url::from_file_path(script_path)
            .map_err(|_| anyhow::anyhow!("Failed to create module specifier from path: {:?}", script_path))?;

        // Create temporary directories for Deno runtime
        let temp_dir = tempfile::tempdir()?;
        let cache_dir = temp_dir.path().join("cache");
        let origin_dir = temp_dir.path().join("origin");
        std::fs::create_dir_all(&cache_dir)?;
        std::fs::create_dir_all(&origin_dir)?;

        // Create permissions
        let permissions = PermissionsContainer::new(Permissions::allow_all());

        // Create bootstrap options
        let bootstrap_options = BootstrapOptions {
            location: Some(main_module.clone()),
            argv0: Some("aish".to_string()),
            ..BootstrapOptions::default()
        };

        // Create worker options with our extensions
        let worker_options = WorkerOptions {
            bootstrap: bootstrap_options.clone(),
            extensions: vec![aish_extension::init_ops_and_esm()],
            cache_storage_dir: Some(cache_dir),
            origin_storage_dir: Some(origin_dir),
            ..Default::default()
        };

        // Create MainWorker with TypeScript support
        let mut worker = MainWorker::from_options(
            ModuleSpecifier::from(main_module.clone()),
            permissions,
            worker_options,
        );
        
        // Bootstrap the worker
        worker.bootstrap(bootstrap_options);

        Ok(Self { worker })
    }

    pub async fn execute(&mut self, script_path: &Path) -> Result<()> {
        // Convert path to module specifier
        let main_module = ModuleSpecifier::from_file_path(script_path)
            .map_err(|_| anyhow::anyhow!("Failed to create module specifier from path: {:?}", script_path))?;
        
        // Execute the main module (MainWorker handles TypeScript transpilation automatically)
        self.worker.execute_main_module(&main_module).await?;
        
        // Run event loop to completion
        self.worker.run_event_loop(false).await?;
        
        Ok(())
    }


    pub async fn call_function(&mut self, function_name: &str, args: &[Value]) -> Result<Value> {
        let script = format!(
            r#"
            if (typeof {} === 'function') {{
                const result = {}({});
                JSON.stringify(result);
            }} else {{
                throw new Error('Function {} not found or not a function');
            }}
            "#,
            function_name,
            function_name,
            args.iter()
                .map(|arg| arg.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            function_name
        );

        let result = self.worker.js_runtime.execute_script("<anon>", script)?;
        let result_string = {
            let scope = &mut self.worker.js_runtime.handle_scope();
            let local_result = deno_core::v8::Local::new(scope, result);
            serde_v8::from_v8::<String>(scope, local_result)?
        };
        let json_value: Value = serde_json::from_str(&result_string)?;
        Ok(json_value)
    }

    pub async fn get_export(&mut self, export_name: &str) -> Result<Value> {
        let script = format!(
            r#"
            if (typeof {} !== 'undefined') {{
                JSON.stringify({});
            }} else {{
                throw new Error('Export {} not found');
            }}
            "#,
            export_name, export_name, export_name
        );

        let result = self.worker.js_runtime.execute_script("<anon>", script)?;
        let result_string = {
            let scope = &mut self.worker.js_runtime.handle_scope();
            let local_result = deno_core::v8::Local::new(scope, result);
            serde_v8::from_v8::<String>(scope, local_result)?
        };
        let json_value: Value = serde_json::from_str(&result_string)?;
        Ok(json_value)
    }
}