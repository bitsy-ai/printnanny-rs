//! `ClientSession` is an actor, it manages peer tcp connection and
//! proxies commands from peer to `QcMessageServer`.

use std::{
    io, net,
    str::FromStr,
    time::{Duration, Instant},
};

use actix::{prelude::*, spawn};
use log::{error, info, warn};
use tokio::{
    io::{split, WriteHalf},
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::FramedRead;

use crate::{
    codec::{QcMessageCodec, QcMessageRequest, QcMessageResponse},
    server::{self, QcMessageServer},
};

/// QcMessage server sends this messages to session
#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub String);

/// `QcMessageSession` actor is responsible for tcp peer communications.
pub struct QcMessageSession {
    /// unique session id
    id: usize,
    /// this is address of chat server
    addr: Addr<QcMessageServer>,
    /// Client must send ping at least once per 10 seconds, otherwise we drop
    /// connection.
    hb: Instant,
    /// Framed wrapper
    framed: actix::io::FramedWrite<QcMessageResponse, WriteHalf<TcpStream>, QcMessageCodec>,
    heartbeat_enabled: bool,
}

impl Actor for QcMessageSession {
    /// For tcp communication we are going to use `FramedContext`.
    /// It is convenient wrapper around `Framed` object from `tokio_io`
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        info!(
            "Started QcMessageSession {:?} with context: {:?}",
            self.addr, ctx
        );
        if self.heartbeat_enabled {
            self.hb(ctx);
        } else {
            warn!("Started QcMessageSession with heartbeat_disabled")
        }

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        let addr = ctx.address();
        self.addr
            .send(server::Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with server
                    Err(e) => {
                        error!("QCMessageSession error, stopping connection {:?}", e);
                        ctx.stop()
                    }
                }
                actix::fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.addr.do_send(server::Disconnect { id: self.id });
        Running::Stop
    }
}

impl actix::io::WriteHandler<io::Error> for QcMessageSession {}

/// To use `Framed` we have to define Io type and Codec
impl StreamHandler<Result<QcMessageRequest, io::Error>> for QcMessageSession {
    /// This is main event loop for client requests
    fn handle(&mut self, msg: Result<QcMessageRequest, io::Error>, ctx: &mut Context<Self>) {
        match msg {
            Ok(QcMessageRequest::JsonMessage(message)) => {
                // send message to chat server
                println!("Peer message: {message:?}");
                self.addr.do_send(server::JsonMessage {
                    id: self.id,
                    json: String::from_utf8(message).expect("Failed to deserialize message"),
                })
            }
            // we update heartbeat time on ping from peer
            Ok(QcMessageRequest::Ping) => self.hb = Instant::now(),
            _ => {
                error!("Received unknown message, closing session {:?}", msg);
                ctx.stop()
            }
        }
    }
}

/// Handler for Message, chat server sends this message, we just send string to
/// peer
impl Handler<Message> for QcMessageSession {
    type Result = ();

    fn handle(&mut self, msg: Message, _: &mut Context<Self>) {
        // send message to peer
        self.framed
            .write(QcMessageResponse::JsonMessage(msg.0.as_bytes().to_vec()));
    }
}

/// Helper methods
impl QcMessageSession {
    pub fn new(
        addr: Addr<QcMessageServer>,
        framed: actix::io::FramedWrite<QcMessageResponse, WriteHalf<TcpStream>, QcMessageCodec>,
    ) -> QcMessageSession {
        QcMessageSession {
            id: 0,
            addr,
            hb: Instant::now(),
            framed,
            heartbeat_enabled: true,
        }
    }

    // quick hack: gstreamer tcpclientsink element does not recv msgs (only sends), so the element can't respond to heartbeat messages
    // a better long-term solution is to implement a new element derived from tcpclientsink, which can send/recv ping/pong in addition to sending pipeline buffer
    pub fn with_heartbeat_disabled(mut self) -> Self {
        self.heartbeat_enabled = false;
        self
    }

    /// helper method that sends ping to client every second.
    ///
    /// also this method check heartbeats from client
    fn hb(&self, ctx: &mut Context<Self>) {
        ctx.run_interval(Duration::new(1, 0), |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > Duration::new(10, 0) {
                // heartbeat timed out
                error!("Client heartbeat failed, disconnecting!");

                // notify chat server
                act.addr.do_send(server::Disconnect { id: act.id });

                // stop actor
                ctx.stop();
            }

            act.framed.write(QcMessageResponse::Ping);
            // if we can not send message to sink, sink is closed (disconnected)
        });
    }
}

/// Define TCP server that will accept incoming TCP connection and create
/// chat actors.
pub fn tcp_server(_s: &str, server: Addr<QcMessageServer>) {
    // Create server listener
    let addr = net::SocketAddr::from_str("127.0.0.1:12345").unwrap();

    spawn(async move {
        let listener = TcpListener::bind(&addr).await.unwrap();

        while let Ok((stream, _)) = listener.accept().await {
            let server = server.clone();
            QcMessageSession::create(|ctx| {
                let (r, w) = split(stream);
                QcMessageSession::add_stream(FramedRead::new(r, QcMessageCodec), ctx);
                QcMessageSession::new(server, actix::io::FramedWrite::new(w, QcMessageCodec, ctx))
                    .with_heartbeat_disabled()
            });
        }
    });
}
