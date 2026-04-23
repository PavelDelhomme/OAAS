//! Instantané charge / CPU par cœur / RAM / GPU (NVIDIA + AMD ROCm) — Linux.

use std::collections::BTreeMap;

use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct CpuCoreSnap {
    pub index: usize,
    pub usage_percent: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuSnap {
    pub vendor: String,
    pub name: String,
    pub memory_used_mb: Option<u64>,
    pub memory_total_mb: Option<u64>,
    pub gpu_util_percent: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemSnapshot {
    pub platform: String,
    pub note: Option<String>,
    pub load_1: Option<f64>,
    pub load_5: Option<f64>,
    pub load_15: Option<f64>,
    pub mem_total_mb: Option<u64>,
    pub mem_available_mb: Option<u64>,
    pub oaas_rss_mib: Option<f64>,
    pub llama_rss_mib: Option<f64>,
    pub llama_pid: Option<u32>,
    pub cpu_cores: Vec<CpuCoreSnap>,
    pub gpus: Vec<GpuSnap>,
}

pub async fn collect_snapshot(llama_pid: Option<u32>) -> SystemSnapshot {
    #[cfg(target_os = "linux")]
    {
        collect_linux(llama_pid).await
    }
    #[cfg(not(target_os = "linux"))]
    {
        SystemSnapshot {
            platform: std::env::consts::OS.to_string(),
            note: Some("Métriques détaillées : build Linux uniquement.".into()),
            load_1: None,
            load_5: None,
            load_15: None,
            mem_total_mb: None,
            mem_available_mb: None,
            oaas_rss_mib: None,
            llama_rss_mib: None,
            llama_pid,
            cpu_cores: Vec::new(),
            gpus: Vec::new(),
        }
    }
}

#[cfg(target_os = "linux")]
async fn collect_linux(llama_pid: Option<u32>) -> SystemSnapshot {
    let load = read_loadavg();
    let (mem_total_mb, mem_available_mb) = read_meminfo();
    let oaas_rss_mib = proc_status_rss_kb(std::process::id()).map(kb_to_mib);
    let llama_rss_mib = llama_pid.and_then(|p| proc_status_rss_kb(p).map(kb_to_mib));
    let cpu_cores = sample_cpu_per_core().await;
    let mut gpus = query_nvidia_gpus().await;
    gpus.extend(query_amd_rocm_gpus().await);

    SystemSnapshot {
        platform: "linux".into(),
        note: None,
        load_1: load.0,
        load_5: load.1,
        load_15: load.2,
        mem_total_mb,
        mem_available_mb,
        oaas_rss_mib,
        llama_rss_mib,
        llama_pid,
        cpu_cores,
        gpus,
    }
}

#[cfg(target_os = "linux")]
fn kb_to_mib(kb: u64) -> f64 {
    kb as f64 / 1024.0
}

#[cfg(target_os = "linux")]
fn read_loadavg() -> (Option<f64>, Option<f64>, Option<f64>) {
    let Ok(s) = std::fs::read_to_string("/proc/loadavg") else {
        return (None, None, None);
    };
    let mut it = s.split_whitespace();
    let a = it.next().and_then(|x| x.parse().ok());
    let b = it.next().and_then(|x| x.parse().ok());
    let c = it.next().and_then(|x| x.parse().ok());
    (a, b, c)
}

#[cfg(target_os = "linux")]
fn read_meminfo() -> (Option<u64>, Option<u64>) {
    let Ok(s) = std::fs::read_to_string("/proc/meminfo") else {
        return (None, None);
    };
    let mut total = None;
    let mut avail = None;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            total = rest
                .trim()
                .strip_suffix(" kB")
                .and_then(|x| x.parse::<u64>().ok())
                .map(|kb| kb / 1024);
        } else if let Some(rest) = line.strip_prefix("MemAvailable:") {
            avail = rest
                .trim()
                .strip_suffix(" kB")
                .and_then(|x| x.parse::<u64>().ok())
                .map(|kb| kb / 1024);
        }
    }
    (total, avail)
}

#[cfg(target_os = "linux")]
fn proc_status_rss_kb(pid: u32) -> Option<u64> {
    let path = format!("/proc/{pid}/status");
    let s = std::fs::read_to_string(path).ok()?;
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            let num = rest.trim().strip_suffix(" kB")?;
            return num.parse().ok();
        }
    }
    None
}

/// Lit les lignes `cpuN` de `/proc/stat` : (idle+iowait, total jiffies).
#[cfg(target_os = "linux")]
fn read_proc_stat_per_cpu() -> Option<BTreeMap<usize, (u64, u64)>> {
    let s = std::fs::read_to_string("/proc/stat").ok()?;
    let mut out = BTreeMap::new();
    for line in s.lines() {
        let mut it = line.split_whitespace();
        let tag = it.next()?;
        if !tag.starts_with("cpu") || tag == "cpu" {
            continue;
        }
        let idx: usize = tag[3..].parse().ok()?;
        let nums: Vec<u64> = it.filter_map(|x| x.parse().ok()).collect();
        if nums.len() < 4 {
            continue;
        }
        let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
        let total: u64 = nums.iter().sum();
        out.insert(idx, (idle, total));
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(target_os = "linux")]
async fn sample_cpu_per_core() -> Vec<CpuCoreSnap> {
    let Some(a) = read_proc_stat_per_cpu() else {
        return Vec::new();
    };
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let Some(b) = read_proc_stat_per_cpu() else {
        return Vec::new();
    };
    let mut cores: Vec<CpuCoreSnap> = Vec::new();
    for (&idx, &(idle1, tot1)) in &a {
        let Some(&(idle2, tot2)) = b.get(&idx) else {
            continue;
        };
        let dt = tot2.saturating_sub(tot1);
        let usage = if dt > 0 {
            let di = idle2.saturating_sub(idle1);
            let busy = dt.saturating_sub(di) as f64 / dt as f64;
            Some((busy * 100.0).clamp(0.0, 100.0) as f32)
        } else {
            None
        };
        cores.push(CpuCoreSnap {
            index: idx,
            usage_percent: usage,
        });
    }
    cores.sort_by_key(|c| c.index);
    cores
}

#[cfg(target_os = "linux")]
async fn query_nvidia_gpus() -> Vec<GpuSnap> {
    let mut cmd = Command::new("nvidia-smi");
    cmd.args([
        "--query-gpu=name,memory.used,memory.total,utilization.gpu",
        "--format=csv,noheader,nounits",
    ]);
    let out = match tokio::time::timeout(std::time::Duration::from_secs(2), cmd.output()).await {
        Ok(Ok(o)) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&out);
    let mut gpus = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').map(str::trim).collect();
        if parts.len() < 4 {
            continue;
        }
        gpus.push(GpuSnap {
            vendor: "nvidia".into(),
            name: parts[0].to_string(),
            memory_used_mb: parts[1].parse().ok(),
            memory_total_mb: parts[2].parse().ok(),
            gpu_util_percent: parts[3].parse().ok(),
        });
    }
    gpus
}

/// `rocm-smi --json` (ROCm). Les clés varient selon la version ; on accepte nombre ou chaîne.
#[cfg(target_os = "linux")]
async fn query_amd_rocm_gpus() -> Vec<GpuSnap> {
    if which::which("rocm-smi").is_err() {
        return Vec::new();
    }
    let mut cmd = Command::new("rocm-smi");
    cmd.arg("--json");
    let out = match tokio::time::timeout(std::time::Duration::from_secs(3), cmd.output()).await {
        Ok(Ok(o)) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out) else {
        return Vec::new();
    };
    let Some(obj) = v.as_object() else {
        return Vec::new();
    };
    let mut gpus = Vec::new();
    for (_k, card) in obj {
        let Some(co) = card.as_object() else {
            continue;
        };
        let name = co
            .get("Card Series")
            .or_else(|| co.get("Card Model"))
            .or_else(|| co.get("Card series"))
            .and_then(json_as_string)
            .unwrap_or_else(|| "AMD GPU".into());
        let gpu_util_percent = co
            .get("GPU use (%)")
            .or_else(|| co.get("GPU Utilization (%)"))
            .and_then(json_as_u32);
        let vram_total_mb = co
            .get("VRAM Total Memory (B)")
            .and_then(json_as_u64)
            .map(|b| b / (1024 * 1024));
        let vram_used_mb = co
            .get("VRAM Total Used Memory (B)")
            .or_else(|| co.get("VRAM Used Memory (B)"))
            .and_then(json_as_u64)
            .map(|b| b / (1024 * 1024));
        gpus.push(GpuSnap {
            vendor: "amd".into(),
            name,
            memory_used_mb: vram_used_mb,
            memory_total_mb: vram_total_mb,
            gpu_util_percent,
        });
    }
    gpus
}

#[cfg(target_os = "linux")]
fn json_as_string(v: &serde_json::Value) -> Option<String> {
    v.as_str()
        .map(String::from)
        .or_else(|| v.as_i64().map(|n| n.to_string()))
}

#[cfg(target_os = "linux")]
fn json_as_u64(v: &serde_json::Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_i64().map(|n| n as u64))
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
}

#[cfg(target_os = "linux")]
fn json_as_u32(v: &serde_json::Value) -> Option<u32> {
    v.as_u64()
        .map(|n| n as u32)
        .or_else(|| v.as_i64().map(|n| n as u32))
        .or_else(|| {
            v.as_str()
                .and_then(|s| s.trim().replace('%', "").parse().ok())
        })
}
