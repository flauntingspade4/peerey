#![warn(clippy::pedantic, clippy::nursery)]

use std::sync::Arc;

mod message;
use message::{ReceiveMessage, SendMessage};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    runtime::Runtime,
};

use druid::{
    widget::{Button, Flex, Label, Scroll, TextBox},
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Handled, Lens, Selector, SingleUse,
    Target, Widget, WidgetExt, WindowDesc,
};

use crossbeam::sync::ShardedLock;

//const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const TEXT_BOX_WIDTH: f64 = 400.;
const WINDOW_TITLE: &str = "peerey - client";

const RECIEVE_MESSAGE: Selector<SingleUse<ReceiveMessage>> = Selector::new("recieved_message");

const IP: &str = include_str!("../.IP");

#[derive(Clone, Data, Lens)]
struct State {
    name: String,
    messages: String,
    current_message: String,
    stream: Arc<ShardedLock<WriteHalf<TcpStream>>>,
    runtime: Arc<ShardedLock<Runtime>>,
}

impl State {
    fn send(&mut self) {
        let mut stream = self.stream.write().unwrap();
        let runtime = self.runtime.read().unwrap();

        let current_message = std::mem::replace(&mut self.current_message, String::new());

        let message = SendMessage::new(&self.name, current_message);

        runtime
            .block_on(stream.write_all(&bincode::serialize(&message).unwrap()))
            .unwrap();
    }
}

struct Delegate;

impl AppDelegate<State> for Delegate {
    fn command(
        &mut self,
        _: &mut DelegateCtx,
        _: Target,
        cmd: &Command,
        data: &mut State,
        _: &Env,
    ) -> Handled {
        cmd.get(RECIEVE_MESSAGE).map_or(Handled::No, |cmd| {
            let message = cmd.take().unwrap();

            data.messages.push_str(&message.to_string());

            Handled::Yes
        })
    }
}

fn main() {
    let runtime = Runtime::new().unwrap();

    let mut name = String::new();
    while let Err(e) = std::io::stdin().read_line(&mut name) {
        println!("{}", e);
    }

    name = name.trim().to_string();

    let window = WindowDesc::new(build_root_widget)
        .title(WINDOW_TITLE)
        .window_size((400.0, 400.0));

    let launcher = AppLauncher::with_window(window);

    let sink = launcher.get_external_handle();

    let stream = runtime.block_on(TcpStream::connect(IP)).unwrap();

    let (r, w) = tokio::io::split(stream);

    runtime.spawn(read_stream(sink, r));

    let data = State {
        name,
        messages: String::new(),
        current_message: String::new(),
        stream: Arc::new(ShardedLock::new(w)),
        runtime: Arc::new(ShardedLock::new(runtime)),
    };

    eprintln!("Starting gui");

    launcher.delegate(Delegate).launch(data).unwrap();
}

fn build_root_widget() -> impl Widget<State> {
    let textbox = TextBox::multiline()
        .with_placeholder("Type a message here!")
        .fix_width(TEXT_BOX_WIDTH)
        .lens(State::current_message);

    let label = Label::new(|data: &State, _: &Env| data.messages.to_string());

    let scroll = Scroll::new(label).vertical();

    let button = Button::new("Send").on_click(|_ctx, data: &mut State, _env| {
        data.send();
    });

    let layout = Flex::column()
        .with_child(scroll)
        .with_child(textbox)
        .with_child(button);

    Scroll::new(layout).vertical()
}

async fn read_stream(sink: druid::ExtEventSink, mut r: ReadHalf<TcpStream>) {
    let mut buf = Vec::new();
    loop {
        r.read_buf(&mut buf).await.unwrap();

        let new_buf = std::mem::replace(&mut buf, Vec::new());

        match bincode::deserialize(&new_buf) {
            Ok(data) => {
                if let Err(e) =
                    sink.submit_command(RECIEVE_MESSAGE, SingleUse::new(data), Target::Auto)
                {
                    println!("{}", e);
                    break;
                }
            }
            Err(e) => {
                println!("{} while parsing {:?}", e, new_buf)
            }
        }
    }
}
