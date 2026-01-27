use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tonic::{transport::Server, Request, Response, Status};

// Include generated code
pub mod debug_control {
    tonic::include_proto!("debug_control");
}

pub use debug_control::debug_control_client::DebugControlClient;
pub use debug_control::debug_control_server::{DebugControl, DebugControlServer};
pub use debug_control::{Ack, InputEventMsg, KeyMsg, MouseMsg, QuitMsg};

use pal::UIEvent;

pub struct RpcServer {}

impl RpcServer {
    /// Start the RPC server on a background thread
    /// Returns the port bound to
    pub fn start(port: u16, event_sender: Sender<UIEvent>) -> Result<u16> {
        let addr: SocketAddr = format!("127.0.0.1:{}", port)
            .parse()
            .context("Invalid address")?;

        // Create the service implementation
        let service = DebugControlImpl {
            sender: Arc::new(Mutex::new(event_sender)),
        };

        // We need to know the bound port.
        // Tonic doesn't easily return the bound port if we pass 0.
        // So we bind a TCP listener manually first.
        let listener = std::net::TcpListener::bind(addr).context("Failed to bind TCP listener")?;
        let local_addr = listener
            .local_addr()
            .context("Failed to get local address")?;
        let bound_port = local_addr.port();
        listener
            .set_nonblocking(true)
            .context("Failed to set nonblocking")?;

        println!("RPC Server listening on {}", local_addr);

        // Spawn a thread to run the server
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime");

            rt.block_on(async {
                Server::builder()
                    .add_service(DebugControlServer::new(service))
                    .serve_with_incoming(tokio_stream::wrappers::TcpListenerStream::new(
                        tokio::net::TcpListener::from_std(listener)
                            .expect("Failed to convert listener"),
                    ))
                    .await
                    .expect("RPC Server failed");
            });
        });

        Ok(bound_port)
    }
}

pub struct DebugControlImpl {
    sender: Arc<Mutex<Sender<UIEvent>>>,
}

#[tonic::async_trait]
impl DebugControl for DebugControlImpl {
    async fn send_input_event(
        &self,
        request: Request<InputEventMsg>,
    ) -> Result<Response<Ack>, Status> {
        let msg = request.into_inner();
        let event = match msg.event {
            Some(debug_control::input_event_msg::Event::Mouse(mouse)) => map_mouse_event(mouse),
            Some(debug_control::input_event_msg::Event::Key(key)) => map_key_event(key),
            Some(debug_control::input_event_msg::Event::Quit(_)) => UIEvent::Quit,
            None => return Err(Status::invalid_argument("No event provided")),
        };

        // Send to channel
        // Explicitly lock and handle the mutex guard
        let guard = self
            .sender
            .lock()
            .map_err(|_| Status::internal("Failed to lock sender"))?;
        match guard.send(event) {
            Ok(_) => Ok(Response::new(Ack { success: true })),
            Err(_) => Err(Status::internal(
                "Failed to send event to input queue (channel closed)",
            )),
        }
    }
}

fn map_mouse_event(msg: MouseMsg) -> UIEvent {
    match msg.r#type {
        0 => UIEvent::MouseDown {
            // DOWN
            x: msg.x as f64,
            y: msg.y as f64,
            button: msg.button,
        },
        1 => UIEvent::MouseUp {
            // UP
            x: msg.x as f64,
            y: msg.y as f64,
            button: msg.button,
        },
        2 => UIEvent::MouseMove {
            // MOVE
            x: msg.x as f64,
            y: msg.y as f64,
        },
        _ => UIEvent::MouseMove { x: 0.0, y: 0.0 }, // Fallback
    }
}

fn map_key_event(msg: KeyMsg) -> UIEvent {
    match msg.r#type {
        0 => UIEvent::KeyDown {
            keycode: msg.keycode,
        }, // DOWN
        1 => UIEvent::KeyUp {
            keycode: msg.keycode,
        }, // UP
        _ => UIEvent::KeyDown { keycode: 0 },
    }
}
