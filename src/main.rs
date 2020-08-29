use std::fs::OpenOptions;
use std::io::Write;
use std::{thread, time};

struct SystemInfo {}

struct Output {
    pwm: u8,
}

fn read_system_info() -> SystemInfo {
    SystemInfo {}
}

fn process(systemInfo: &SystemInfo) -> Output {
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
            let str = pwm.to_string();
            let bytes = str.as_bytes();
            match f.write_all(bytes) {
                Ok(_) => {}
                Err(_) => {}
            }
        }
        Err(_) => {}
    }
}

fn sleep() {
    thread::sleep(time::Duration::from_secs(1));
}

fn main() -> std::io::Result<()> {
    loop {
        let input = read_system_info();
        let output = process(&input);
        set_fan_pwm(output.pwm);
        sleep();
    }
}
