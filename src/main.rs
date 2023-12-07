use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

use load_balancer::{Routes, ThreadPool};
use reqwest::Response;

// TODO: command line args for periodic health check interval
// TODO: command line args for ports
fn main() {
    let pool = ThreadPool::new(4);
    let listener: TcpListener =
        TcpListener::bind("127.0.0.1:8080").expect("Failed to bind, port already in use ");

    // TODO: leverage a builder pattern
    let routes = Arc::new(Mutex::new(Routes::new(3)));

    // Scope created to ensure lock acquired during this scope is relased after initialization is
    // complete
    // Known as "lock scope" or "scoped locking"
    {
        // Acquires lock
        // MutexGuard is a lock on Mutex and implements "Drop" trait
        let mut routes: MutexGuard<'_, Routes> = routes.lock().unwrap();
        routes.add_server("http://127.0.0.1:8082").unwrap();
        routes.add_server("http://127.0.0.1:8081").unwrap();
        routes.add_server("http://127.0.0.1:8083").unwrap();
    } // Lock is automaically relased when "routes" goes out of scope

    let server_status = Arc::clone(&routes);
    thread::spawn(move || ping_server("http://127.0.0.1:8083", 5, server_status));

    for stream in listener.incoming() {
        let arc_routes: Arc<Mutex<Routes>> = Arc::clone(&routes);
        pool.execute(move || match stream {
            Ok(stream) => {
                let server: String;
                // Lock scope
                {
                    // Acquire lock on routes
                    let mut routes: MutexGuard<'_, Routes> = arc_routes.lock().unwrap();

                    // Handle this error better
                    server = routes.get_running_server().unwrap();
                }
                println!("Current route is :{:?}", server);
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

    // println!("Request: {:?}", http_request);

    let body: String = reqwest::blocking::get(route)?.text()?;
    // println!("{:?}", body);

    let response: String = String::from("HTTP/1.1 200 OK\r\n\r\n") + body.as_str();

    stream.write_all(response.as_bytes()).unwrap();

    Ok(())
}

fn ping_server(server: &str, interval: u64, routes: Arc<Mutex<Routes>>) {
    loop {
        let body = reqwest::blocking::get(server);
        println!("{:?}", body);
        match reqwest::blocking::get(server) {
            Ok(_) => {
                println!("Successful ping! {} is healthy", server);
                let routes: Arc<Mutex<Routes>> = Arc::clone(&routes);

                let mut arc_routes: MutexGuard<'_, Routes> = routes.lock().unwrap();

                if !arc_routes.is_current_server_running() {
                    arc_routes.enable_server(server).unwrap();
                }
            }
            Err(_) => {
                let routes: Arc<Mutex<Routes>> = Arc::clone(&routes);
                let mut arc_routes: MutexGuard<'_, Routes> = routes.lock().unwrap();
                arc_routes.disable_server(server).unwrap();
            }
        }
        thread::sleep(Duration::from_secs(interval));
    }
}
