use anyhow::Result;

use super::gpus::Gpu;

pub mod nvidia;
pub mod amd;
pub mod intel;

pub trait GpuIf {
    fn name(&self) -> String;
    fn id(&self) -> String;
    fn usage(&self) -> Result<u32>;
    fn vram_total(&self) -> u64;
    fn vram_used(&self) -> Result<u64>;

    // Stop polling, to allow it to sleep
    fn stop(&mut self);
    // Resume active polling
    fn restart(&mut self);
    // Stopped or active for polling?
    fn is_active(&self) -> bool;
}
