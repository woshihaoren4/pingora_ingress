use serde::{Deserialize, Serialize};
use crate::pkg::pod;

#[derive(Debug,Serialize,Deserialize)]
pub struct Config{
    #[serde(default="Config::port_df")]
    pub port:i32,
    #[serde(default="String::default")]
    pub log_level:String,
}

impl Config{
    fn port_df()->i32{
        30666
    }
    pub fn json(&self)->String{
        serde_json::to_string(self).unwrap()
    }
}


impl Config{
    pub async fn from_pod()->Self{
        let mut cfg = serde_json::from_str::<Config>("{}").unwrap();
        let pod = match pod::PodApi::get_self_pod_info().await {
            Ok(o) => o,
            Err(e) => {
                wd_log::log_error_ln!("load config failed:{:?}",e);
                return cfg
            }
        };
        if let Some(ref an) = pod.metadata.annotations {
            if let Some(level) = an.get("pga-log-level"){
                cfg.log_level = level.to_string();
            }
        };
        if let Some(ref spec) = pod.spec{
            for i in spec.containers.iter(){
                if let Some(ref ports) = i.ports{
                    for j in ports.iter(){
                        if let Some(ref n) = j.name{
                            if n=="http"{
                                cfg.port = j.container_port
                            }
                        }
                    }
                }
            }
        }
        cfg
    }
}