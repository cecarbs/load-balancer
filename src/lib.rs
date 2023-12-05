// use core::fmt::Display;
use std::{
    collections::HashMap,
    fmt,
    sync::{mpsc, Arc, Mutex},
    thread, usize,
};

// Since thread::spawn immediately expects code to run this will handle the awaition of code to be
// executed
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
// FnOnce() enforces that this can only be called once, in our case, the job can only be run once
type Job = Box<dyn FnOnce() + Send + 'static>;

pub enum PoolCreationError {
    SizeTooLarge,
}

impl fmt::Display for PoolCreationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoolCreationError::SizeTooLarge => write!(f, "The maximum number of threads is 4!"),
        }
    }
}

impl ThreadPool {
    // Create a new ThreadPool
    //
    // The size is the number of threads in the pool.
    //
    // # panic!
    //
    // The 'new' function will panic if the size is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        // Arc allows multiple workers own the receiver and Mutex ensures only one workers gets a
        // job from the receiver at a time
        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        // For each new worker, we clone the Arc to bump the reference count so the workers can
        // share ownership of the receiver
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn build(size: usize) -> Result<ThreadPool, PoolCreationError> {
        if size > 4 {
            return Err(PoolCreationError::SizeTooLarge);
        }

        let (sender, receiver) = mpsc::channel();

        let mut workers = Vec::with_capacity(4);

        Ok(ThreadPool {
            workers,
            sender: Some(sender),
        })
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

// Picks up code that  needs to be run and runs the code in the Worker's thread
struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            // It's okay to use unwrap() since we know failure case won't happen but compiler
            // doesn't
            let job = receiver.lock().unwrap().recv().unwrap();

            println!("Worker {id} got a job; executing.");

            job();
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

#[derive(Debug)]
enum ServerStatus {
    Running,
    Stopped,
    Offline,
}

#[derive(Debug)]
pub struct Routes {
    capacity: usize,
    servers: Vec<String>,
    read_index: usize,
    status: HashMap<String, ServerStatus>,
}
// TODO: use a tuple inside of the vector instead of a string (String, bool)
impl Routes {
    pub fn new(capacity: usize) -> Routes {
        Routes {
            capacity,
            servers: Vec::with_capacity(capacity),
            read_index: 0,
            status: HashMap::with_capacity(capacity),
        }
    }

    fn is_current_server_running(&self) -> bool {
        let server: &str = &self.servers[self.read_index];
        matches!(self.status.get(server).unwrap(), ServerStatus::Running)
    }

    fn cycle_and_find_running_server(&mut self) -> bool {
        let mut count = 0;
        while count <= self.capacity {
            let server: &str = &self.servers[self.read_index];
            if let Some(entry) = self.status.get(server) {
                match *entry {
                    ServerStatus::Running => return true,
                    _ => {
                        self.read_index = (self.read_index + 1) % self.capacity;
                        count += 1;
                    }
                };
            }
        }
        eprintln!("There are no servers running!");
        false
    }

    pub fn add_server(&mut self, route: &str) -> Result<(), &'static str> {
        if self.servers.len() == self.capacity {
            Err("Not enough capacity!")
        } else {
            self.status.insert(route.to_string(), ServerStatus::Running);
            self.servers.push(route.to_string());
            Ok(())
        }
    }

    // TODO: refactor to use a result
    pub fn get_running_server(&mut self) -> &str {
        if self.is_current_server_running() {
            let server: &str = &self.servers[self.read_index];
            self.read_index = (self.read_index + 1) % self.capacity;
            server
        } else {
            self.cycle_and_find_running_server();
        }
    }

    pub fn disable_server(&mut self, server: &str) {
        if let Some(entry) = self.status.get_mut(server) {
            *entry = ServerStatus::Offline;
        } else {
            println!("Server {} does not exist!", server);
        }
    }
}

// #[derive(Debug, Clone)]
pub struct RingBuffer<T: Clone> {
    buffer: Vec<Option<Box<T>>>,
    capacity: usize,
    // read index
    front: usize,
    // write index
    rear: usize,
}

impl<T: Clone> RingBuffer<T> {
    pub fn new(capacity: usize) -> Self {
        let buffer = vec![None; capacity];
        RingBuffer {
            buffer,
            capacity,
            front: 0,
            rear: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.front == self.rear && self.buffer[self.front].is_none()
    }

    pub fn is_full(&self) -> bool {
        self.front == self.rear && self.buffer[self.front].is_some()
    }

    pub fn write(&mut self, item: T) -> Result<(), &'static str> {
        // checks to see if the buffer is full first before adding
        if self.is_full() {
            Err("Buffer is full")
        } else {
            // adds new item where the rear is pointing
            self.buffer[self.rear] = Some(Box::new(item));
            // write index
            self.rear = (self.rear + 1) % self.capacity;
            Ok(())
        }
    }

    // the &'static str is often used for error messages that are known at compile time and do not
    // depend on the lifetime of the specific data. For example, error messages like "Buffer is
    // empty" are constant strings that are always available and do not dpened on the specific data
    // in the buffer
    pub fn read(&mut self) -> Result<T, &'static str> {
        if self.is_empty() {
            return Err("Buffer is empty");
        }
        // takes the value from the read index and replaces it with None
        let item = self.buffer[self.front].take().unwrap();
        // read index
        self.front = (self.front + 1) % self.capacity;

        Ok(*item)
    }
}
