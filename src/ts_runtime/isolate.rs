use anyhow::Result;
use deno_core::{
    JsRuntime, RuntimeOptions, FastString, ModuleSpecifier,
};
use std::path::Path;
use std::rc::Rc;
use serde_json::Value;
use super::module_loader::TsModuleLoader;
use super::ops;

pub struct TypeScriptIsolate {
    runtime: JsRuntime,
}

impl TypeScriptIsolate {
    pub async fn new(_script_path: &Path) -> Result<Self> {
        // Define the extension declaratively
        deno_core::extension!(
            aish_ops,
            ops = [
                ops::op_get_shell_info,
                ops::op_get_env,
                ops::op_set_env,
                ops::op_log,
                ops::op_console_log,
                ops::op_execute_command,
                ops::op_register_agent_tool,
                ops::op_get_agent_tools,
                ops::op_call_agent_tool,
            ],
        );
        
        // Create JsRuntime with module loader for TypeScript support
        let mut runtime = JsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(TsModuleLoader)),
            extensions: vec![aish_ops::init()],
            ..Default::default()
        });
        
        // Initialize console object
        let console_init = r#"
            globalThis.console = {
                log: (...args) => {
                    const message = args.map(arg => {
                        if (typeof arg === 'string') {
                            return arg;
                        } else if (typeof arg === 'object' && arg !== null) {
                            try {
                                return JSON.stringify(arg, null, 2);
                            } catch {
                                return '[object]';
                            }
                        } else {
                            return String(arg);
                        }
                    }).join(' ');
                    Deno.core.ops.op_console_log(message);
                },
                error: (...args) => {
                    const message = "ERROR: " + args.map(arg => {
                        if (typeof arg === 'string') {
                            return arg;
                        } else if (typeof arg === 'object' && arg !== null) {
                            try {
                                return JSON.stringify(arg, null, 2);
                            } catch {
                                return '[object]';
                            }
                        } else {
                            return String(arg);
                        }
                    }).join(' ');
                    Deno.core.ops.op_console_log(message);
                },
                warn: (...args) => {
                    const message = "WARN: " + args.map(arg => {
                        if (typeof arg === 'string') {
                            return arg;
                        } else if (typeof arg === 'object' && arg !== null) {
                            try {
                                return JSON.stringify(arg, null, 2);
                            } catch {
                                return '[object]';
                            }
                        } else {
                            return String(arg);
                        }
                    }).join(' ');
                    Deno.core.ops.op_console_log(message);
                }
            };
        "#;
        
        runtime.execute_script("console_init", FastString::from(console_init.to_string()))?;
        
        Ok(Self { runtime })
    }

    pub async fn execute(&mut self, script_path: &Path) -> Result<()> {
        // Convert path to module specifier
        let module_specifier = ModuleSpecifier::from_file_path(script_path)
            .map_err(|_| anyhow::anyhow!("Failed to convert path to module specifier"))?;
        
        // Load and execute the module (TypeScript will be transpiled automatically)
        let module_id = self.runtime.load_main_es_module(&module_specifier).await?;
        
        // Evaluate the module
        let result = self.runtime.mod_evaluate(module_id);
        self.runtime.run_event_loop(Default::default()).await?;
        result.await?;
        
        Ok(())
    }

    pub async fn call_function(&mut self, function_name: &str, args: &[Value]) -> Result<Value> {
        let args_str = args.iter()
            .map(|arg| arg.to_string())
            .collect::<Vec<_>>()
            .join(", ");
            
        let script = format!(
            r#"
            (function() {{
                if (typeof globalThis.{} === 'function') {{
                    const result = globalThis.{}({});
                    return JSON.stringify(result);
                }} else {{
                    throw new Error('Function {} not found or not a function');
                }}
            }})()
            "#,
            function_name, function_name, args_str, function_name
        );

        let result = self.runtime.execute_script("call_function", FastString::from(script))?;
        let scope = &mut self.runtime.handle_scope();
        let local_result = deno_core::v8::Local::new(scope, result);
        let result_string = serde_v8::from_v8::<String>(scope, local_result)?;
        let json_value: Value = serde_json::from_str(&result_string)?;
        print!("{}", json_value);
        Ok(json_value)
    }

    pub async fn get_export(&mut self, export_name: &str) -> Result<Value> {
        let script = format!(
            r#"
            (function() {{
                if (typeof globalThis.{} !== 'undefined') {{
                    return JSON.stringify(globalThis.{});
                }} else {{
                    throw new Error('Export {} not found');
                }}
            }})()
            "#,
            export_name, export_name, export_name
        );

        let result = self.runtime.execute_script("get_export", FastString::from(script))?;
        let scope = &mut self.runtime.handle_scope();
        let local_result = deno_core::v8::Local::new(scope, result);
        let result_string = serde_v8::from_v8::<String>(scope, local_result)?;
        let json_value: Value = serde_json::from_str(&result_string)?;
        Ok(json_value)
    }
}