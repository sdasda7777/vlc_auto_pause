
use std::env;
use windows::Media::Control::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::AUTHORIZATION;

trait SessionManager {
    fn is_any_other_playing(&self) -> bool;
}

struct WindowsSessionManager {
    manager: GlobalSystemMediaTransportControlsSessionManager,
}

impl WindowsSessionManager {
    fn new() -> Self {
        // Get media session manager
        let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
            Err(err) => panic!("`manager.GetSessions()` got {:?}", err),
            Ok(manager) => match manager.get() {
                Err(err) => panic!("`manager.get()` got {:?}", err),
                Ok(manager) => manager,
            }
        };
        Self { manager }
    }
}

impl SessionManager for WindowsSessionManager {
    fn is_any_other_playing(&self) -> bool {
        if let Ok(sessions) = self.manager.GetSessions() {
            for e in sessions {
                if let Ok(info) = e.GetPlaybackInfo() {
                    if let Ok(status) = info.PlaybackStatus() {
                        if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                            || status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing {
                            return true;
                        }
                    }
                }
            }
        }
        return false;
    }
}



/// Find named argument, parse it to T
fn find_parse<T: std::str::FromStr>(
    args: &[String],
    name: &str
) -> Option<T> {
    args.windows(2)
        .find(|e| e[0] == name)
        .and_then(|e| e[1].parse::<T>().ok())
}


// Main function for Windows
#[cfg(target_os = "windows")]
fn main() {
    do_stuff(&env::args().collect::<Vec<_>>(), WindowsSessionManager::new());
}

// Main function for Linux
#[cfg(target_os = "linux")]
fn main() {
    do_stuff(&env::args().collect::<Vec<_>>(), MPRISSessionManager::new());
}

fn do_stuff(args: &[String], manager: impl SessionManager) {
    
    let check_interval = find_parse(args, "--check-interval").unwrap_or(1000);
    
    // URLs
    let base_url = find_parse(args, "--vlc-base-url").unwrap_or("http://localhost:8080".to_string());
    let status_url = format!("{}/requests/status.json", base_url);
    let command_url = format!("{}/requests/status.xml?command=pl_pause", base_url);
    
    // username is (afaik) always blank, password is your vlc http server password
    let username = "";
    let password: String = find_parse(args, "--vlc-http-password")
              .expect("Error: Mandatory argument `--vlc-http-password` not found");
    
    let authstring = "Basic ".to_owned()
                        + &STANDARD.encode(format!("{}:{}", username, password));
    
    let client = reqwest::blocking::Client::new();
    
    // Get current VLC state (very jank)
    let mut vlc_currently_paused = !client.get(status_url)
                                        .header(AUTHORIZATION, authstring.clone()).send()
                                        .map_err(|_|())
                                        .and_then(|e| e.text().map_err(|_|())?.find("\"state\":\"playing\"").ok_or(()))
                                        .is_ok();
    
    // Main loop
    loop {
        std::thread::sleep(std::time::Duration::from_millis(check_interval));
        if manager.is_any_other_playing() != vlc_currently_paused {
            let req = client.get(command_url.clone())
                .header(AUTHORIZATION, authstring.clone());
            match req.send() { // pause/unpause VLC
                Err(err) => println!("req got {:?}", err),
                Ok(_resp) => (), // println!("req got {:?}", resp),
            }
            
            vlc_currently_paused = !vlc_currently_paused;
        }
    }
}
