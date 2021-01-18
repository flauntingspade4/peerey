use actix::Message;
use bytes::{BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

#[derive(Clone)]
pub struct ChatMessage(pub Bytes);

impl Message for ChatMessage {
    type Result = ();
}
pub struct Codec;

impl Encoder<ChatMessage> for Codec {
    type Error = std::io::Error;
    fn encode(&mut self, response: ChatMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.reserve(response.0.len());
        dst.put(response.0);

        Ok(())
    }
}

impl Decoder for Codec {
    type Item = ChatMessage;
    type Error = std::io::Error;
    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.is_empty() {
            Ok(None)
        } else {
            let len = src.len();
            let msg = src.split_to(len);
            Ok(Some(ChatMessage(msg.freeze())))
        }
    }
}
