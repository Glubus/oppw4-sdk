use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

type Subscriber<T> = Box<dyn Fn(T) + Send + Sync>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SignalId(&'static str);

impl SignalId {
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

pub struct Signal<T> {
    id: SignalId,
    subscribers: Mutex<Vec<Subscriber<T>>>,
}

impl<T: Clone> Signal<T> {
    pub fn new(id: SignalId) -> Self {
        Self {
            id,
            subscribers: Mutex::new(Vec::new()),
        }
    }

    pub fn id(&self) -> SignalId {
        self.id
    }

    pub fn subscribe(&self, subscriber: impl Fn(T) + Send + Sync + 'static) {
        if let Ok(mut subscribers) = self.subscribers.lock() {
            subscribers.push(Box::new(subscriber));
        }
    }

    pub fn emit(&self, value: T) {
        let Ok(subscribers) = self.subscribers.lock() else {
            return;
        };
        for subscriber in subscribers.iter() {
            subscriber(value.clone());
        }
    }
}

#[derive(Default)]
pub struct SignalBus {
    names: Mutex<HashMap<SignalId, usize>>,
}

impl SignalBus {
    pub fn register<T: Clone>(&self, signal: &Arc<Signal<T>>) {
        if let Ok(mut names) = self.names.lock() {
            names.insert(signal.id(), Arc::as_ptr(signal) as usize);
        }
    }
}

pub struct SignalHook<T> {
    signal: Arc<Signal<T>>,
}

impl<T: Clone> SignalHook<T> {
    pub fn new(signal: Arc<Signal<T>>) -> Self {
        Self { signal }
    }

    pub fn emit(&self, value: T) {
        self.signal.emit(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn emits_to_subscribers() {
        let signal = Signal::new(SignalId::new("test"));
        let seen = Arc::new(AtomicUsize::new(0));
        let seen_for_callback = Arc::clone(&seen);
        signal.subscribe(move |value| {
            seen_for_callback.store(value, Ordering::Relaxed);
        });

        signal.emit(42usize);

        assert_eq!(seen.load(Ordering::Relaxed), 42);
    }
}
