#![feature(clamp)]

use std::fs::{self, OpenOptions};
use std::io::{self, prelude::*, Write};
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
    #[allow(dead_code)]
    gpu_load: Option<f64>,
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
    fn get_gpu_temp(&self) -> Option<f64> {
        self.get_temp("GPU-therm")
    }
}

struct SysFs {}

impl SysFs {
    fn read_file(path: &PathBuf) -> Option<String> {
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .create_new(false)
            .open(path);
        let mut buffer = String::new();

        file.ok()
            .and_then(|mut f| f.read_to_string(&mut buffer).ok())
            .map(|_| buffer)
    }

    fn get_thermal_zone(path_buf: &PathBuf) -> Option<ThermalZone> {
        let name_path = path_buf.join("type");

        Self::read_file(&name_path).and_then(|name| {
            let name = name.trim().to_string();

            let mode_path = path_buf.join("mode");
            let enabled = Self::read_file(&mode_path).map(|s| s.trim() == "enabled");

            let temp_path = path_buf.join("temp");
            let temperature = Self::read_file(&temp_path)
                .and_then(|s| s.trim().parse::<f64>().ok())
                .map(|s| s / 1000.0);

            let thermal = ThermalZone {
                name,
                enabled,
                temperature,
            };

            Some(thermal)
        })
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
                            if let Some(thermal) = Self::get_thermal_zone(&path_buf) {
                                res.push(thermal);
                            }
                        }
                    }
                }
            }
        }
        res
    }

    fn read_gpu_load() -> Option<f64> {
        Self::read_file(&PathBuf::from("/sys/devices/gpu.0/load"))
            .and_then(|s| s.trim().parse::<f64>().ok())
            .map(|s| s / 10.0)
    }

    fn set_fan_pwm(pwm: u8) {
        let file = OpenOptions::new()
            .read(false)
            .write(true)
            .create_new(false)
            .open("/sys/devices/pwm-fan/target_pwm");

        file.and_then(|file| {
            let mut buf = Vec::with_capacity(3);
            let _ = itoa::write(&mut buf, pwm);
            Ok((file, buf))
        })
        .and_then(|(mut file, buf)| file.write_all(&buf))
        .err()
        .map(|err| println!("{:?}", err));
    }
}

struct PwmCalculator {
    cpu_temp: Option<f64>,
    gpu_temp: Option<f64>,
}

impl PwmCalculator {
    fn max(o1: Option<f64>, o2: Option<f64>) -> Option<f64> {
        match o1 {
            Some(f1) => match o2 {
                Some(f2) => Some(f1.max(f2)),
                None => o1,
            },
            None => o2,
        }
    }

    fn get_pwm(&self) -> Option<u8> {
        PwmCalculator::max(self.cpu_temp, self.gpu_temp).map(|cpu_temp| {
            const FAN_OFF_TEMP: f64 = 35.0;
            const FAN_MAX_TEMP: f64 = 50.0;
            let spd = (u8::MAX as f64) * (cpu_temp - FAN_OFF_TEMP) / (FAN_MAX_TEMP - FAN_OFF_TEMP);
            spd.clamp(u8::MIN as f64, u8::MAX as f64) as u8
        })
    }
}

fn main() -> io::Result<()> {
    loop {
        let system_info = SystemInfo {
            temperatures: SysFs::read_temperatures(),
            gpu_load: SysFs::read_gpu_load(),
        };

        let pwm_calculator = PwmCalculator {
            cpu_temp: system_info.get_cpu_temp(),
            gpu_temp: system_info.get_gpu_temp(),
        };
        if let Some(pwm) = pwm_calculator.get_pwm() {
            // let cpu_temp = pwm_calculator.cpu_temp.unwrap();
            // let gpu_temp = pwm_calculator.gpu_temp.unwrap();
            // println!(
            //     "CPU temp: {:?} GPU temp: {:?} pwm: {}",
            //     cpu_temp, gpu_temp, pwm
            // );
            SysFs::set_fan_pwm(pwm);
        }

        thread::sleep(time::Duration::from_secs(5));
    }
}
