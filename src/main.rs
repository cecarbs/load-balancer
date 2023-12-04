use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, MutexGuard},
};

use load_balancer::{Routes, ThreadPool};

fn main() {
    let pool = ThreadPool::new(4);
    let listener: TcpListener =
        TcpListener::bind("127.0.0.1:8080").expect("Failed to bind, port already in use ");

    let routes = Arc::new(Mutex::new(Routes::new(2)));

    // Scope created to ensure lock acquired during this scope is relased after initialization is
    // complete
    // Known as "lock scope" or "scoped locking"
    {
        // Acquires lock
        // MutexGuard is a lock on Mutex and implements "Drop" trait
        let mut routes: MutexGuard<'_, Routes> = routes.lock().unwrap();
        routes.add_server("http://127.0.0.1:8082").unwrap();
        routes.add_server("http://127.0.0.1.8081").unwrap();
    } // Lock is automaically relased when "routes" goes out of scope

    for stream in listener.incoming() {
        let arc_routes = Arc::clone(&routes);
        pool.execute(move || match stream {
            Ok(stream) => {
                let server: String;
                // Lock scope
                {
                    // Acquire lock on routes
                    let mut routes = arc_routes.lock().unwrap();

                    server = routes.get_server().to_string();
                }
                println!("Current route is :{:?}", server);
                // handle_connection(stream);
                blocking_get(stream, &server).unwrap();
            }
            Err(e) => {
                println!("Connection failed with error: {}", e);
            }
        })
    }
}

fn blocking_get(mut stream: TcpStream, route: &str) -> Result<(), Box<dyn std::error::Error>> {
    let buf_reader = BufReader::new(&mut stream);

    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {:?}", http_request);

    let body = reqwest::blocking::get(route)?.text()?;

    let response = String::from("HTTP/1.1 200 OK\r\n\r\n") + body.as_str();

    stream.write_all(response.as_bytes()).unwrap();

    Ok(())
}
