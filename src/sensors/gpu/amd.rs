use anyhow::Result;

use crate::sensors::gpus::Gpu;
//use log::{debug, warn};

pub struct AmdGpu {
    pub name: String,
    pub id: String,
}

impl AmdGpu {
    pub fn new(name: String, id: String) -> Self {
        AmdGpu { name, id }
    }
}

impl super::GpuIf for AmdGpu {
    fn restart(&mut self) {
        todo!();
    }

    fn stop(&mut self) {
        todo!();
    }

    fn is_active(&self) -> bool {
        todo!();
    }

    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn id(&self) -> String {
        self.id.to_owned()
    }

    fn usage(&self) -> Result<u32> {
        todo!();
    }

    fn vram_total(&self) -> u64 {
        todo!();
    }

    fn vram_used(&self) -> Result<u64> {
        todo!();
    }
}

impl AmdGpu {
    pub fn get_gpus() -> Vec<Gpu> {
        Vec::new()
    }
}
