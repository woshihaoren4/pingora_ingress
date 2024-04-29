use std::collections::HashMap;
use async_channel::Receiver;
use k8s_openapi::api::networking::v1::{HTTPIngressPath, Ingress, IngressRule, IngressServiceBackend, IngressTLS};
use kube::{Api, Client};
use futures::prelude::*;
use kube::runtime::{watcher, WatchStreamExt};
use kube::runtime::watcher::{ Event};
use serde::{Deserialize, Serialize};
use wd_tools::PFSome;

const INGRESS_CLASS_NAME_PINGORA:&'static str = "pingora";

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct IngressEvent{
    pub ty:u8, //1:init  2:update 3:delete
    pub default_backend:Option<IngRule>,
    pub hosts:Vec<IngHost>,
    pub sni:IngSni,
    #[serde(skip)]
    pub ing:Option<Event<Ingress>>,
}
impl IngressEvent{
    pub fn init(mut self)->Self{
        match self.ing.as_ref().unwrap() {
            Event::Applied(ref k) => {
                let (default_backend,hosts,sni) = IngressEvent::ing_to_host_backend(k);
                self.ty = 2;
                self.default_backend = default_backend;
                self.hosts = hosts;
                self.sni = sni;
            },
            Event::Deleted(ref k) => {
                let (default_backend,hosts,sni) = IngressEvent::ing_to_host_backend(k);
                self.ty = 3;
                self.default_backend = default_backend;
                self.hosts = hosts;
                self.sni = sni;
            },
            Event::Restarted(ref k) => {
                let mut default_backend = None;
                let mut hosts = vec![];
                let mut sni = IngSni::default();
                for i in k.iter(){
                    let (db, mut hs,is) = IngressEvent::ing_to_host_backend(i);
                    if db.is_some() {
                        default_backend = db;
                    }
                    hosts.append(&mut hs);
                    sni.append(is);
                }
                self.ty = 1;
                self.default_backend = default_backend;
                self.hosts = hosts;
                self.sni = sni;
            },
        };

        self
    }
    pub fn ing_to_host_backend(ing:&Ingress)->(Option<IngRule>,Vec<IngHost>,IngSni){
        let i = if let Some(ref i) = ing.spec{
            i
        }else{
            return (None,vec![],IngSni::default())
        };
        if let Some(ref n) = i.ingress_class_name {
            if n != INGRESS_CLASS_NAME_PINGORA {
                return (None,vec![],IngSni::default())
            }
        }
        let default_backend = if let Some(ref d) = i.default_backend{
            if let Some(ref s) = d.service{Some(IngRule::from(s))}else{None}
        }else{None};
        let mut hosts = vec![];
        if let Some(ref i) = i.rules{
            for i in i.iter(){
                let ih = IngHost::from(i);
                if !ih.rules.is_empty() {
                    hosts.push(ih);
                }
            }
        }
        let sni = if let Some(ref s) = i.tls{
            IngSni::from_ingress_tls(s)
        }else{
            IngSni::default()
        };
        (default_backend,hosts,sni)
    }
    pub fn json(&self)->String{
        serde_json::to_string(self).unwrap_or_else(|e| {
            wd_log::log_error_ln!("IngressEvent json error:{}",e);
            "".to_string()
        })
    }
}
impl From<Event<Ingress>> for IngressEvent {
    fn from(value: Event<Ingress>) -> Self {
        let ie = IngressEvent{
            ty: 0,
            default_backend: None,
            hosts: vec![],
            sni: Default::default(),
            ing: Some(value),
        };
        ie.init()
    }
}
#[derive(Default,Debug,Clone,Serialize,Deserialize)]
pub struct IngSni{
    pub sni : HashMap<String,String>
}
impl IngSni{
    pub fn from_ingress_tls(tls:&Vec<IngressTLS>)->IngSni{
        let mut sni = HashMap::new();
        for i in tls.iter(){
            let secret = if let Some(ref s) = i.secret_name {
                s.clone()
            }else{
                continue
            };
            if let Some(ref hs) = i.hosts{
                for j in hs.iter(){
                    sni.insert(j.clone(),secret.clone());
                }
            }
        }
        Self{sni}
    }
    pub fn append(&mut self,other:Self){
        self.sni.extend(other.sni);
    }
}

#[derive(Default,Debug,Clone,Serialize,Deserialize)]
pub struct IngHost{
    pub host:String,
    pub rules:Vec<IngRule>,
}

impl From<&IngressRule> for IngHost {
    fn from(value: &IngressRule) -> Self {
        let host = if let Some(ref s) = value.host{
            s.clone()
        }else{"".to_string()};
        let mut rules = vec![];
        if let Some(ref list) = value.http{
            for p in list.paths.iter(){
                if let Some(r) = IngRule::new_from_path(p){
                    rules.push(r)
                }
            }
        }
        Self{host,rules}
    }
}

#[derive(Default,Debug,Clone,Serialize,Deserialize)]
pub struct IngRule{
    pub path: String,
    pub ty:u8, //1:prefix 2:exact 3:specific
    pub backend: String,
    pub port:i32,
}

impl From<&IngressServiceBackend> for IngRule {
    fn from(value: &IngressServiceBackend) -> Self {
        let mut this = Self{
            path: "".to_string(),
            ty: 1,
            backend: value.name.clone(),
            port: 80,
        };
        if let Some(ref i) = value.port{
            if let Some(i) = i.number{
                this.port = i
            }
        }
        this
    }
}
impl IngRule{
    pub fn new_from_path(value:&HTTPIngressPath)->Option<Self>{
        let ty = match value.path_type.as_str().to_lowercase().as_str() {
            "prefix" => 1u8,
            "exact" => 2u8,
            "implementationspecific" => 3u8,
            _ => 99u8,
        };
        let path = value.path.clone().unwrap_or("".to_string());
        let mut this = if let Some(ref s) = value.backend.service{
            IngRule::from(s)
        }else{
            return None
        };
        this.ty = ty;
        this.path = path;
        this.some()
    }
}

#[derive(Default,Debug,Clone)]
pub struct WatchIngress{
    namespace:Option<String>,
    selector_labels:Option<String>,
}

impl WatchIngress {
    #[allow(dead_code)]
    pub fn from_namespace<S:Into<String>>(ns:S)->Self{
        let namespace = Some(ns.into());
        Self{namespace,
        .. Default::default()}
    }
    pub fn add_label_selector(mut self,key:&str,value:&str)->Self{
        self.selector_labels = match self.selector_labels {
            None => Some(format!("{}={}",key,value)),
            Some(s) => {Some(format!("{},{}={}",s,key,value))}
        };self
    }
    pub async fn start_watch(&self)-> anyhow::Result<Receiver<IngressEvent>> {
        let (sender,receiver) = async_channel::bounded(8);


        let client = Client::try_default().await?;
        let api:Api<Ingress> = match self.namespace {
            None =>{
                Api::all(client)
            } ,
            Some(ref s) => Api::namespaced(client,s)
        };

        let mut wc = watcher::Config::default();
        if let Some(ref s) = self.selector_labels{
            wc = wc.labels(s)
        };
        let mut watch = watcher(api, wc)
            // .applied_objects()
            .default_backoff().boxed();
        tokio::spawn(async move {
            while let Some(result) = watch.next().await{
                let event = match result{
                    Ok(o) => o,
                    Err(e) => {
                        wd_log::log_error_ln!("watch ingress event error:{:?}",e);
                        continue
                    }
                };
                let event = IngressEvent::from(event);
                if let Err(e) = sender.send(event).await{
                    wd_log::log_error_ln!("watch ingress event to sender error:{:?}",e)
                }
            }
        });
        // let mut watch = watcher(api, wc).boxed();
        // #[allow(irrefutable_let_patterns)]
        // while let res = watch.try_next().await {
        //     if let Ok(Some(event)) = res{
        //         wd_log::log_info_ln!("event-->{:?}",event);
        //     }else{
        //         wd_log::log_warn_ln!("watch unknown error");
        //     }
        // }
        Ok(receiver)
    }

}