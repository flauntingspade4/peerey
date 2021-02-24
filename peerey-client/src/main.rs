#![warn(clippy::pedantic, clippy::nursery)]

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

mod message;

use message::PeereyMessage;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, ReadHalf, WriteHalf},
    net::TcpStream,
    runtime::Runtime,
};

use druid::{
    widget::{Align, Button, Flex, Label, LineBreaking, Scroll, TextBox},
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, Handled, Lens, Selector, SingleUse,
    Target, Widget, WidgetExt, WindowDesc,
};

use crossbeam::sync::ShardedLock;

const TEXT_BOX_WIDTH: f64 = 400.;
const WINDOW_TITLE: &str = "peerey - client";

const RECIEVE_MESSAGE: Selector<SingleUse<PeereyMessage>> = Selector::new("recieved_message");

const IP: &str = include_str!("../.IP");
const MAX_MESSAGE_SIZE: usize = include!("../../MAX_MESSAGE_SIZE");

#[derive(Clone, Data, Lens)]
struct State {
    name: String,
    messages: String,
    current_message: String,
    last_sent: Arc<Instant>,
    stream: Arc<ShardedLock<WriteHalf<TcpStream>>>,
    runtime: Arc<ShardedLock<Runtime>>,
}

impl State {
    fn send(&mut self) {
        if self.current_message.len() > MAX_MESSAGE_SIZE {
            eprintln!(
                "ERROR Message is too large, must be {} bytes at most",
                MAX_MESSAGE_SIZE
            );
        } else if self.last_sent.elapsed().as_secs() < 5 {
            eprintln!(
                "ERROR Messages cannot be sent more than once every {} seconds",
                5
            );
        } else {
            let mut stream = self.stream.write().unwrap();
            let mut runtime = self.runtime.write().unwrap();

            let mut current_message = std::mem::replace(&mut self.current_message, String::new());

            let mut message = Vec::with_capacity(current_message.len() + self.name.len() + 2);

            message.extend(self.name.as_bytes());
            message.push(0);

            // Use .append() instead of .extend() so
            // 'current_message' doesn't have to be cloned,
            // instead just emptied
            message.append(unsafe { current_message.as_mut_vec() });
            message.push(0);

            self.last_sent = Arc::new(Instant::now());

            runtime.block_on(stream.write_all(&message)).unwrap();
        }
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
    let mut runtime = tokio::runtime::Builder::new()
        .enable_io()
        .threaded_scheduler()
        .build()
        .unwrap();

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
        last_sent: Arc::new(Instant::now() - Duration::from_secs(6)),
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

    let label = Label::new(|data: &State, _: &Env| data.messages.to_string())
        .with_line_break_mode(LineBreaking::WordWrap);

    let label = Align::left(label);

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

async fn read_stream(sink: druid::ExtEventSink, r: ReadHalf<TcpStream>) {
    let initial_len = 25;
    let mut r = BufReader::new(r);
    let mut buf: Vec<u8> = vec![0; initial_len];

    loop {
        let mut src = [String::new(), String::new()];
        for i in 0..=1usize {
            //r.read_until(0, &mut buf).await.unwrap();

            let read = r.read_to_end(&mut buf).await.unwrap();

            println!("Read {} bytes", read);

            src[i] = String::from_utf8(std::mem::replace(&mut buf, vec![0; initial_len])).unwrap();
        }
        let data = PeereyMessage::from(src);

        if let Err(e) = sink.submit_command(RECIEVE_MESSAGE, SingleUse::new(data), Target::Auto) {
            eprintln!("ERROR {}", e);
            break;
        }
    }
}
