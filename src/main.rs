use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, MutexGuard},
};

use load_balancer::{RingBuffer, Routes, ThreadPool};

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

// #[tokio::main]
// async fn main() {
//     let pool = ThreadPool::new(4);
//     let listener: TcpListener =
//         TcpListener::bind("127.0.0.1:8080").expect("Failed to bind, port already in use ");
//
//     let mut routes = Routes::new(2);
//     if routes.add_server("http://127.0.0.1:8081").is_ok() {
//         println!("Successfully added server!");
//     } else {
//         println!("Something went wrong!");
//     }
//
//     for stream in listener.incoming() {
//         pool.execute(move || match stream {
//             Ok(stream) => {
//                 // handle_connection(stream);
//                 async_get(stream, "http://127.0.0.1:8082");
//             }
//             Err(e) => {
//                 println!("Connection failed with error: {}", e);
//             }
//         })
//     }
// }

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

    let mut count = 0;
    for stream in listener.incoming() {
        // count += 1;
        // println!("The current count is: {:?}", count);
        let arc_routes = Arc::clone(&routes);
        pool.execute(move || match stream {
            // Declare a variable to hold the server path
            Ok(stream) => {
                let server: String;
                // Lock scope
                {
                    // Acquire lock on routes
                    let mut routes = arc_routes.lock().unwrap();

                    server = routes.get_server().to_string();
                }
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

    let response_from_server = reqwest::blocking::get(route)?.text()?;

    let response = String::from("HTTP/1.1 200 OK\r\n\r\n") + response_from_server.as_str();

    stream.write_all(response.as_bytes()).unwrap();

    Ok(())
}

// async fn async_get(mut stream: TcpStream, route: &str) -> Result<(), Box<dyn std::error::Error>> {
//     let buf_reader = BufReader::new(&mut stream);
//
//     let http_request: Vec<_> = buf_reader
//         .lines()
//         .map(|result| result.unwrap())
//         .take_while(|line| !line.is_empty())
//         .collect();
//
//     println!("Request: {:?}", http_request);
//
//     let response_from_server = reqwest::get(route).await?.text().await?;
//
//     let response = String::from("HTTP/1.1 200 OK\r\n\r\n") + response_from_server.as_str();
//
//     stream.write_all(response.as_bytes()).unwrap();
//
//     Ok(())
// }
