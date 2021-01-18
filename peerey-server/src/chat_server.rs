use std::time::Instant;

use crate::{
    codec::{ChatMessage, Codec},
    MAX_MESSAGE_SIZE,
};

use actix::{
    io::FramedWrite, Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message,
    StreamHandler,
};

use tokio::{io::WriteHalf, net::TcpStream};

pub struct Joined(pub Addr<ChatSession>);

impl Message for Joined {
    type Result = ();
}

impl Handler<Joined> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Joined, _: &mut Self::Context) -> Self::Result {
        self.sessions.push(msg.0);
    }
}

pub struct Disconnect(pub Addr<ChatSession>);

impl Message for Disconnect {
    type Result = ();
}

impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) -> Self::Result {
        if let Some(index) = self.sessions.iter().position(|a| a == &msg.0) {
            self.sessions.swap_remove(index);
        } else {
            eprintln!("ERROR: Not found index to remove in the server");
        }
    }
}

#[derive(Default)]
pub struct ChatServer {
    sessions: Vec<Addr<ChatSession>>,
}

impl Actor for ChatServer {
    type Context = Context<Self>;
}

impl Handler<ChatMessage> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: ChatMessage, _: &mut Self::Context) -> Self::Result {
        for id in self.sessions.iter() {
            id.do_send(msg.clone());
        }
    }
}

pub struct ChatSession {
    addr: Addr<ChatServer>,
    // The wrapper around that which actually sends ChatMessages
    framed: FramedWrite<ChatMessage, WriteHalf<TcpStream>, Codec>,
    last_recieved: Instant,
}

impl ChatSession {
    pub fn new(
        addr: Addr<ChatServer>,
        framed: FramedWrite<ChatMessage, WriteHalf<TcpStream>, Codec>,
    ) -> Self {
        let last_recieved = Instant::now();
        Self { addr, framed, last_recieved }
    }
}

impl Actor for ChatSession {
    type Context = Context<Self>;

    fn stopping(&mut self, ctx: &mut Self::Context) -> actix::Running {
        self.addr.do_send(Disconnect(ctx.address()));
        actix::Running::Stop
    }
}

impl Handler<ChatMessage> for ChatSession {
    type Result = <ChatMessage as Message>::Result;

    fn handle(&mut self, msg: ChatMessage, _: &mut Self::Context) -> Self::Result {
        self.framed.write(msg);
    }
}

impl actix::io::WriteHandler<std::io::Error> for ChatSession {}

impl StreamHandler<Result<ChatMessage, std::io::Error>> for ChatSession {
    fn handle(&mut self, msg: Result<ChatMessage, std::io::Error>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                let instant = Instant::now();
                if msg.0.len() <= MAX_MESSAGE_SIZE && (instant - self.last_recieved).as_secs() > 5 {
                    self.last_recieved = instant;
                    self.addr.do_send(msg)
                }
            }
            Err(e) => {
                eprintln!("ERROR: {}", e);
                ctx.stop();
            }
        }
    }
}
