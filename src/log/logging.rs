use env_logger::Builder as EnvLoggerBuilder;
use env_logger::Logger as EnvLogger;
use log::{LevelFilter, Log, SetLoggerError};

#[cfg(target_arch = "wasm32")]
use web_sys::{console, wasm_bindgen::prelude::JsValue};

#[cfg(target_arch = "wasm32")]
use log::Level;

pub struct LoggerBuilder {
    pub env: EnvLoggerBuilder,
}

impl Default for LoggerBuilder {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        {
            Self::new(LevelFilter::Info)
        }
        #[cfg(not(debug_assertions))]
        {
            Self::new(LevelFilter::Info)
        }
    }
}

impl LoggerBuilder {
    pub fn new(level: LevelFilter) -> Self {
        let mut builder = EnvLoggerBuilder::new();
        builder
            .filter_level(level)
            .filter_module("wgpu_hal", LevelFilter::Off)
            .filter_module("wgpu", LevelFilter::Warn)
            .filter_module("wgpu_core", LevelFilter::Warn)
            .filter_module("naga", LevelFilter::Warn)
            .filter_module("winit", LevelFilter::Warn)
            .filter_module("symphonia_core", LevelFilter::Warn)
            .filter_module("symphonia_bundle_mp3", LevelFilter::Warn);
        Self { env: builder }
    }

    pub fn custom(builder: EnvLoggerBuilder) -> Self {
        Self { env: builder }
    }

    pub fn init(mut self) -> Result<(), SetLoggerError> {
        let logger = Logger {
            env: self.env.build(),
            #[cfg(target_arch = "wasm32")]
            wasm_style: Style::new(),
        };

        let max_level = logger.env.filter();
        let r = log::set_boxed_logger(Box::new(logger));

        if r.is_ok() {
            log::set_max_level(max_level);
        }

        r
    }
}

struct Logger {
    env: EnvLogger,
    #[cfg(target_arch = "wasm32")]
    wasm_style: Style,
}

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.env.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        #[cfg(not(target_arch = "wasm32"))]
        return self.env.log(record);

        #[cfg(target_arch = "wasm32")]
        if self.env.matches(record) {
            let style = &self.wasm_style;
            let message_separator = "\n";
            let s = format!(
                "%c{}%c {}:{}%c{}{}",
                record.level(),
                record.file().unwrap_or_else(|| record.target()),
                record
                    .line()
                    .map_or_else(|| "[Unknown]".to_string(), |line| line.to_string()),
                message_separator,
                record.args(),
            );
            let s = JsValue::from_str(&s);
            let tgt_style = JsValue::from_str(&style.tgt);
            let args_style = JsValue::from_str(&style.args);

            match record.level() {
                Level::Trace => console::debug_4(
                    &s,
                    &JsValue::from(&style.lvl_trace),
                    &tgt_style,
                    &args_style,
                ),
                Level::Debug => console::log_4(
                    &s,
                    &JsValue::from(&style.lvl_debug),
                    &tgt_style,
                    &args_style,
                ),
                Level::Info => {
                    console::info_4(&s, &JsValue::from(&style.lvl_info), &tgt_style, &args_style)
                }
                Level::Warn => {
                    console::warn_4(&s, &JsValue::from(&style.lvl_warn), &tgt_style, &args_style)
                }
                Level::Error => console::error_4(
                    &s,
                    &JsValue::from(&style.lvl_error),
                    &tgt_style,
                    &args_style,
                ),
            }
        }
    }

    fn flush(&self) {}
}

#[cfg(target_arch = "wasm32")]
struct Style {
    lvl_trace: String,
    lvl_debug: String,
    lvl_info: String,
    lvl_warn: String,
    lvl_error: String,
    tgt: String,
    args: String,
}

#[cfg(target_arch = "wasm32")]
impl Style {
    fn new() -> Style {
        let base = String::from("color: white; padding: 0 3px; background:");
        Style {
            lvl_trace: format!("{} gray;", base),
            lvl_debug: format!("{} blue;", base),
            lvl_info: format!("{} green;", base),
            lvl_warn: format!("{} orange;", base),
            lvl_error: format!("{} darkred;", base),
            tgt: String::from("font-weight: bold; color: inherit"),
            args: String::from("background: inherit; color: inherit"),
        }
    }
}
