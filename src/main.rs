use chrono::{DateTime, Utc};
use clap::Parser;
use notify::{Event, RecursiveMode, Result, Watcher};
use std::{
    fs,
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::{Arc, Mutex},
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

    pub fn contents_as_str(&self) -> Option<&str> {
        match self {
            StatusLine::NotFound => Some("Err 404: Not Found"),
            StatusLine::InternalServerError => Some("Err 500: Internal Server Error"),
            _ => None,
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    file_path: String,

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
        if fs::metadata(&self.file_path).is_err() {
            panic!("{}", format!("Json file [{}] Not Found", self.file_path));
        }
    }
}

fn main() -> Result<()> {
    let args = Args::safe_parse();
    let file_path = args.file_path;
    let contents = fs::read_to_string(&file_path);
    let contents = match contents {
        Ok(contents) => contents,
        Err(_) => panic!("Err reading file"),
    };

    let data = Arc::new(Mutex::new(contents));

    let mut watcher = notify::recommended_watcher({
        let data = data.clone();
        move |res: Result<Event>| match res {
            Ok(res) => {
                let file_path = res.paths[0].clone();
                match fs::read_to_string(&file_path) {
                    Ok(contents) => {
                        *data.lock().unwrap() = contents;
                    }
                    Err(_) => panic!("Err reading file"),
                };
            }
            Err(e) => println!("Watch Error: {:?}", e),
        }
    })?;

    watcher.watch(Path::new(&file_path), RecursiveMode::Recursive)?;

    let listener = TcpListener::bind(format!("{}:{}", LOCALHOST, args.port));

    match listener {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let data = data.clone();
                        let res = handle_connection(stream, data);
                        if let Err(e) = res {
                            eprintln!("Error handling connection: {}", e);
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
    Ok(())
}

fn get_now_as_rfc3339() -> String {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    now.to_rfc3339()
}

fn handle_connection(mut stream: TcpStream, contents: Arc<Mutex<String>>) -> std::io::Result<()> {
    let buf_reader = BufReader::new(&mut stream);
    let request_line = match buf_reader.lines().next() {
        Some(Ok(line)) => line,
        Some(Err(_)) => {
            eprintln!("Error reading request line");
            return Ok(());
        }
        None => {
            eprintln!("No request line found");
            return Ok(());
        }
    };

    if request_line == GET_REQ_LINE {
        let Ok(contents) = contents.lock() else {
            let status_line = StatusLine::InternalServerError.as_str();
            let contents = StatusLine::InternalServerError
                .contents_as_str()
                .expect("This is an OK response");
            let length = contents.len();

            let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

            eprintln!("Error reading file");
            stream.write_all(response.as_bytes())?;
            return Ok(());
        };
        let status_line = StatusLine::Ok.as_str();
        let length = contents.len();
        let response = format!("{status_line}\r\nContent-Type: application/json\r\nContent-Length: {length}\r\n\r\n{contents}");

        stream.write_all(response.as_bytes())?;
        println!("JSON SERVED AT: {}", get_now_as_rfc3339());
    } else {
        let status_line = StatusLine::NotFound.as_str();
        let contents = StatusLine::NotFound
            .contents_as_str()
            .expect("This is an OK response");
        let length = contents.len();

        let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
        stream.write_all(response.as_bytes())?;
        eprintln!("Invalid request line: {}", request_line);
    }
    Ok(())
}
