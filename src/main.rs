mod pkg;

use pkg::*;

#[tokio::main]
async fn main(){
    ingress::WatchIngress::default()
        .add_label_selector("control-class","pingora")
        .start_watch().await.unwrap();
}