use std::sync::{OnceLock, RwLock};

use super::{
    bus::{RuntimeDispatchReport, RuntimeEventBus, RuntimeHandlerError},
    events::{RuntimeEvent, RuntimeMutation},
};

static RUNTIME_EVENT_BUS: OnceLock<RwLock<RuntimeEventBus>> = OnceLock::new();

pub(crate) fn register_runtime_handler(
    id: impl Into<String>,
    handler: impl Fn(&RuntimeEvent) -> Result<Vec<RuntimeMutation>, RuntimeHandlerError>
        + Send
        + Sync
        + 'static,
) {
    let mut bus = runtime_event_bus().write().expect("runtime event bus lock");
    bus.register_handler(id, handler);
}

pub(crate) fn dispatch_runtime_event(event: RuntimeEvent) -> RuntimeDispatchReport {
    let Ok(bus) = runtime_event_bus().read() else {
        return RuntimeDispatchReport::default();
    };
    bus.dispatch(&event)
}

fn runtime_event_bus() -> &'static RwLock<RuntimeEventBus> {
    RUNTIME_EVENT_BUS.get_or_init(|| RwLock::new(RuntimeEventBus::new()))
}

#[cfg(test)]
pub(crate) fn reset_runtime_handlers_for_tests() {
    let mut bus = runtime_event_bus().write().expect("runtime event bus lock");
    *bus = RuntimeEventBus::new();
}

#[cfg(test)]
pub(crate) fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .expect("runtime bus test lock")
}

#[cfg(test)]
mod tests {
    use crate::runtime::core::{
        rank::RankValue,
        rewards::{RewardCommitEvent, RewardMutation, RewardState},
    };

    use super::*;

    #[test]
    fn global_bus_dispatches_registered_handler() {
        let _guard = test_lock();
        reset_runtime_handlers_for_tests();
        register_runtime_handler("double_berry", |_| {
            Ok(vec![RewardMutation::MultiplyBerry(2.0).into()])
        });

        let report = dispatch_runtime_event(
            RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(100)).into(),
        );

        assert_eq!(
            report.mutations,
            [RuntimeMutation::Reward(RewardMutation::MultiplyBerry(2.0))]
        );
        assert_eq!(report.errors, []);
        reset_runtime_handlers_for_tests();
    }
}
