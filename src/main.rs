use io::BufReader;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use std::io::Write;
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::str;
use std::{collections::HashMap, thread, time};

struct Thermal {
    name: String,
    enabled: Option<bool>,
    temp: Option<f64>,
}

struct SystemInfo {
    temperatures: Vec<Thermal>,
}

struct Output {
    pwm: u8,
}

fn read_system_info() -> SystemInfo {
    SystemInfo {
        temperatures: read_temperatures(),
    }
}

fn process(system_info: &SystemInfo) -> Output {
    let cpu_temp = system_info
        .temperatures
        .into_iter()
        .find(|t| t.name == "CPU-therm")
        .unwrap();

    Output { pwm: 255 }
}

fn set_fan_pwm(pwm: u8) {
    let file = OpenOptions::new()
        .read(false)
        .write(true)
        .create_new(false)
        .open("/sys/devices/pwm-fan/target_pwm");

    match file {
        Ok(mut f) => {
            let mut buf = Vec::with_capacity(3);
            match itoa::write(&mut buf, pwm) {
                Ok(_) => match f.write_all(&buf) {
                    Ok(_) => {}
                    Err(_) => {}
                },
                Err(_) => {}
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

fn get_thermal_zone(path_buf: &PathBuf) -> Option<Thermal> {
    let name_path = path_buf.join("type");
    match read_file(&name_path) {
        Ok(name) => {
            let name = name.trim().to_string();

            let mode_path = path_buf.join("mode");
            let enabled = read_file(&mode_path).map(|s| s.trim() == "enabled").ok();

            let temp_path = path_buf.join("temp");
            let temp = read_file(&temp_path)
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .map(|s| s / 1000.0);

            let thermal = Thermal {
                name,
                enabled,
                temp,
            };

            Some(thermal)
        }
        Err(_) => None,
    }
}

fn read_temperatures() -> Vec<Thermal> {
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
        let input = read_system_info();
        let output = process(&input);
        set_fan_pwm(output.pwm);
        sleep(20);
    }
}
