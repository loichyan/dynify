use std::future::Future;
use std::mem::MaybeUninit;

use dynify::Dynify;

#[trait_variant::make(Send)]
#[dynify::dynify]
trait Client {
    async fn request(&self, uri: &str) -> String;
}

async fn make_request(client: &(dyn Sync + DynClient)) {
    let mut stack = [MaybeUninit::<u8>::uninit(); 16];
    let mut heap = Vec::<MaybeUninit<u8>>::new();
    client
        .request("http://magic/coffee/shop")
        .init2(&mut stack, &mut heap)
        .await;
}

fn poll_future(fut: impl Send + Future<Output = ()>) {
    pollster::block_on(Box::pin(fut));
}

struct MyClient(String);

impl Client for MyClient {
    async fn request(&self, uri: &str) -> String {
        println!("request from {} to {}", self.0, uri);
        String::from("Cheer up, my friend!")
    }
}

fn main() {
    let client = MyClient("latte".into());
    let fut = make_request(&client);
    poll_future(fut);
}
