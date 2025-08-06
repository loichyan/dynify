use dynify::PinDynify;

#[trait_variant::make(Send)]
#[dynify::dynify]
trait Client {
    async fn request(&self, uri: &str) -> String;
}

fn run_client(
    client: &(dyn DynClient + Sync),
) -> impl '_ + std::future::Future<Output = ()> + Send {
    async move {
        client.request("http://magic/request").pin_boxed().await;
    }
}

#[trait_variant::make(Server: Send)]
#[dynify::dynify]
trait LocalServer {
    async fn request(&self, uri: &str) -> String;
}

fn run_server(
    client: &(dyn DynServer + Sync),
) -> impl '_ + std::future::Future<Output = ()> + Send {
    async move {
        client.request("http://magic/request").pin_boxed().await;
    }
}

fn main() {}
