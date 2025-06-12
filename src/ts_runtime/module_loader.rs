use deno_ast::{MediaType, ParseParams, SourceMapOption, TranspileModuleOptions};
use deno_core::{
    ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
    RequestedModuleType, ResolutionKind, error::ModuleLoaderError,
};

pub struct TsModuleLoader;

impl ModuleLoader for TsModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, ModuleLoaderError> {
        deno_core::resolve_import(specifier, referrer).map_err(|e| ModuleLoaderError::from(e))
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: RequestedModuleType,
    ) -> ModuleLoadResponse {
        let module_specifier = module_specifier.clone();
        
        let fut = async move {
            let path = module_specifier
                .to_file_path()
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Only file:// URLs are supported"))?;

            let media_type = MediaType::from_path(&path);
            let (module_type, should_transpile) = match media_type {
                MediaType::JavaScript | MediaType::Mjs => (ModuleType::JavaScript, false),
                MediaType::TypeScript
                | MediaType::Mts
                | MediaType::Tsx
                | MediaType::Jsx => (ModuleType::JavaScript, true),
                MediaType::Json => (ModuleType::Json, false),
                _ => (ModuleType::JavaScript, false),
            };

            let code = std::fs::read_to_string(&path)
                .map_err(|e| ModuleLoaderError::from(e))?;
            let code = if should_transpile {
                let parsed = deno_ast::parse_module(ParseParams {
                    specifier: module_specifier.clone(),
                    text: code.into(),
                    media_type,
                    capture_tokens: false,
                    scope_analysis: false,
                    maybe_syntax: None,
                })
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Parse error: {:?}", e)))?;

                let transpiled = parsed.transpile(
                    &deno_ast::TranspileOptions::default(),
                    &TranspileModuleOptions::default(),
                    &deno_ast::EmitOptions {
                        source_map: SourceMapOption::None,
                        ..Default::default()
                    },
                )
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Transpile error: {:?}", e)))?;

                transpiled.into_source().text
            } else {
                code
            };

            let module_source = ModuleSource::new(
                module_type,
                ModuleSourceCode::String(code.into()),
                &module_specifier,
                None,
            );

            Ok(module_source)
        };

        ModuleLoadResponse::Async(Box::pin(fut))
    }
}