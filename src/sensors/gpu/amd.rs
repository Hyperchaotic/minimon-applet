use anyhow::{Result, anyhow};
use hex;
use log::{debug, info};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::sensors::gpus::Gpu;
//use log::{debug, warn};

pub struct AmdGpu {
    name: String,
    id: String,
    usage_path: String,
    vram_used_path: String,
    power_status_path: String,
    vram_total: u64,
    paused: bool,
}

impl AmdGpu {
    pub fn new(name: &str, card: &str, id: &str, vram_total: u64) -> Self {
        let base = format!("/sys/class/drm/{}/device", card);
        Self {
            name: name.to_string(),
            id: id.to_string(),
            usage_path: format!("{}/gpu_busy_percent", base),
            vram_used_path: format!("{}/mem_info_vram_used", base),
            power_status_path: format!("{}/power/runtime_status", base),
            vram_total,
            paused: false,
        }
    }

    fn powered_on(&self) -> bool {
        Self::read_file_to_string(&self.power_status_path).map_or(true, |s| s != "suspended")
    }

    fn parse_u32_file(path: &str) -> Option<u32> {
        Self::read_file_to_string(path).ok()?.parse().ok()
    }

    fn parse_u64_file(path: &str) -> Option<u64> {
        Self::read_file_to_string(path).ok()?.parse().ok()
    }

    fn read_file_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
        fs::read_to_string(path).map(|s| s.trim().to_string())
    }

    fn get_amd_cards() -> Vec<String> {
        debug!("AmdGpu::get_amd_cards().");
        let mut cards = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/class/drm/") {
            for entry in entries.flatten() {
                let path = entry.path();
                debug!("                    entry {:?}", path);
                if path.join("device/vendor").exists() {
                    if let Ok(vendor_id) = Self::read_file_to_string(path.join("device/vendor")) {
                        if vendor_id == "0x1002" {
                            debug!("                    AMD vendor ID");
                            if let Some(card) = path.file_name().and_then(|n| n.to_str()) {
                                if card.contains("card") {
                                    debug!("                    phyical Card.");
                                    cards.push(card.to_string());
                                } else {
                                    debug!("                    virtual card");
                                }
                            }
                        } else {
                            debug!("                    Not AMD");
                        }
                    }
                }
            }
        }
        cards
    }

    fn get_vram_total(card: &str) -> Option<u64> {
        let path = format!("/sys/class/drm/{}/device/mem_info_vram_total", card);
        Self::parse_u64_file(&path)
    }

    fn get_pci_slot(card: &str) -> Option<String> {
        let path = format!("/sys/class/drm/{}/device/uevent", card);
        Self::read_file_to_string(path)
            .ok()?
            .lines()
            .find_map(|line| line.strip_prefix("PCI_SLOT_NAME=").map(|s| s.to_string()))
    }

    fn get_lspci_gpu_names() -> Vec<(String, String)> {
        let output = Command::new("lspci").arg("-nn").output();
        let Ok(output) = output else {
            return Vec::new();
        };
        let Ok(stdout) = String::from_utf8(output.stdout) else {
            return Vec::new();
        };

        debug!("get_lspci_gpu_names(): {}", stdout);
        stdout
            .lines()
            .filter(|line| {
                line.contains("AMD")
                    && (line.contains("VGA") || line.contains("Display") || line.contains("3D"))
            })
            .filter_map(|line| {
                line.split_once(' ')
                    .map(|(slot, name)| (slot.to_string(), name.trim().to_string()))
            })
            .collect()
    }

    fn get_gpu_name(card: &str, lspci_map: &[(String, String)]) -> String {
        let pci_slot = Self::get_pci_slot(card);
        debug!("AmdGpu::get_gpu_name({}), slot: {:?}", card, pci_slot);
        debug!("lspci_map: {:?}", lspci_map);
        pci_slot
            .and_then(|slot| {
                let short_slot = slot.rsplit_once(':').map(|(_, s)| s).unwrap_or(&slot);
                debug!("Short slot: {}", short_slot);
                lspci_map
                    .iter()
                    .find(|(s, _)| s.ends_with(short_slot))
                    .map(|(_, name)| name.clone())
            })
            .or_else(|| {
                Self::read_file_to_string(format!("/sys/class/drm/{}/device/subsystem_device", card))
                    .ok()
                    .and_then(|dev_id| {
                        AMD_GPU_DEVICE_IDS
                            .get(dev_id.to_lowercase().as_str())
                            .map(|s| s.to_string())
                    })
            })
            .unwrap_or_else(|| "Unknown AMD GPU".to_string())
    }

    fn generate_gpu_id(card: &str) -> Option<String> {
        let device_path = PathBuf::from(format!("/sys/class/drm/{}/device", card));
        let pci_address = device_path.canonicalize().ok()?;
        let subsystem_vendor =
            Self::read_file_to_string(device_path.join("subsystem_vendor")).ok()?;
        let subsystem_device =
            Self::read_file_to_string(device_path.join("subsystem_device")).ok()?;

        let mut hasher = Sha256::new();
        hasher.update(pci_address.to_string_lossy().as_bytes());
        hasher.update(subsystem_vendor.as_bytes());
        hasher.update(subsystem_device.as_bytes());

        Some(hex::encode(hasher.finalize()))
    }

    pub fn get_gpus() -> Vec<Gpu> {
        debug!("AmdGpu::get_gpus().");

        let mut gpus = Vec::new();

        let lspci_map = AmdGpu::get_lspci_gpu_names();

        let cards = AmdGpu::get_amd_cards();

        for card in cards {
            debug!("                    Found card {}", card);
            if let Some(vram_total) = AmdGpu::get_vram_total(&card) {
                debug!("                    total vram {}", vram_total);
                if let Some(id) = AmdGpu::generate_gpu_id(&card) {
                    debug!("                    id {}", id);
                    let name = AmdGpu::get_gpu_name(&card, &lspci_map);
                    debug!("                    name {}", name);
                    gpus.push(Gpu::new(Box::new(AmdGpu::new(
                        &name, &card, &id, vram_total,
                    ))));
                }
            }
        }
        gpus
    }
}

impl super::GpuIf for AmdGpu {
    fn restart(&mut self) {
        debug!("AmdGpu::restart({}).", self.name);
        self.paused = false;
    }

    fn stop(&mut self) {
        debug!("AmdGpu::stop({}).", self.name);
        self.paused = true;
    }

    fn is_active(&self) -> bool {
        !self.paused
    }

    fn name(&self) -> String {
        self.name.to_owned()
    }

    fn id(&self) -> String {
        self.id.to_owned()
    }

    fn vram_total(&self) -> u64 {
        debug!("AmdGpu::vram_total({}) - {}.", self.name, self.vram_total);
        self.vram_total
    }

    fn usage(&self) -> Result<u32> {
        if !self.is_active() {
            debug!("AmdGpu::usage({}) - AMD device paused.", self.name);
            return Err(anyhow!("AMD device paused"));
        }
        if !self.powered_on() {
            debug!(
                "AmdGpu::usage({}) - AMD device sleeping, returning 0.",
                self.name
            );
            return Ok(0);
        }
        let usage = Ok(Self::parse_u32_file(&self.usage_path).unwrap_or(0));
//        debug!("AmdGpu::usage({}) - {:?} %.", self.name, usage);
        usage
    }

    fn vram_used(&self) -> Result<u64> {
        if !self.is_active() {
            debug!("AmdGpu::vram_used({}) - AMD device paused.", self.name);
            return Err(anyhow!("AMD device paused"));
        }
        if !self.powered_on() {
//            debug!(
  //              "AmdGpu::vram_used({}) - AMD device sleeping, returning 0.",
    //            self.name
      //      );
            return Ok(0);
        }
        let vram = Ok(Self::parse_u64_file(&self.vram_used_path).unwrap_or(0));
//        debug!("AmdGpu::vram_used({}) - {:?} bytes.", self.name, vram);
        vram
    }
}

impl std::fmt::Debug for AmdGpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AmdGpu {{ name: {}, id: {}, paused: {} }}",
            self.name, self.id, self.paused
        )
    }
}

use once_cell::sync::Lazy;

/// A hashmap containing AMD graphics card subsystem device IDs and their names
/// Keys are the values found in /sys/class/drm/card?/device/subsystem_device
pub static AMD_GPU_DEVICE_IDS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    // Radeon RX 7000 Series
    m.insert("0x744C", "AMD Radeon RX 7700S");
    m.insert("0x73FF", "AMD Radeon RX 7900 XTX");
    m.insert("0x73DF", "AMD Radeon RX 7900 XT");
    m.insert("0x7470", "AMD Radeon RX 7800 XT");
    m.insert("0x7460", "AMD Radeon RX 7700 XT");
    m.insert("0x7420", "AMD Radeon RX 7600");
    m.insert("0x7422", "AMD Radeon RX 7600 XT");
    
    // Radeon RX 6000 Series
    m.insert("0x73BF", "AMD Radeon RX 6950 XT");
    m.insert("0x73A5", "AMD Radeon RX 6900 XT");
    m.insert("0x73A3", "AMD Radeon RX 6800 XT");
    m.insert("0x73AB", "AMD Radeon RX 6800");
    m.insert("0x73DF", "AMD Radeon RX 6750 XT");
    m.insert("0x73D5", "AMD Radeon RX 6700 XT");
    m.insert("0x73FF", "AMD Radeon RX 6700");
    m.insert("0x73EF", "AMD Radeon RX 6650 XT");
    m.insert("0x73E8", "AMD Radeon RX 6600 XT");
    m.insert("0x73E3", "AMD Radeon RX 6600");
    m.insert("0x7422", "AMD Radeon RX 6500 XT");
    m.insert("0x7424", "AMD Radeon RX 6400");
    
    // Radeon RX 5000 Series
    m.insert("0x731F", "AMD Radeon RX 5700 XT");
    m.insert("0x7340", "AMD Radeon RX 5700");
    m.insert("0x7341", "AMD Radeon RX 5600 XT");
    m.insert("0x7347", "AMD Radeon RX 5500 XT");
    
    // Radeon RX Vega Series
    m.insert("0x687F", "AMD Radeon VII");
    m.insert("0x6863", "AMD Radeon RX Vega 64");
    m.insert("0x6867", "AMD Radeon RX Vega 56");
    
    // Radeon RX 500 Series
    m.insert("0x67DF", "AMD Radeon RX 590");
    m.insert("0x67FF", "AMD Radeon RX 580");
    m.insert("0x67EF", "AMD Radeon RX 570");
    m.insert("0x67E0", "AMD Radeon RX 560");
    m.insert("0x699F", "AMD Radeon RX 550");
    
    // APUs - Integrated Graphics
    m.insert("0x1681", "AMD Radeon 780M iGPU");
    m.insert("0x15E7", "AMD Radeon 760M iGPU");
    m.insert("0x15D8", "AMD Radeon 680M iGPU");
    m.insert("0x1638", "AMD Radeon 660M iGPU");
    m.insert("0x164C", "AMD Radeon 610M iGPU");
    m.insert("0x15DD", "AMD Radeon Vega 8 iGPU");
    m.insert("0x15D8", "AMD Radeon Vega 7 iGPU");
    
    // Radeon Pro Series
    m.insert("0x73A2", "AMD Radeon Pro W6800");
    m.insert("0x73A3", "AMD Radeon Pro W6600");
    m.insert("0x6867", "AMD Radeon Pro VII");
    m.insert("0x66AF", "AMD Radeon Pro WX 9100");
    m.insert("0x67C4", "AMD Radeon Pro WX 7100");
    
    m
});