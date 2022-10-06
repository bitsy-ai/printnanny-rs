//! `QcMessageServer` is an actor. It maintains list of connection client session.
//! And manages available rooms. Peers send messages to other peers in same
//! room through `QcMessageServer`.

use std::collections::{HashMap, HashSet};

use actix::prelude::*;
use log::{info, warn};
use rand::{self, rngs::ThreadRng, Rng};

use crate::session;

/// Message for chat server communications

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<session::Message>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

/// Send message to specific room
#[derive(Message)]
#[rtype(result = "()")]
pub struct JsonMessage {
    /// Id of the client session
    pub id: usize,
    /// json serializable message
    pub json: String,
}

/// `QcMessageServer` responsible for coordinating websocket sessions
pub struct QcMessageServer {
    sessions: HashMap<usize, Recipient<session::Message>>,
    rng: ThreadRng,
}

impl Default for QcMessageServer {
    fn default() -> QcMessageServer {
        QcMessageServer {
            sessions: HashMap::new(),
            rng: rand::thread_rng(),
        }
    }
}

impl QcMessageServer {
    /// Send message to all connected recv clients
    fn send_message(&self, message: &str) {
        for (id, recipient) in self.sessions.iter() {
            recipient.do_send(session::Message(message.to_owned()));
        }
    }
}

/// Make actor from `QcMessageServer`
impl Actor for QcMessageServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for QcMessageServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);
        info!("New client connected, assigned session id: {}", id);

        // send client id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for QcMessageServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        match self.sessions.remove(&msg.id) {
            Some(_) => info!("Closing session with id {}", &msg.id),
            None => warn!("Session with id is already closed {}", &msg.id),
        }
    }
}

/// Handler for Message message.
impl Handler<JsonMessage> for QcMessageServer {
    type Result = ();

    fn handle(&mut self, msg: JsonMessage, _: &mut Context<Self>) {
        self.send_message(msg.json.as_str());
    }
}
