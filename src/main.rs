#![feature(clamp)]

use io::BufReader;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::io::{self, prelude::*};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::{str, thread, time};

struct ThermalZone {
    name: String,
    enabled: Option<bool>,
    temperature: Option<f64>,
}

struct SystemInfo {
    temperatures: Vec<ThermalZone>,
}

impl SystemInfo {
    fn get_temp(&self, name: &str) -> Option<f64> {
        self.temperatures
            .iter()
            .find(|t| t.name == name && t.enabled.unwrap_or(true))
            .and_then(|t| t.temperature)
    }
    fn get_cpu_temp(&self) -> Option<f64> {
        self.get_temp("CPU-therm")
    }
}

struct Output {
    pwm: Option<u8>,
}

fn read_system_info() -> SystemInfo {
    SystemInfo {
        temperatures: read_temperatures(),
    }
}

fn process(system_info: &SystemInfo) -> Output {
    let pwm = if let Some(cpu_temp) = system_info.get_cpu_temp() {
        const FAN_OFF_TEMP: f64 = 30.0;
        const FAN_MAX_TEMP: f64 = 50.0;
        let spd = (u8::MAX as f64) * (cpu_temp - FAN_OFF_TEMP) / (FAN_MAX_TEMP - FAN_OFF_TEMP);
        Some(spd.clamp(u8::MIN as f64, u8::MAX as f64) as u8)
    } else {
        None
    };

    Output { pwm }
}

fn set_fan_pwm(pwm: u8) {
    let file = OpenOptions::new()
        .read(false)
        .write(true)
        .create_new(false)
        .open("/sys/devices/pwm-fan/target_pwm");

    match file {
        Ok(mut file) => {
            let mut buf = Vec::with_capacity(3);
            match itoa::write(&mut buf, pwm) {
                Ok(_) => match file.write_all(&buf) {
                    Ok(_) => {}
                    Err(err) => {
                        println!("{:?}", err);
                    }
                },
                Err(err) => {
                    println!("{:?}", err);
                }
            }
        }
        Err(err) => {
            println!("{:?}", err);
        }
    }
}

fn sleep(secs: u64) {
    thread::sleep(time::Duration::from_secs(secs));
}

fn read_file(path: &PathBuf) -> std::io::Result<String> {
    let file = OpenOptions::new()
        .read(true)
        .write(false)
        .create_new(false)
        .open(path);

    match file {
        Ok(inner) => {
            let mut buf_reader = BufReader::new(inner);
            let mut buf = String::new();
            buf_reader.read_to_string(&mut buf)?;
            Ok(buf)
        }
        Err(err) => Err(err),
    }
}

fn get_thermal_zone(path_buf: &PathBuf) -> Option<ThermalZone> {
    let name_path = path_buf.join("type");
    match read_file(&name_path) {
        Ok(name) => {
            let name = name.trim().to_string();

            let mode_path = path_buf.join("mode");
            let enabled = read_file(&mode_path).map(|s| s.trim() == "enabled").ok();

            let temp_path = path_buf.join("temp");
            let temperature = read_file(&temp_path)
                .ok()
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|s| s / 1000.0);

            let thermal = ThermalZone {
                name,
                enabled,
                temperature,
            };

            Some(thermal)
        }
        Err(err) => {
            println!("{:?}", err);
            None
        }
    }
}

fn read_temperatures() -> Vec<ThermalZone> {
    let mut res = vec![];
    if let Ok(read_dir) = fs::read_dir("/sys/devices/virtual/thermal/") {
        for dir_entry in read_dir {
            if let Ok(dir_entry) = dir_entry {
                let path_buf = dir_entry.path();
                if let Some(file) = path_buf.file_name() {
                    if str::from_utf8(file.as_bytes())
                        .unwrap()
                        .starts_with("thermal_zone")
                    {
                        if let Some(thermal) = get_thermal_zone(&path_buf) {
                            res.push(thermal);
                        }
                    }
                }
            }
        }
    }
    res
}

fn main() -> io::Result<()> {
    loop {
        let system_info = read_system_info();
        let output = process(&system_info);
        if let Some(pwm) = output.pwm {
            let cpu_temp = system_info.get_cpu_temp();
            println!("temp: {:?} pwm: {}", cpu_temp, pwm);
            set_fan_pwm(pwm);
        }
        sleep(5);
    }
}
