use anyhow::{anyhow, Context, Result};
use log::{debug, warn};
use nvml_wrapper::{error::NvmlError, Device, Nvml};

use std::sync::LazyLock;

pub static NVML: LazyLock<Result<Nvml, NvmlError>> = LazyLock::new(|| {
    let nvml = Nvml::init();

    if let Err(error) = nvml.as_ref() {
        warn!("Connection to NVML failed, reason: {error}");
    } else {
        debug!("Successfully connected to NVML");
    }
    nvml
});

pub struct NvidiaGpu<'a> {
    // Index returned by NVML
    pub index: u32,
    pub name: String,
    pub uuid: String,
    vram_total: u64,
    device: Option<Device<'a>>,
}

impl NvidiaGpu<'_> {
    pub fn new(index: u32, name: String, uuid: String) -> Self {
        let mut device = None;
        let mut vram = 0;

        if let Ok(nvml) = NVML.as_ref() {
            if let Ok(dev) = nvml.device_by_index(index) {
                if let Ok(mem) = dev.memory_info() {
                    vram = mem.total;
                    device = Some(dev);
                }
            }
        }

        NvidiaGpu {
            index,
            name,
            uuid,
            vram_total: vram,
            device,
        }
    }
}

impl super::GpuIf for NvidiaGpu<'_> {
    fn restart(&mut self) {
        if self.device.is_none() {
            if let Ok(nvml) = NVML.as_ref() {
                if let Ok(dev) = nvml.device_by_index(self.index) {
                    self.device = Some(dev);
                }
            }
        }
    }

    fn stop(&mut self) {
        if self.device.is_some() {
            // Drop device
            self.device = None;
        }
    }

    fn is_active(&self) -> bool {
        self.device.is_some()
    }

    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn id(&self) -> String {
        self.uuid.to_owned()
    }

    fn usage(&self) -> Result<u32> {
        self.with_device(|device_ref| {
            let rates = device_ref.utilization_rates()?;
            Ok(rates.gpu)
        })
    }

    fn vram_total(&self) -> u64 {
        self.vram_total
    }

    fn vram_used(&self) -> Result<u64> {
        self.with_device(|device_ref| {
            let mem = device_ref.memory_info()?;
            Ok(mem.used)
        })
    }
}

impl NvidiaGpu<'_> {
    pub fn uuid(idx: u32) -> Result<String> {
        NVML.as_ref()
            .context("unable to establish NVML connection")
            .and_then(|nvml| {
                let dev = nvml.device_by_index(idx)?;
                dev.uuid().context("Unable to retrieve uuid")
            })
    }

    pub fn name(idx: u32) -> Result<String> {
        NVML.as_ref()
            .context("unable to establish NVML connection")
            .and_then(|nvml| {
                let dev = nvml.device_by_index(idx)?;
                dev.name().context("Unable to retrieve name")
            })
    }

    pub fn gpus() -> Result<u32> {
        NVML.as_ref()
            .context("unable to establish NVML connection")
            .and_then(|nvml| nvml.device_count().context("failed to get GPU count"))
    }

    fn with_device<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Device) -> Result<T>,
    {
        match self.device.as_ref() {
            Some(device_ref) => f(device_ref),
            None => Err(anyhow!("nvml device not loaded")),
        }
    }
}
