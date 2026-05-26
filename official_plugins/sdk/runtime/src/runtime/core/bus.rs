use std::fmt;

use super::events::{RuntimeEvent, RuntimeMutation};

type RuntimeHandler =
    dyn Fn(&RuntimeEvent) -> Result<Vec<RuntimeMutation>, RuntimeHandlerError> + Send + Sync;

/// In-memory runtime event bus used by Rust-side frontends and future script adapters.
#[derive(Default)]
pub(crate) struct RuntimeEventBus {
    handlers: Vec<RegisteredHandler>,
}

impl RuntimeEventBus {
    pub(crate) const fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub(crate) fn register_handler(
        &mut self,
        id: impl Into<String>,
        handler: impl Fn(&RuntimeEvent) -> Result<Vec<RuntimeMutation>, RuntimeHandlerError>
            + Send
            + Sync
            + 'static,
    ) {
        let id = id.into();
        self.handlers.retain(|registered| registered.id != id);
        self.handlers.push(RegisteredHandler {
            id,
            handler: Box::new(handler),
        });
    }

    pub(crate) fn dispatch(&self, event: &RuntimeEvent) -> RuntimeDispatchReport {
        let mut report = RuntimeDispatchReport::default();

        for registered in &self.handlers {
            match (registered.handler)(event) {
                Ok(mut mutations) => report.mutations.append(&mut mutations),
                Err(error) => report.errors.push(RuntimeDispatchFailure {
                    handler_id: registered.id.clone(),
                    error,
                }),
            }
        }

        report
    }
}

struct RegisteredHandler {
    id: String,
    handler: Box<RuntimeHandler>,
}

/// Result of dispatching one event through all registered handlers.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct RuntimeDispatchReport {
    pub(crate) mutations: Vec<RuntimeMutation>,
    pub(crate) errors: Vec<RuntimeDispatchFailure>,
}

/// Handler failure tagged with the handler that produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeDispatchFailure {
    pub(crate) handler_id: String,
    pub(crate) error: RuntimeHandlerError,
}

/// Error returned by a single runtime handler.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeHandlerError {
    message: String,
}

impl RuntimeHandlerError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RuntimeHandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RuntimeHandlerError {}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use super::*;
    use crate::runtime::core::{
        rank::RankValue,
        rewards::{RewardCommitEvent, RewardMutation, RewardState},
    };

    #[test]
    fn handler_receives_reward_commit_event() {
        let received = Arc::new(AtomicBool::new(false));
        let received_in_handler = Arc::clone(&received);
        let mut bus = RuntimeEventBus::new();
        bus.register_handler("observer", move |event| {
            if matches!(event, RuntimeEvent::RewardCommit(_)) {
                received_in_handler.store(true, Ordering::SeqCst);
            }
            Ok(Vec::new())
        });

        let report = bus.dispatch(&reward_commit_event());

        assert!(received.load(Ordering::SeqCst));
        assert_eq!(report.mutations, []);
        assert_eq!(report.errors, []);
    }

    #[test]
    fn two_handlers_can_each_produce_mutations() {
        let mut bus = RuntimeEventBus::new();
        bus.register_handler("double_berry", |_| {
            Ok(vec![RewardMutation::MultiplyBerry(2.0).into()])
        });
        bus.register_handler("add_berry", |_| {
            Ok(vec![RewardMutation::AddBerry(50).into()])
        });

        let report = bus.dispatch(&reward_commit_event());

        assert_eq!(
            report.mutations,
            [
                RuntimeMutation::Reward(RewardMutation::MultiplyBerry(2.0)),
                RuntimeMutation::Reward(RewardMutation::AddBerry(50)),
            ]
        );
        assert_eq!(report.errors, []);
    }

    #[test]
    fn registering_same_handler_id_replaces_previous_handler() {
        let mut bus = RuntimeEventBus::new();
        bus.register_handler("rank", |_| Ok(vec![RewardMutation::SetBerry(1).into()]));
        bus.register_handler("rank", |_| Ok(vec![RewardMutation::SetBerry(2).into()]));

        let report = bus.dispatch(&reward_commit_event());

        assert_eq!(
            report.mutations,
            [RuntimeMutation::Reward(RewardMutation::SetBerry(2))]
        );
        assert_eq!(report.errors, []);
    }

    #[test]
    fn handler_errors_are_collected_with_handler_id() {
        let mut bus = RuntimeEventBus::new();
        bus.register_handler("broken", |_| {
            Err(RuntimeHandlerError::new("handler failed"))
        });
        bus.register_handler("still_runs", |_| {
            Ok(vec![RewardMutation::SetBerry(100).into()])
        });

        let report = bus.dispatch(&reward_commit_event());

        assert_eq!(
            report.mutations,
            [RuntimeMutation::Reward(RewardMutation::SetBerry(100))]
        );
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].handler_id, "broken");
        assert_eq!(report.errors[0].error.message(), "handler failed");
    }

    #[test]
    fn dispatch_without_handlers_returns_empty_report() {
        let bus = RuntimeEventBus::new();

        let report = bus.dispatch(&reward_commit_event());

        assert_eq!(report, RuntimeDispatchReport::default());
    }

    fn reward_commit_event() -> RuntimeEvent {
        RewardCommitEvent::new(RankValue::SPlus, RewardState::new().with_berry(100)).into()
    }
}
