use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::process::Command;

fn read_first_line(path: &str) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    Some(line.trim().to_string())
}

fn get_os() -> String {
    File::open("/etc/os-release")
        .map(BufReader::new)
        .ok()
        .and_then(|reader| {
            for line in reader.lines().flatten() {
                if let Some(stripped) = line.strip_prefix("PRETTY_NAME=") {
                    return Some(stripped.trim_matches('"').to_string());
                }
            }
            None
        })
        .unwrap_or_else(|| "Linux".to_string())
}

fn get_kernel() -> String {
    read_first_line("/proc/sys/kernel/osrelease").unwrap_or_else(|| "Inconnu".to_string())
}

fn get_cpu() -> String {
    File::open("/proc/cpuinfo")
        .map(BufReader::new)
        .ok()
        .and_then(|reader| {
            for line in reader.lines().flatten() {
                if line.starts_with("model name") {
                    if let Some(pos) = line.find(':') {
                        return Some(line[pos + 1..].trim().to_string());
                    }
                }
            }
            None
        })
        .unwrap_or_else(|| "Inconnu".to_string())
}

fn get_gpu_lspci() -> String {
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

fn get_gpu() -> String {
    let mut vendor_id = String::new();
    let mut device_id = String::new();
    let mut found = false;

    if let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(class_content) = fs::read_to_string(path.join("class")) {
                let class_trimmed = class_content.trim();
                if class_trimmed.starts_with("0x03") || class_trimmed.starts_with("03") {
                    if let (Ok(v), Ok(d)) = (
                        fs::read_to_string(path.join("vendor")),
                        fs::read_to_string(path.join("device")),
                    ) {
                        vendor_id = v.trim().trim_start_matches("0x").to_lowercase();
                        device_id = d.trim().trim_start_matches("0x").to_lowercase();
                        found = true;
                        break;
                    }
                }
            }
        }
    }

    if !found {
        return get_gpu_lspci();
    }

    // Try looking up in pci.ids first
    for database_path in &["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"] {
        if let Ok(file) = File::open(database_path) {
            let reader = BufReader::new(file);
            let mut vendor_name = None;
            let mut device_name = None;
            let mut in_vendor = false;

            for line in reader.lines().flatten() {
                if line.starts_with('#') || line.trim().is_empty() {
                    continue;
                }

                if line.starts_with('\t') {
                    if in_vendor {
                        let trimmed = line.trim_start_matches('\t');
                        if !trimmed.starts_with('\t') {
                            if let Some(pos) = trimmed.find(' ') {
                                let (d_id, d_name) = trimmed.split_at(pos);
                                if d_id.trim() == device_id {
                                    device_name = Some(d_name.trim().to_string());
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    if in_vendor {
                        break; // Left our vendor section, stop scanning
                    }
                    if let Some(pos) = line.find(' ') {
                        let (v_id, v_name) = line.split_at(pos);
                        if v_id.trim() == vendor_id {
                            vendor_name = Some(v_name.trim().to_string());
                            in_vendor = true;
                        }
                    }
                }
            }

            if let Some(v_name) = vendor_name {
                if let Some(d_name) = device_name {
                    return format!("{} {}", v_name, d_name);
                }
                return v_name;
            }
        }
    }

    get_gpu_lspci()
}

fn get_ram() -> String {
    let mut mem_total = None;
    let mut mem_available = None;

    if let Ok(file) = File::open("/proc/meminfo") {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            if line.starts_with("MemTotal:") {
                mem_total = line.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok());
            } else if line.starts_with("MemAvailable:") {
                mem_available = line.split_whitespace().nth(1).and_then(|s| s.parse::<u64>().ok());
            }
            if mem_total.is_some() && mem_available.is_some() {
                break;
            }
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

    let tux_lines = [
        "    .--.",
        "   |o_o |",
        "   |:_/ |       ",
        "  //   \\ \\      ",
        " (|     | )     ",
        "/'\\_   _/`\\     ",
        "\\___)=(___/     ",
    ];

    let _ = writeln!(handle, "{}{}{}", blue, tux_lines[0], reset);
    let _ = writeln!(handle, "{}{}{}", blue, tux_lines[1], reset);
    let _ = writeln!(handle, "{}{}os   :{} {}", blue, tux_lines[2], reset, os);
    let _ = writeln!(handle, "{}{}krnl :{} {}", blue, tux_lines[3], reset, krnl);
    let _ = writeln!(handle, "{}{}cpu  :{} {}", blue, tux_lines[4], reset, cpu);
    let _ = writeln!(handle, "{}{}gpu  :{} {}", blue, tux_lines[5], reset, gpu);
    let _ = writeln!(handle, "{}{}ram  :{} {}", blue, tux_lines[6], reset, ram);
}
