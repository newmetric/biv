use crate::{packet::Rpc, runtime::Runtime};

mod packet;
mod runtime;
mod util;
#[tokio::main]
async fn main() {
    // let packet = packet::Packet::Rpc(Rpc {
    //     src: "hi".to_owned(),
    //     dst: "todo!()".to_owned(),
    //     data: "adsfas".as_bytes().to_vec(),
    // });
    // println!("{}", serde_json::to_string(&packet).unwrap());
    // launch().await;
}

// Design decisions. We can make it cli but also code.
