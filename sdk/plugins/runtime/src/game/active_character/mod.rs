mod abi;
mod hook;
mod probe;
mod state;

pub(crate) use abi::read_active_character;

pub(crate) fn start_probe() {
    probe::start();
}
