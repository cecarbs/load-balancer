use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};

use load_balancer::ThreadPool;

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    // Reads and prints the request from the client
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    // Print out the request from the client
    println!("Request: {:?}", http_request);

    let response = "HTTP/1.1 200 OK\r\n\r\nReplied with a hello message";

    // TODO: error handle here
    stream.write_all(response.as_bytes()).unwrap();
}

fn main() {
    let pool = ThreadPool::new(4);
    let listener: TcpListener =
        TcpListener::bind("127.0.0.1:8080").expect("Failed to bind, port already in use ");

    for stream in listener.incoming() {
        pool.execute(|| match stream {
            Ok(stream) => {
                // handle_connection(stream);
                blocking_get(stream).unwrap();
            }
            Err(e) => {
                println!("Connection failed with error: {}", e);
            }
        })
    }
}

fn blocking_get(mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    let buf_reader = BufReader::new(&mut stream);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:?}", http_request);

    let response_from_server = reqwest::blocking::get("http://127.0.0.1:8081")?.text()?;

    let response = String::from("HTTP/1.1 200 OK\r\n\r\n") + response_from_server.as_str();

    stream.write_all(response.as_bytes()).unwrap();

    Ok(())
}
