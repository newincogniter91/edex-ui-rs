use std::sync::{Arc, Mutex};

pub type Hw = Arc<Mutex<Option<[String; 3]>>>;

/// Legge produttore / modello / telaio in un thread (PowerShell su Windows, DMI su Linux).
pub fn spawn() -> Hw {
    let h: Hw = Arc::new(Mutex::new(None));
    let h2 = h.clone();
    std::thread::spawn(move || {
        let v = query();
        if let Ok(mut g) = h2.lock() {
            *g = Some(v);
        }
    });
    h
}

fn chassis_name(n: u32) -> &'static str {
    match n {
        1 => "Other",
        3 => "Desktop",
        4 => "Low Profile Desktop",
        5 => "Pizza Box",
        6 => "Mini Tower",
        7 => "Tower",
        8 => "Portable",
        9 => "Laptop",
        10 => "Notebook",
        11 => "Hand Held",
        12 => "Docking Station",
        13 => "All in One",
        14 => "Sub Notebook",
        15 => "Space-saving",
        16 => "Lunch Box",
        17 => "Main System Chassis",
        23 => "Rack Mount",
        24 => "Sealed-case PC",
        30 => "Tablet",
        31 => "Convertible",
        32 => "Detachable",
        _ => "Unknown",
    }
}

#[cfg(windows)]
fn query() -> [String; 3] {
    use std::os::windows::process::CommandExt;
    let script = "$c=Get-CimInstance Win32_ComputerSystem; $e=Get-CimInstance Win32_SystemEnclosure; \
                  Write-Output ($c.Manufacturer+'|'+$c.Model+'|'+$e.ChassisTypes[0])";
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output();
    if let Ok(o) = out {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        let p: Vec<&str> = s.split('|').collect();
        if p.len() == 3 {
            let ch = p[2].trim().parse::<u32>().map(chassis_name).unwrap_or("Unknown");
            return [p[0].trim().to_string(), p[1].trim().to_string(), ch.to_string()];
        }
    }
    ["--".into(), "--".into(), "--".into()]
}

#[cfg(not(windows))]
fn query() -> [String; 3] {
    let rd = |f: &str| std::fs::read_to_string(format!("/sys/class/dmi/id/{f}")).map(|s| s.trim().to_string()).unwrap_or_else(|_| "--".into());
    let ch = rd("chassis_type").parse::<u32>().map(chassis_name).unwrap_or("--");
    [rd("sys_vendor"), rd("product_name"), ch.to_string()]
}
