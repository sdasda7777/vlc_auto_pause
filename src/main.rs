
use std::env;
use windows::Media::Control::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;

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
    let command_url = format!("{}/requests/status.xml?command=pl_pause", base_url);
    
    // username is (afaik) always blank, password is your vlc http server password
    let username = "";
    let password: String = find_parse(args, "--vlc-http-password")
              .expect("Error: Mandatory argument `--vlc-http-password` not found");
    
    // create Basic authstring
    let authstring = "Basic ".to_owned()
                        + &STANDARD.encode(format!("{}:{}", username, password));
    
    // Get current VLC state as a baseline
    let mut vlc_should_be_paused = !vlc_is_playing(&base_url, &authstring);
    
    // Main loop
    loop {
        std::thread::sleep(std::time::Duration::from_millis(check_interval));
        
        let other_is_playing = manager.is_any_other_playing();
        if other_is_playing != vlc_should_be_paused
           && other_is_playing != !vlc_is_playing(&base_url, &authstring) {
            let req = reqwest::blocking::Client::new()
                        .get(command_url.clone())
                        .header(AUTHORIZATION, authstring.clone());
            match req.send() { // pause/unpause VLC
                Err(err) => println!("req got {:?}", err),
                Ok(_resp) => (), // println!("req got {:?}", resp),
            }
            
            vlc_should_be_paused = !vlc_should_be_paused;
        }
    }
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum VLCPlayState {
    Playing,
    Paused,
    Stopped,
}

#[derive(Deserialize)]
struct VLCStatus {
    state: VLCPlayState,
}

fn vlc_is_playing(base_url: &str, authstring: &str) -> bool {
    reqwest::blocking::Client::new().get(format!("{}/requests/status.json", base_url))
        .header(AUTHORIZATION, authstring).send().map_err(|_|())
        .and_then(|e| 
            match e.json::<VLCStatus>() {
                Ok(s) if s.state == VLCPlayState::Playing => {
                    // println!("VLCPlayState::Playing");
                    Ok(())
                },
                Ok(_) => {
                    // println!("VLCPlayState::other");
                    Err(())
                },
                Err(_err) => {
                    // println!("{}", err);
                    Err(())
                }
            }
        )
        .is_ok()
}
