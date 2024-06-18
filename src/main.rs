use core::fmt;
use std::time::Duration;

use dto::dto::{InMessage, OutMessage};
use serde_json;
use serialport::{self, SerialPort};

mod dto;

const MESSAGE_END_BYTE: u8 = '\n' as u8;

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

enum Error {
    IO(std::io::Error),
    UtfConversion(std::string::FromUtf8Error),
    JsonParsing {
        error: serde_json::Error,
        source_string: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IO(error) => error.fmt(f),
            Self::UtfConversion(error) => error.fmt(f),
            Self::JsonParsing {
                error,
                source_string,
            } => {
                write!(f, "{} source string: {}", error, source_string)
            }
        }
    }
}

fn read_message_string(port: &mut Box<dyn SerialPort>) -> Result<String, Error> {
    let mut message_string_buffer: Vec<u8> = Vec::new();

    let mut found_message_start = false;
    let mut found_message_end = false;

    while !found_message_end {
        let mut message_buffer: [u8; 1] = [0; 1];
        let result = port.read(&mut message_buffer);

        match result {
            Ok(size) => {
                let (message_bytes, _) = message_buffer.split_at(size);

                for byte_ref in message_bytes {
                    let byte = byte_ref.to_owned();

                    if byte == MESSAGE_END_BYTE {
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
                return Err(Error::IO(error));
            }
        }
    }

    match String::from_utf8(message_string_buffer) {
        Ok(string) => {
            return Ok(string);
        }
        Err(error) => {
            return Err(Error::UtfConversion(error));
        }
    }
}

fn read_message(
    port: &mut Box<dyn SerialPort>,
    is_communication_begin: &mut bool,
) -> Result<dto::dto::InMessage, Error> {
    if *is_communication_begin {
        *is_communication_begin = false;
        return Ok(InMessage::NeedGaugeConfig {});
    }

    match read_message_string(port) {
        Ok(json_string) => match serde_json::from_str::<dto::dto::InMessage>(&json_string) {
            Ok(json_value) => {
                return Ok(json_value);
            }
            Err(error) => {
                return Err(Error::JsonParsing {
                    error: error,
                    source_string: json_string,
                });
            }
        },
        Err(error) => {
            return Err(error);
        }
    }
}

fn handle_error(error: Error) -> Result<(), Error> {
    // Cast the error to `&dyn Any` to use `is::<T>()` method
    if matches!(error, Error::IO(_)) {
        println!(
            "IO error while working with port: {}; Abandoning port...",
            error
        );
        return Err(error);
    }

    println!("Transient error while working with port: {}", error);
    return Ok(());
}

fn handle_message(message: &InMessage) -> Option<OutMessage> {
    use rand::prelude::*;

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

            return Some(result);
        }
        InMessage::NeedGaugeData {} => {
            let mut rng = rand::thread_rng();
            let factor = rng.gen::<f32>();

            let result = OutMessage::Data {
                message: dto::dto::Data {
                    display1: dto::dto::DisplayData {
                        gauges: vec![dto::dto::GaugeData {
                            // COOLANT C
                            current_value: 77.0 * factor,
                        }],
                    },
                    display2: dto::dto::DisplayData {
                        gauges: vec![dto::dto::GaugeData {
                            // OIL bar
                            current_value: 6.5 * factor,
                        }],
                    },
                    display3: dto::dto::DisplayData { gauges: vec![] },
                },
            };

            return Some(result);
        }
        InMessage::Debug { message } => {
            println!("Debug: {}", message);
            return None;
        }
    }
}

fn write_message(
    port: &mut Box<dyn SerialPort>,
    message: dto::dto::OutMessage,
) -> Result<(), Error> {
    println!("OutMessage: {}", serde_json::to_string(&message).unwrap());

    let mut out_message_buf = serde_json::to_vec(&message).unwrap();

    out_message_buf.push(MESSAGE_END_BYTE);

    match port.write_all(&out_message_buf) {
        Ok(_) => {
            return Ok(());
        }
        Err(error) => {
            return handle_error(Error::IO(error));
        }
    }
}

fn main() {
    loop {
        match get_port() {
            Some(mut port) => {
                let mut is_communication_begin = true;
                match port.write_data_terminal_ready(true) {
                    Err(error) => {
                        println!("Error activating port: {}", error);
                        std::thread::sleep(Duration::from_secs(1));
                    }
                    Ok(_) => loop {
                        match read_message(&mut port, &mut is_communication_begin) {
                            Ok(message) => {
                                println!("InMessage: {}", message);
                                let res = handle_message(&message).and_then(|out_message| {
                                    return Some(write_message(&mut port, out_message));
                                });

                                if res.is_some_and(|res| res.is_err()) {
                                    // unrecoverable error - stop using port
                                    break;
                                }
                            }
                            Err(error) => {
                                if handle_error(error).is_err() {
                                    // unrecoverable error - stop using port
                                    break;
                                }
                            }
                        }
                    },
                }
            }
            None => {
                println!("Waiting for port...");
                std::thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
