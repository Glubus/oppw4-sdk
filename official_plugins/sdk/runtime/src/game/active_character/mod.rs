mod abi;
mod hook;
mod probe;

pub(crate) use abi::read_active_character;

pub(crate) fn start_probe() {
    probe::start();
}
