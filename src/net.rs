use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub struct NetInfo {
    pub ip: String,
    pub ping_ms: Option<f32>,
    pub online: bool,
    pub geo: Option<(f64, f64)>,
    pub addr: String,
    pub geo_on: bool,
}

pub type Net = Arc<Mutex<NetInfo>>;

/// Thread di rete: IP locale e ping ogni 2 s; la posizione si ottiene una volta (se abilitata)
/// con una richiesta HTTP a ip-api.com.
pub fn spawn(addr: String, geo_on: bool) -> Net {
    let n: Net = Arc::new(Mutex::new(NetInfo { addr, geo_on, ip: "--".into(), ..Default::default() }));
    let n2 = n.clone();
    std::thread::spawn(move || {
        let mut last_geo = Instant::now() - Duration::from_secs(3600);
        loop {
            let (addr, geo_on, have_geo) = match n2.lock() {
                Ok(g) => (g.addr.clone(), g.geo_on, g.geo.is_some()),
                Err(_) => return,
            };
            let ping = ping(&addr);
            let ip = local_ip(&addr).unwrap_or_else(|| "--".into());
            let mut geo = None;
            if geo_on && ping.is_some() && !have_geo && last_geo.elapsed() > Duration::from_secs(60) {
                last_geo = Instant::now();
                geo = geolocate();
            }
            if let Ok(mut g) = n2.lock() {
                g.ping_ms = ping;
                g.online = ping.is_some();
                g.ip = ip;
                if geo.is_some() {
                    g.geo = geo;
                }
                if !g.geo_on {
                    g.geo = None;
                }
            }
            std::thread::sleep(Duration::from_secs(2));
        }
    });
    n
}

fn ping(addr: &str) -> Option<f32> {
    for port in [443u16, 53, 80] {
        if let Some(sa) = format!("{addr}:{port}").to_socket_addrs().ok().and_then(|mut i| i.next()) {
            let t = Instant::now();
            if TcpStream::connect_timeout(&sa, Duration::from_millis(800)).is_ok() {
                return Some(t.elapsed().as_secs_f32() * 1000.0);
            }
        }
    }
    None
}

fn local_ip(addr: &str) -> Option<String> {
    let sa = format!("{addr}:80").to_socket_addrs().ok()?.next()?;
    let s = UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect(sa).ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

fn geolocate() -> Option<(f64, f64)> {
    let sa = "ip-api.com:80".to_socket_addrs().ok()?.next()?;
    let mut s = TcpStream::connect_timeout(&sa, Duration::from_secs(3)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(4))).ok()?;
    s.write_all(b"GET /json/?fields=status,lat,lon HTTP/1.1\r\nHost: ip-api.com\r\nConnection: close\r\n\r\n").ok()?;
    let mut body = String::new();
    s.read_to_string(&mut body).ok()?;
    let (a, b) = (body.find('{')?, body.rfind('}')?);
    let v: serde_json::Value = serde_json::from_str(&body[a..=b]).ok()?;
    if v["status"].as_str() != Some("success") {
        return None;
    }
    Some((v["lat"].as_f64()?, v["lon"].as_f64()?))
}
