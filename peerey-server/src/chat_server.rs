use std::{
    io::Error as IoError,
    time::{Duration, Instant},
};

use crate::{
    codec::{ChatMessage, Codec},
    MAX_MESSAGE_SIZE,
};

use actix::{
    io::FramedWrite, Actor, ActorContext, Addr, AsyncContext, Context, Handler, Message, Running,
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
        // Set 6 seconds in the past so a user can immediately mesage
        let last_recieved = Instant::now() - Duration::from_secs(6);
        Self {
            addr,
            framed,
            last_recieved,
        }
    }
}

impl Actor for ChatSession {
    type Context = Context<Self>;

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        self.addr.do_send(Disconnect(ctx.address()));
        Running::Stop
    }
}

impl Handler<ChatMessage> for ChatSession {
    type Result = ();

    fn handle(&mut self, msg: ChatMessage, _: &mut Self::Context) {
        self.framed.write(msg);
    }
}

impl actix::io::WriteHandler<IoError> for ChatSession {
    fn error(&mut self, e: IoError, _: &mut Self::Context) -> Running {
        println!("ERROR {}", e);
        Running::Stop
    }
}

impl StreamHandler<Result<ChatMessage, IoError>> for ChatSession {
    fn handle(&mut self, msg: Result<ChatMessage, IoError>, ctx: &mut Self::Context) {
        match msg {
            Ok(msg) => {
                let instant = Instant::now();
                if msg.0.len() <= MAX_MESSAGE_SIZE && (instant - self.last_recieved).as_secs() > 5 {
                    self.last_recieved = instant;
                    self.addr.do_send(msg);
                }
            }
            Err(e) => {
                eprintln!("ERROR {}", e);
                ctx.stop();
            }
        }
    }
}
