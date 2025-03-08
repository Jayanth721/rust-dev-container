use std::error::Error;
use std::io::{Read, Write};

use mio::net::{TcpListener, TcpStream};
use mio::{Events, Interest, Poll, Token};

// Some tokens to allow us to identify which event is for which socket.
const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

fn main() -> Result<(), Box<dyn Error>> {
    // Create a poll instance.
    let mut poll = Poll::new()?;
    // Create storage for events.
    let mut events = Events::with_capacity(128);

    // Setup the server socket.
    let addr = "127.0.0.1:13265".parse()?;
    let mut server = TcpListener::bind(addr)?;
    // Start listening for incoming connections.
    poll.registry()
        .register(&mut server, SERVER, Interest::READABLE)?;

    // Setup the client socket.
    let mut client = TcpStream::connect(addr)?;
    // Register the socket.
    poll.registry()
        .register(&mut client, CLIENT, Interest::READABLE | Interest::WRITABLE)?;

    // Start an event loop.
    loop {
        // Poll Mio for events, blocking until we get an event.
        poll.poll(&mut events, None)?;

        // Process each event.
        for event in events.iter() {
            // We can use the token we previously provided to `register` to
            // determine for which socket the event is.
            match event.token() {
                SERVER => {
                    // If this is an event for the server, it means a connection
                    // is ready to be accepted.
                    let (mut client_stream, addr) = server.accept()?;

                    // Let's say hello to our client
                    client_stream.write(format!("Hello from Server to client {:?}", addr).as_bytes()).unwrap();
                }
                CLIENT => {
                    if event.is_writable() {
                        // We can (likely) write to the socket without blocking.
                    }

                    if event.is_readable() {
                        // We can (likely) read from the socket without blocking.
                        let mut bytes = [0u8; 1024];

                        match client.read(&mut bytes) {
                            Ok(0) => {
                                println!("Server disconnected! Bye.");
                                return Ok(());
                            },
                            Ok(num_bytes) => {
                                println!("Server said: {:?}", std::str::from_utf8(&bytes[0..num_bytes]).expect("Failed to parse message from server!"));
                            },
                            Err(e) => {
                                println!("Error reading: {:?}", e);
                            }
                        }
                    }
                }
                // We don't expect any events with tokens other than those we provided.
                _ => unreachable!(),
            }
        }
    }
}
