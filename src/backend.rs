#[cfg(feature = "desktop")]
pub mod desktop_backend;

#[cfg(feature = "remarkable")]
pub mod remarkable_backend;

pub trait Backend {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
