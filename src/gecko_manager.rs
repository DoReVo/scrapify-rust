use std::{error::Error, process::Stdio};

use tokio::process::{self, Child};

pub async fn start() -> Child {
    process::Command::new("geckodriver")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .arg("-p")
        .arg("7000")
        .spawn()
        .expect("Cannot start geckodriver")
}

pub async fn stop(mut ch: Child) -> Result<(), Box<dyn Error>> {
    match ch.kill().await {
        Ok(_) => {
            print!("geckodriver killed");
            Ok(())
        }
        Err(err) => {
            panic!("Error in killing geckodriver {}", err.to_string())
        }
    }
}
