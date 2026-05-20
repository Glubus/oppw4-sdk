use plugin_sdk::{export_plugin, Plugin, PluginContext, PluginResult};

struct LogExample;

impl Plugin for LogExample {
    const ID: &'static str = "log_example";

    fn init(context: PluginContext<'_>) -> PluginResult<()> {
        context.log("log_example initialized");
        if let Some(status) = context.game_status() {
            context.log(format!(
                "game status phase={} flags=0x{:x}",
                status.phase, status.flags
            ));
        }
        Ok(())
    }
}

export_plugin!(LogExample);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_stable_plugin_id() {
        assert_eq!(LogExample::ID, "log_example");
    }
}
