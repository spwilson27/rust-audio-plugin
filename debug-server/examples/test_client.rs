use debug_server::{DebugControlClient, InputEventMsg, MouseMsg};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read port from lockfile
    // Wait a bit for server to start if we run them together
    std::thread::sleep(std::time::Duration::from_secs(2));

    let pid_file = std::env::temp_dir().join(format!("splug_pid_{}.json", find_standalone_pid()?));
    println!("Reading lockfile: {:?}", pid_file);
    let content = std::fs::read_to_string(&pid_file)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;
    let port = json["port"].as_u64().ok_or("Port not found")?;

    let addr = format!("http://127.0.0.1:{}", port);
    println!("Connecting to {}", addr);

    let mut client = DebugControlClient::connect(addr).await?;

    println!("Sending MouseDown event...");
    let request = tonic::Request::new(InputEventMsg {
        event: Some(debug_server::debug_control::input_event_msg::Event::Mouse(
            MouseMsg {
                r#type: 0, // Down
                x: 100.0,
                y: 100.0,
                button: 0,
            },
        )),
    });

    let response = client.send_input_event(request).await?;
    println!("RESPONSE={:?}", response);

    Ok(())
}

fn find_standalone_pid() -> Result<u32, Box<dyn std::error::Error>> {
    // Hacky way to find the pid file. In practice pass as arg.
    // Here we just look for any splug_pid file in temp
    let temp_dir = std::env::temp_dir();
    for entry in std::fs::read_dir(temp_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with("splug_pid_") && name_str.ends_with(".json") {
            // Parse pid from filename
            let pid_str = &name_str["splug_pid_".len()..name_str.len() - 5];
            return Ok(pid_str.parse()?);
        }
    }
    Err("No standalone PID file found".into())
}
