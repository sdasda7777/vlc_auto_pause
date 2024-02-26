
use std::env;
use windows::Media::Control::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use reqwest::header::AUTHORIZATION;

/// Find named argument, parse it to T
fn find_parse<T: std::str::FromStr>(
    args: &[String],
    name: &str
) -> Option<T> {
    args.windows(2)
        .find(|e| e[0] == name)
        .and_then(|e| e[1].parse::<T>().ok())
}

// Main function
fn main() {
    
    let args: Vec<_> = env::args().collect();
    
    let status_url = "http://localhost:8080/requests/status.json";
    let command_url = "http://localhost:8080/requests/status.xml?command=pl_pause";
    // username is (afaik) always blank, password is your vlc http server password
    let username = "";
    let password: String = find_parse(&args, "--vlc-http-password")
              .expect("Error: Expected vlc password as `--vlc-http-password` argument");
    
    let authstring = "Basic ".to_owned()
                        + &STANDARD.encode(format!("{}:{}", username, password));
    
    let client = reqwest::blocking::Client::new();
    macro_rules! vlc_play_pause { () => {
        let req = client.get(command_url)
            .header(AUTHORIZATION, authstring.clone());
        match req.send() {
            Err(err) => println!("req got {:?}", err),
            Ok(_resp) => (), // println!("req got {:?}", resp),
        }
    }}
    
    // Get current VLC state (very jank)
    let mut vlc_currently_paused = !client.get(status_url)
                                        .header(AUTHORIZATION, authstring.clone()).send()
                                        .map_err(|_|())
                                        .and_then(|e| e.text().map_err(|_|())?.find("\"state\":\"playing\"").ok_or(()))
                                        .is_ok();
    
    // Get media session manager
    let manager = match GlobalSystemMediaTransportControlsSessionManager::RequestAsync() {
        Err(err) => panic!("`manager.GetSessions()` got {:?}", err),
        Ok(manager) => match manager.get() {
            Err(err) => panic!("`manager.get()` got {:?}", err),
            Ok(manager) => manager,
        }
    };
    
    'outer: loop {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        match manager.GetSessions() {
            Err(err) => println!("`manager.GetSessions()` is {:?}", err),
            Ok(sessions) => {
                
                for e in sessions {
                    match e.GetPlaybackInfo() {
                        Err(err) => println!("`GetPlaybackInfo` got {:?}", err),
                        Ok(info) => match info.PlaybackStatus() {
                            Err(err) => println!("`PlaybackStatus` got {:?}", err),
                            Ok(status) => {
                                if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing
                                    || status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Changing {
                                    if !vlc_currently_paused {
                                        vlc_play_pause!(); // pause VLC
                                        vlc_currently_paused = true;
                                    }
                                    continue 'outer;
                                }
                            }
                        }
                    }
                }
                
                // No sessions can be currently playing
                if vlc_currently_paused {
                    vlc_play_pause!(); // unpause VLC
                    vlc_currently_paused = false;
                }
            }
        }
    }
}
