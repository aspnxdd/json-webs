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
pub const GET_REQ_LINE: &str = "GET / HTTP/1.1";

pub enum StatusLine {
    Ok,
    NotFound,
    InternalServerError,
}

impl StatusLine {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatusLine::Ok => "HTTP/1.1 200 OK",
            StatusLine::NotFound => "HTTP/1.1 404 NOT FOUND",
            StatusLine::InternalServerError => "HTTP/1.1 500 INTERNAL SERVER ERROR",
        }
    }

    pub fn contents_as_str(&self) -> &'static str {
        match self {
            StatusLine::NotFound => "Err 404: Not Found",
            StatusLine::InternalServerError => "Err 500: Internal Server Error",
            _ => "",
        }
    }
}

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
    let listener = TcpListener::bind(format!("{}:{}", LOCALHOST, args.port));
    match listener {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let res = handle_connection(stream, &args.file_name);
                        match res {
                            Ok(_) => {}
                            Err(e) => {
                                eprintln!("Error handling connection: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Connection failed: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to bind to port: {}", e);
        }
    }
}

fn get_now_as_rfc3339() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    now.to_rfc3339()
}

fn handle_connection(mut stream: TcpStream, file_name: &String) -> std::io::Result<()> {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = buf_reader.lines().next();
    let request_line = match request_line {
        Some(line) => match line {
            Ok(line) => line,
            Err(_) => {
                eprintln!("Error reading request line");
                return Ok(());
            }
        },
        None => {
            eprintln!("No request line found");
            return Ok(());
        }
    };

    if request_line == GET_REQ_LINE {
        let contents = fs::read_to_string(file_name);
        let contents = match contents {
            Ok(contents) => contents,
            Err(_) => {
                let status_line = StatusLine::InternalServerError.as_str();
                let contents = StatusLine::InternalServerError.contents_as_str();
                let length = contents.len();

                let response =
                    format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

                eprintln!("Error reading file: {}", file_name);
                stream.write_all(response.as_bytes())?;
                return Ok(());
            }
        };
        let status_line = StatusLine::Ok.as_str();
        let length = contents.len();
        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
        println!("JSON SERVED AT: {}", get_now_as_rfc3339());
        stream.write_all(response.as_bytes())?;
    } else {
        let status_line = StatusLine::NotFound.as_str();
        let contents = StatusLine::NotFound.contents_as_str();
        let length = contents.len();

        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

        stream.write_all(response.as_bytes())?;
    }
    Ok(())
}
