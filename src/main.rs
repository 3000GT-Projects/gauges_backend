use std::{error::Error, result, time::Duration};

use dto::dto::{Configuration, GaugeData, InMessage, OutMessage};
use serde::{Deserialize, Serialize};
use serde_json;
use serialport::{self, SerialPort};

mod dto;

fn get_port() -> Option<Box<dyn serialport::SerialPort>> {
    println!("Searching for serial ports...");

    let ports = serialport::available_ports().expect("No ports found!");

    for port_info in ports {
        println!("{}", port_info.port_name);

        // FIXME: port_name as path probably won't work on Linux
        let port = serialport::new(port_info.port_name, 115_200)
            .timeout(Duration::from_millis(1000))
            .open()
            .expect("Failed to open port");

        println!("Port {} opened", port.name().expect("No port name!"));

        return Some(port);
    }

    return None;
}

// NOTE: this function drops some messages but it should suffice
fn read_message_string(port: &mut Box<dyn SerialPort>) -> Result<String, Box<dyn Error>> {
    let mut message_string_buffer: Vec<u8> = Vec::new();
    let message_end_byte = '\n' as u8;

    let mut found_message_start = false;
    let mut found_message_end = false;

    while !found_message_end {
        let mut message_buffer: [u8; 20] = [0; 20];
        let result = port.read(&mut message_buffer);

        match result {
            Ok(size) => {
                let (message_bytes, _) = message_buffer.split_at(size);

                for byte_ref in message_bytes {
                    let byte = byte_ref.to_owned();

                    if byte == message_end_byte {
                        if !found_message_start {
                            found_message_start = true;
                            continue;
                        } else if !found_message_end {
                            found_message_end = true;
                            continue;
                        }
                    }

                    if found_message_start && !found_message_end {
                        message_string_buffer.push(byte);
                    }
                }
            }
            Err(error) => {
                return Err(Box::new(error));
            }
        }
    }

    match String::from_utf8(message_string_buffer) {
        Ok(string) => {
            return Ok(string);
        }
        Err(error) => {
            return Err(Box::new(error));
        }
    }
}

fn read_message(port: &mut Box<dyn SerialPort>) -> Result<dto::dto::InMessage, Box<dyn Error>> {
    match read_message_string(port) {
        Ok(json_string) => match serde_json::from_str::<dto::dto::InMessage>(&json_string) {
            Ok(json_value) => {
                return Ok(json_value);
            }
            Err(error) => {
                return Err(Box::new(error));
            }
        },
        Err(error) => {
            return Err(error);
        }
    }
}

fn handle_message(message: &InMessage) -> OutMessage {
    match message {
        InMessage::NeedGaugeConfig {} => {
            let result = OutMessage::Configuration {
                message: dto::dto::Configuration {
                    theme: dto::dto::GaugeTheme::default(),
                    display1: dto::dto::DisplayConfiguration {
                        gauges: vec![dto::dto::GaugeConfig {
                            name: String::from("COOLANT"),
                            units: String::from("C"),
                            format: String::from("%.0f"),
                            min: 0.0,
                            max: 130.0,
                            low_value: 60.0,
                            high_value: 100.0,
                        }],
                    },
                    display2: dto::dto::DisplayConfiguration {
                        gauges: vec![dto::dto::GaugeConfig {
                            name: String::from("OIL"),
                            units: String::from("bar"),
                            format: String::from("%.2f"),
                            min: 0.0,
                            max: 10.0,
                            low_value: 1.0,
                            high_value: 8.0,
                        }],
                    },
                    display3: dto::dto::DisplayConfiguration { gauges: vec![] },
                },
            };

            return result;
        }
        InMessage::NeedGaugeData {} => {
            let result = OutMessage::Data {
                message: dto::dto::Data {
                    display1: dto::dto::DisplayData {
                        gauges: vec![dto::dto::GaugeData {
                            // COOLANT C
                            current_value: 77.0,
                        }],
                    },
                    display2: dto::dto::DisplayData {
                        gauges: vec![dto::dto::GaugeData {
                            // OIL bar
                            current_value: 6.5,
                        }],
                    },
                    display3: dto::dto::DisplayData { gauges: vec![] },
                },
            };

            return result;
        }
    }
}

fn main() {
    loop {
        match get_port() {
            Some(mut port) => match port.write_data_terminal_ready(true) {
                Err(error) => {
                    println!("Error activating port: {}", error);
                    std::thread::sleep(Duration::from_secs(1));
                }
                Ok(_) => loop {
                    match read_message(&mut port) {
                        Ok(message) => {
                            println!("InMessage: {}", message);
                            println!(
                                "OutMessage: {}",
                                serde_json::to_string(&handle_message(&message)).unwrap()
                            );
                            port.write_all(&serde_json::to_vec(&handle_message(&message)).unwrap());
                        }
                        Err(error) => {
                            if error.is::<std::io::Error>() {
                                println!(
                                    "IO error while working with port: {}; Abandoning port...",
                                    error
                                );
                                break;
                            }

                            println!("Transient error while working with port: {}", error);
                        }
                    }
                },
            },
            None => {
                println!("Waiting for port...");
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
