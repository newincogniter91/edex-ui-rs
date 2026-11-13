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

/// Stato della batteria (None se il PC non ne ha una).
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
    pub ac: bool,
}

#[cfg(windows)]
pub fn battery() -> Option<Battery> {
    #[repr(C)]
    struct Sps {
        ac: u8,
        flag: u8,
        pct: u8,
        sys: u8,
        life: u32,
        full: u32,
    }
    extern "system" {
        fn GetSystemPowerStatus(p: *mut Sps) -> i32;
    }
    let mut s = Sps { ac: 0, flag: 0, pct: 0, sys: 0, life: 0, full: 0 };
    if unsafe { GetSystemPowerStatus(&mut s) } == 0 || s.flag == 128 || s.flag == 255 {
        return None;
    }
    Some(Battery { percent: if s.pct == 255 { 0 } else { s.pct }, charging: s.flag & 8 != 0, ac: s.ac == 1 })
}

#[cfg(not(windows))]
pub fn battery() -> Option<Battery> {
    let rd = std::fs::read_dir("/sys/class/power_supply").ok()?;
    let mut bat: Option<Battery> = None;
    let mut ac = false;
    for e in rd.flatten() {
        let p = e.path();
        let ty = std::fs::read_to_string(p.join("type")).unwrap_or_default();
        match ty.trim() {
            "Battery" => {
                let pct = std::fs::read_to_string(p.join("capacity")).ok().and_then(|s| s.trim().parse::<u8>().ok()).unwrap_or(0);
                let st = std::fs::read_to_string(p.join("status")).unwrap_or_default();
                bat = Some(Battery { percent: pct, charging: st.trim() == "Charging", ac: false });
            }
            "Mains" => ac = std::fs::read_to_string(p.join("online")).map(|s| s.trim() == "1").unwrap_or(false),
            _ => {}
        }
    }
    bat.map(|mut b| {
        b.ac = ac;
        b
    })
}
