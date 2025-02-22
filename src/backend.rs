pub trait Backend {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}
