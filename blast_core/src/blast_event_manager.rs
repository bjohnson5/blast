use std::sync::Arc;
use std::fmt;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::fs::File;
use std::io::BufReader;

use bitcoincore_rpc::Client;
use bitcoincore_rpc::Auth;
use anyhow::Error;
use tokio::sync::mpsc::Sender;
use serde::{Serialize, Deserialize};

pub const FRAME_RATE: u64 = 1;
pub const MINE_RATE: u64 = 5;
pub const BLOCKS_PER_MINE: u64 = 10;

pub struct BlastEventManager {
    running: Arc<AtomicBool>,
    events: HashMap<u64,Vec<BlastEvent>>,
    bitcoin_rpc: Option<Client>,
}

impl Clone for BlastEventManager {
    fn clone(&self) -> Self {
        Self {
            running: self.running.clone(),
            events: self.events.clone(),
            bitcoin_rpc: match Client::new("http://127.0.0.1:18443/", Auth::UserPass(String::from("user"), String::from("pass"))) {
                Ok(c) => Some(c),
                Err(_) => None
            }
        }
    }
}

impl BlastEventManager {
    pub fn new() -> Self {
        let event_manager = BlastEventManager {
            running: Arc::new(AtomicBool::new(false)),
            events: HashMap::new(),
            bitcoin_rpc: match Client::new("http://127.0.0.1:18443/", Auth::UserPass(String::from("user"), String::from("pass"))) {
                Ok(c) => Some(c),
                Err(_) => None
            }
        };

        event_manager
    }

    /// Start the event thread.
    pub async fn start(&mut self, sender: Sender<BlastEvent>) -> Result<(), Error> {
        self.running.store(true, Ordering::SeqCst);
        let mut frame_num = 0;
        loop {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            log::info!("BlastEventManager running frame number {}", frame_num);

            if frame_num % MINE_RATE == 0 {
                match crate::mine_blocks(&mut self.bitcoin_rpc, BLOCKS_PER_MINE) {
                    Ok(_) => {},
                    Err(e) => return Err(anyhow::Error::msg(e)),
                }
            }

            if self.events.contains_key(&frame_num) {
                let current_events = &self.events[&frame_num];
                let current_events_iter = current_events.iter();
                for e in current_events_iter {
                    log::info!("BlastEventManager sending event {}", e);
                    if sender.send(e.clone()).await.is_err() {
                        return Err(anyhow::Error::msg("Error sending event."));
                    }
                }
            }

            frame_num = frame_num + 1;
            tokio::time::sleep(Duration::from_secs(FRAME_RATE)).await;
        }
        Ok(())
    }

    /// Stop the event thread.
    pub fn stop(&mut self) {
        log::info!("BlastEventManager stopping simulation.");
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get the events in json format.
    pub fn get_event_json(&self) -> Result<String, String> {
        match serde_json::to_string(&self.events) {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting event data: {}", e))
        }
    }

    /// Set the events from a json file.
    pub fn set_event_json(&mut self, path: &str) -> Result<(), String> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => return Err(format!("Error opening event file: {}", e))
        };

        let reader = BufReader::new(file);
        self.events = match serde_json::from_reader(reader) {
            Ok(e) => e,
            Err(e) => return Err(format!("Error reading event data: {}", e))
        };

        Ok(())
    }

    /// Create an event for the simulation.
    pub fn add_event(&mut self, frame_num: u64, event: &str, args: Option<Vec<String>>) -> Result<(), String> {
        if let Some(e) = BlastEvent::from_str(event) {
            match e {
                BlastEvent::StartNodeEvent(_) => {
                    let a = self.validate_args(args, e)?;
                    let blast_event = BlastEvent::StartNodeEvent(a.get(0).unwrap().to_owned());
                    self.push_event(frame_num, blast_event);
                    Ok(())
                },
                BlastEvent::StopNodeEvent(_) => {
                    let a = self.validate_args(args, e)?;
                    let blast_event = BlastEvent::StopNodeEvent(a.get(0).unwrap().to_owned());
                    self.push_event(frame_num, blast_event);
                    Ok(())
                },
                BlastEvent::OpenChannelEvent(_,_,_,_,_) => {
                    let a = self.validate_args(args, e)?;
                    let arg0 = a.get(0).unwrap().to_owned();
                    let arg1 = a.get(1).unwrap().to_owned();
                    let arg2 =  match a.get(2).unwrap().to_owned().parse::<i64>() {
                        Ok(n) => n,
                        Err(e) => return Err(format!("Error parsing argument: {}", e))
                    };
                    let arg3 =  match a.get(3).unwrap().to_owned().parse::<i64>() {
                        Ok(n) => n,
                        Err(e) => return Err(format!("Error parsing argument: {}", e))
                    };
                    let arg4 =  match a.get(4).unwrap().to_owned().parse::<i64>() {
                        Ok(n) => n,
                        Err(e) => return Err(format!("Error parsing argument: {}", e))
                    };
                    let blast_event = BlastEvent::OpenChannelEvent(arg0, arg1, arg2, arg3, arg4);
                    self.push_event(frame_num, blast_event);
                    Ok(())
                },
                BlastEvent::CloseChannelEvent(_,_) => {
                    let a = self.validate_args(args, e)?;
                    let arg0 = a.get(0).unwrap().to_owned();
                    let arg1 =  match a.get(1).unwrap().to_owned().parse::<i64>() {
                        Ok(n) => n,
                        Err(e) => return Err(format!("Error parsing argument: {}", e))
                    };
    
                    let blast_event = BlastEvent::CloseChannelEvent(arg0, arg1);
                    self.push_event(frame_num, blast_event);
                    Ok(())
                },
                BlastEvent::OnChainTransaction(_, _, _) => {
                    let a = self.validate_args(args, e)?;
                    let arg0 = a.get(0).unwrap().to_owned();
                    let arg1 = a.get(1).unwrap().to_owned();
                    let arg2 =  match a.get(2).unwrap().to_owned().parse::<i64>() {
                        Ok(n) => n,
                        Err(e) => return Err(format!("Error parsing argument: {}", e))
                    };
    
                    let blast_event = BlastEvent::OnChainTransaction(arg0, arg1, arg2);
                    self.push_event(frame_num, blast_event);
                    Ok(())
                }
            }
        } else {
            Err(format!("Could not parse the BlastEvent"))
        }
    }

    // Validate that the correct args were given.
    fn validate_args(&self, args: Option<Vec<String>>, event: BlastEvent) -> Result<Vec<String>, String> {
        match args {
            Some(a) => {
                if a.len() != event.num_fields() {
                    return Err(format!("Not the correct number of args for {}.", event));
                }
                Ok(a)
            },
            None => return Err(format!("No args given for {}.", event))
        }
    }

    /// Create an event for the simulation.
    fn push_event(&mut self, frame_num: u64, event: BlastEvent) {
        if self.events.contains_key(&frame_num) {
            let current_events = self.events.get_mut(&frame_num).unwrap();
            current_events.push(event);
        } else {
            let mut current_events: Vec<BlastEvent> = Vec::new();
            current_events.push(event);
            self.events.insert(frame_num, current_events);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BlastEvent {
    StartNodeEvent(String),
    StopNodeEvent(String),
    OpenChannelEvent(String, String, i64, i64, i64),
    CloseChannelEvent(String, i64),
    OnChainTransaction(String, String, i64)
}

impl fmt::Display for BlastEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlastEvent::StartNodeEvent(a) => write!(f, "StartNodeEvent: {}", a),
            BlastEvent::StopNodeEvent(a) => write!(f, "StopNodeEvent: {}", a),
            BlastEvent::OpenChannelEvent(a, b, c, d, e) => write!(f, "OpenChannelEvent: {} {} {} {} {}", a, b, c, d, e),
            BlastEvent::CloseChannelEvent(a, b) => write!(f, "CloseChannelEvent: {} {}", a, b),
            BlastEvent::OnChainTransaction(a, b, c) => write!(f, "OnChainTransaction: {} {} {}", a, b, c)
        }
    }
}

impl BlastEvent {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "StartNode" => Some(BlastEvent::StartNodeEvent(String::from(""))),
            "StopNode" => Some(BlastEvent::StopNodeEvent(String::from(""))),
            "OpenChannel" => Some(BlastEvent::OpenChannelEvent(String::from(""), String::from(""), 0, 0, 0)),
            "CloseChannel" => Some(BlastEvent::CloseChannelEvent(String::from(""), 0)),
            "OnChainTransaction" => Some(BlastEvent::OnChainTransaction(String::from(""), String::from(""), 0)),
            _ => None
        }
    }

    pub fn num_fields(&self) -> usize {
        match self {
            BlastEvent::StartNodeEvent(_) => 1,
            BlastEvent::StopNodeEvent(_) => 1,
            BlastEvent::OpenChannelEvent(_, _, _, _, _) => 5,
            BlastEvent::CloseChannelEvent(_, _) => 2,
            BlastEvent::OnChainTransaction(_, _, _) => 3,
        }
    }
}
