#![warn(clippy::pedantic, clippy::nursery)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

mod chat_server;
mod codec;
use chat_server::{ChatServer, ChatSession, Joined};
use codec::Codec;

use actix::{io::FramedWrite, Actor, Addr, AsyncContext, Context, Handler, Message, System};
use actix_rt::net::{TcpListener, TcpStream};
use tokio::stream::StreamExt;
use tokio_util::codec::FramedRead;

static PORTS: &[u16] = &[45623];

const MAX_MESSAGE_SIZE: usize = 512;

#[derive(Message)]
#[rtype(result = "()")]
struct Incoming(TcpStream);

struct Server {
    chat: Addr<ChatServer>,
}

impl Actor for Server {
    type Context = Context<Self>;
}

impl Handler<Incoming> for Server {
    type Result = ();

    fn handle(&mut self, msg: Incoming, _: &mut Context<Self>) {
        let addr = ChatSession::create(|ctx| {
            let (r, w) = tokio::io::split(msg.0);
            ctx.add_stream(FramedRead::new(r, Codec));
            ChatSession::new(self.chat.clone(), FramedWrite::new(w, Codec, ctx))
        });
        self.chat.do_send(Joined(addr));
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let mut listener = None;
    for port in PORTS.iter() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), *port);
        match TcpListener::bind(&addr).await {
            Ok(t) => {
                eprintln!("INFO Began running successfully on port {}", port);
                listener = Some(Box::new(t));
                break;
            }
            Err(e) => {
                eprintln!("ERROR Error {} starting on port {}", e, port);
            }
        };
    }
    let listener = listener.expect("ERROR No port could be started on");

    Server::create(|ctx| {
        ctx.add_message_stream(
            Box::leak(listener)
                .incoming()
                .map(|stream| Incoming(stream.unwrap())),
        );
        Server {
            chat: ChatServer::default().start(),
        }
    });

    tokio::signal::ctrl_c().await.unwrap();
    System::current().stop();
    println!("INFO Ctrl-C received, shutting down");

    Ok(())
}
