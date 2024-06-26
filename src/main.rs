use std::env; // read env variables
use std::fs::File;
use std::io::BufRead; // read unix socket
use std::io::BufReader; // read unix socket
use std::os::unix::fs::PermissionsExt; // check file permissions
use std::os::unix::net::UnixStream;
use std::process::Command; // execute system command

// listen Hyprland socket
fn listen(
    socket_addr: String,
    script_attached: &str,
    script_detached: Option<&str>,
) -> std::io::Result<()> {
    let stream = match UnixStream::connect(socket_addr) {
        Ok(stream) => stream,
        Err(e) => {
            println!("Couldn't connect: {e:?}");
            return Err(e);
        }
    };
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: provide a script to execute.");
        std::process::exit(1);
    }
    let mut reader = BufReader::new(stream);
    loop {
        // read message from socket
        let mut buf: Vec<u8> = vec![];
        reader.read_until(b'\n', &mut buf).unwrap();
        let data = String::from_utf8_lossy(&buf);
        let data_parts: Vec<&str> = data.trim().split(">>").collect();
        if data_parts[0] == "monitoradded" {
            // check user has permission to execute script
            let metadata = {
                let this = File::open(script_attached);
                match this {
                    Ok(t) => t,
                    Err(_e) => {
                        eprintln!("Error: '{script_attached}' file not found.");
                        continue;
                    }
                }
            }
            .metadata()
            .unwrap();
            let permissions = metadata.permissions();
            if !permissions.mode() & 0o100 != 0 {
                eprintln!("Error: '{script_attached}' file is not executable.");
                continue;
            }
            Command::new(script_attached)
                .args([data_parts[1]])
                .spawn()
                .expect("Failed to execute command");
        } else if data_parts[0] == "monitorremoved" {
            if let Some(script_detached) = script_detached {
                let metadata = {
                    let this = File::open(script_detached);
                    match this {
                        Ok(t) => t,
                        Err(_e) => {
                            eprintln!("Error: '{script_detached}' file not found.");
                            continue;
                        }
                    }
                }
                .metadata()
                .unwrap();
                let permissions = metadata.permissions();
                if !permissions.mode() & 0o100 != 0 {
                    eprintln!("Error: '{script_detached}' file is not executable.");
                    continue;
                }
                Command::new(script_detached)
                    .args([data_parts[1]])
                    .spawn()
                    .expect("Failed to execute command");
            }
        }
    }
}

// read env variables and listen Hyprland unix socket
fn main() {
    match env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        Ok(hypr_inst) => {
            let default_socket = format!("/tmp/hypr/{}/.socket2.sock", hypr_inst);

            // check if socket is in $XDG_RUNTIME_DIR/hypr first, then fall back
            // for backawards compatibility
            let socket = match env::var("XDG_RUNTIME_DIR") {
                Ok(runtime_dir) => match std::fs::metadata(format!(
                    "{}/hypr/{}/.socket2.sock",
                    runtime_dir, hypr_inst
                )) {
                    Ok(_) => format!("{}/hypr/{}/.socket2.sock", runtime_dir, hypr_inst),
                    Err(..) => default_socket,
                },
                Err(..) => default_socket,
            };

            let script_attached = std::env::args()
                .nth(1)
                .expect("Missing script for monitor attached");
            let script_detached = std::env::args().nth(2);
            // listen Hyprland socket
            match listen(socket, &script_attached, script_detached.as_deref()) {
                Ok(()) => {}
                Err(..) => {}
            }
        }
        Err(e) => println!("Fatal Error: Hyprland is not run. {e}"),
    }
    std::process::exit(1);
}
