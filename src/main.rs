use chrono::{DateTime, Utc};
use clap::Parser;
use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    time::SystemTime,
};

pub const DEFAULT_PORT: u128 = 7878;
pub const LOCALHOST: &str = "127.0.0.1";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file_name: String,

    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u128,
}

impl Args {
    fn safe_parse() -> Self {
        let args = Args::parse();
        args.assert_file_exists();
        args
    }
    fn assert_file_exists(&self) {
        if fs::metadata(&self.file_name).is_err() {
            panic!(
                "{}",
                format!("Json file [{}] Not Found", self.file_name.to_string())
            );
        }
    }
}

fn main() {
    let args = Args::safe_parse();
    let listener = TcpListener::bind(format!("{}:{}", LOCALHOST, args.port)).unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_connection(stream, &args.file_name);
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}

fn get_now_as_rfc3339() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    now.to_rfc3339()
}

fn handle_connection(mut stream: TcpStream, file_name: &String) {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    if request_line == "GET / HTTP/1.1" {
        let status_line = "HTTP/1.1 200 OK";
        let contents = fs::read_to_string(file_name).unwrap_or("Err 404: Not Found".to_string());
        let length = contents.len();

        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

        stream.write_all(response.as_bytes()).unwrap();
        println!("JSON SERVED AT: {}", get_now_as_rfc3339());
    } else {
        let status_line = "HTTP/1.1 404 NOT FOUND";
        let contents = "Err 404: Not Found";
        let length = contents.len();

        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

        stream.write_all(response.as_bytes()).unwrap();
    }
}
