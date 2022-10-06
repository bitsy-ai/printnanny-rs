#![allow(dead_code)]
use std::io;

use actix::prelude::*;
use actix_codec::{Decoder, Encoder};
use actix_web::web::{BufMut, BytesMut};
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json as json;

/// Client request
#[derive(Serialize, Deserialize, Debug, Message, PartialEq)]
#[rtype(result = "()")]
#[serde(tag = "cmd", content = "data")]
pub enum QcMessageRequest {
    // Send JSON-serialized bytearray
    JsonMessage(String),
    /// Ping
    Ping,
}

/// Server response
#[derive(Serialize, Deserialize, Debug, Message, PartialEq)]
#[rtype(result = "()")]
#[serde(tag = "cmd", content = "data")]
pub enum QcMessageResponse {
    Ping,
    JsonMessage(String),
}

/// Codec for Client -> Server transport
pub struct QcMessageCodec;

impl Decoder for QcMessageCodec {
    type Item = QcMessageRequest;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let msg = json::from_slice::<QcMessageRequest>(&src)?;
        debug!("Decoded client msg {:?}", msg);
        Ok(Some(msg))
    }
}

impl Encoder<QcMessageRequest> for QcMessageCodec {
    type Error = io::Error;

    fn encode(&mut self, msg: QcMessageRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        debug!("Encoding msg {:?}", msg);
        let msg = json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();
        dst.put(msg_ref);
        Ok(())
    }
}

impl Encoder<QcMessageResponse> for QcMessageCodec {
    type Error = io::Error;

    fn encode(&mut self, msg: QcMessageResponse, dst: &mut BytesMut) -> Result<(), Self::Error> {
        debug!("Encoding msg {:?}", msg);
        let msg = json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();
        dst.put(msg_ref);
        Ok(())
    }
}

/// Codec for Server -> Client transport
pub struct ClientQcMessageCodec;

impl Decoder for ClientQcMessageCodec {
    type Item = QcMessageResponse;
    type Error = io::Error;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(Some(json::from_slice::<QcMessageResponse>(&src)?))
    }
}

impl Encoder<QcMessageRequest> for ClientQcMessageCodec {
    type Error = io::Error;

    fn encode(&mut self, msg: QcMessageRequest, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let msg = json::to_string(&msg).unwrap();
        let msg_ref: &[u8] = msg.as_ref();
        dst.put(msg_ref);
        Ok(())
    }
}
