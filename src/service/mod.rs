pub mod http_proxy;
mod config;

use pingora::prelude::*;
use http_proxy::*;
use crate::pkg::ingress;
use crate::service::config::Config;

pub fn start_pingora(){
    //监听pod
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build().unwrap();
    let (hpc,cfg) = rt.block_on(async {
        let recv = ingress::WatchIngress::default()
            .add_label_selector("control-class", "pingora")
            .start_watch().await.unwrap();
        let hpc = HttpProxyControl::new_ing_event_watch(recv).await;
        let cfg = Config::from_pod().await;
        (hpc,cfg)
    });

    wd_log::log_info_ln!("config=>{}",cfg.json());

    let mut my_server = Server::new(Some(Opt::default())).unwrap();
    my_server.bootstrap();

    let mut gateway = http_proxy_service(&my_server.configuration,hpc);
    gateway.add_tcp(format!("0.0.0.0:{}",cfg.port).as_str());

    my_server.add_service(gateway);

    my_server.run_forever();
}