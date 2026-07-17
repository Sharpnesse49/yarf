use std::fs;
use std::io::{self, Write};
use std::process::Command;

fn get_os() -> String {
    fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|content| {
            content.lines()
                .find(|line| line.starts_with("PRETTY_NAME="))
                .map(|line| {
                    line.strip_prefix("PRETTY_NAME=")
                        .unwrap_or(line)
                        .trim_matches('"')
                        .to_string()
                })
        })
        .unwrap_or_else(|| "Linux".to_string())
}

fn get_kernel() -> String {
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "Inconnu".to_string())
}

fn get_cpu() -> String {
    fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content.lines()
                .find(|line| line.starts_with("model name"))
                .map(|line| {
                    line.split(':')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .to_string()
                })
        })
        .unwrap_or_else(|| "Inconnu".to_string())
}

fn get_gpu() -> String {

    Command::new("lspci")
        .output()
        .ok()
        .and_then(|out| {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.lines()
                .find(|line| {
                    line.contains("VGA compatible controller") 
                        || line.contains("3D controller") 
                        || line.contains("Display controller")
                })
                .map(|line| {
                    line.split(':')
                        .last()
                        .unwrap_or("")
                        .trim()
                        .to_string()
                })
        })
        .unwrap_or_else(|| "Inconnu".to_string())
}

fn get_ram() -> String {
    let content = fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mut mem_total = None;
    let mut mem_available = None;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            mem_total = line.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok());
        } else if line.starts_with("MemAvailable:") {
            mem_available = line.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok());
        }
        
        if mem_total.is_some() && mem_available.is_some() {
            break;
        }
    }

    let total = mem_total.unwrap_or(0);
    let available = mem_available.unwrap_or(0);
    let used = total.saturating_sub(available);

    format!("{}Mo / {}Mo", used / 1024, total / 1024)
}

fn main() {
    let blue = "\x1b[34m";
    let reset = "\x1b[0m";

    let os = get_os();
    let krnl = get_kernel();
    let cpu = get_cpu();
    let gpu = get_gpu();
    let ram = get_ram();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let _ = writeln!(handle, "{}os   :{} {}", blue, reset, os);
    let _ = writeln!(handle, "{}krnl :{} {}", blue, reset, krnl);
    let _ = writeln!(handle, "{}cpu  :{} {}", blue, reset, cpu);
    let _ = writeln!(handle, "{}gpu  :{} {}", blue, reset, gpu);
    let _ = writeln!(handle, "{}ram  :{} {}", blue, reset, ram);
}
