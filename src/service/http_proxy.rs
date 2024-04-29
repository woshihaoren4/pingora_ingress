use std::collections::HashMap;
use std::sync::Arc;
use async_channel::Receiver;
use wd_tools::sync::Acl;
use crate::infra::url_tree::Node;
use crate::pkg::ingress::{IngressEvent, IngRule};
use pingora::prelude::*;
use wd_tools::PFArc;

#[derive(Default)]
pub struct HttpProxyControl{
    router : Acl<HashMap<String,Router>>,
}
impl HttpProxyControl {
    pub async fn new_ing_event_watch(recv:Receiver<IngressEvent>)->Self{
        let router = Acl::default();
        let rt = router.clone();
        tokio::spawn(async move{
            while let Ok(e) = recv.recv().await{
                wd_log::log_info_ln!("watch ingress event=>{}",e.json());
                HttpProxyControl::ing_event_to_router(e,rt.clone());
            }
            wd_log::log_info_ln!("IngressEvent receiver channel over");
        });
        Self{router}
    }
    fn ing_event_to_router(ing:IngressEvent,acl:Acl<HashMap<String,Router>>){
        let IngressEvent{
            ty, default_backend, hosts, sni, ..
        } = ing;
        let mut map = (*acl.share()).clone();
        match ty {
            1 | 2=>{ //init | update
                if let Some(db) = default_backend {
                    map.insert("*".into(), Router::from_default_backend(db));
                }
                for i in hosts{
                    if !map.contains_key(i.host.as_str()) {
                        map.insert(i.host.clone(),Router::from_host(i.host.clone()));
                    }
                    let router = map.get_mut(i.host.as_str()).unwrap();
                    wd_log::log_debug_ln!("update host:[{}]",i.host.as_str());
                    router.update_from_ing_rule(i.rules);
                }
            }
            3=>{ //delete
                if let Some(db) = default_backend {
                    wd_log::log_debug_ln!("delete default_backend:[{}]",db.backend);
                    map.remove("*");
                }
                for i in hosts{
                    wd_log::log_debug_ln!("delete host:[{}]",i.host.as_str());
                    map.remove(i.host.as_str());
                }
            }
            _=>{
                wd_log::log_info_ln!("unknown ingress event type:{:?}",ty);
                return;
            }
        }
        for (host,i) in sni.sni{
            if let Some(router) = map.get_mut(host.as_str()){
                router.sni = i;
            }
        }
        acl.update(move |_|{
            map
        });
    }
}

#[derive(Default,Clone,Debug)]
pub struct Router{
    pub sni:String,
    pub host:String,
    pub default_backend:Option<Arc<RouterNode>>,
    pub exact:HashMap<String,Arc<RouterNode>>,
    pub prefix:Node<RouterNode>,
}
#[derive(Clone,Debug)]
pub struct RouterNode{
    pub backend: String,
    pub port:i32,
}
impl From<IngRule> for RouterNode{
    fn from(value: IngRule) -> Self {
        let backend = value.backend;
        let port = value.port;
        Self{backend,port}
    }
}

impl RouterNode {
    pub fn new<B:Into<String>>(backend:B,port:i32)->Self{
        let backend = backend.into();
        Self{backend,port}
    }

}
impl Router{
    pub fn from_host<S:Into<String>>(host:S)->Self{
        let host = host.into();
        Self{host,..Default::default()}
    }
    pub fn from_default_backend(ir:IngRule)->Self{
        let mut rt = Router::default();
        rt.default_backend = Some(Arc::new(RouterNode::from(ir)));
        rt
    }
    pub fn update_from_ing_rule(&mut self,rules:Vec<IngRule>){
        for rule in rules{
            let IngRule{ path, backend, port,ty } = rule;
            match ty {
                1=>{ //prefix
                    wd_log::log_debug_ln!("insert prefix rule: path[{}] service[{}] port[{}]",path,backend,port);
                    self.prefix.insert_path(path.as_str(),RouterNode::new(backend,port).arc());
                }
                2=>{ //exact
                    wd_log::log_debug_ln!("insert exact rule: path[{}] service[{}] port[{}]",path,backend,port);
                    self.exact.insert(path,RouterNode::new(backend,port).arc());
                }
                _=>{
                    wd_log::log_warn_ln!("RouterNode do not support specific path:{}",path);
                }
            }
        }
    }
}

#[derive(Default)]
pub struct HttpProxyCtx{
    service:Option<Arc<RouterNode>>,
    sni:String,
}

#[async_trait::async_trait]
impl ProxyHttp for HttpProxyControl{
    type CTX = HttpProxyCtx;

    fn new_ctx(&self) -> Self::CTX {
        HttpProxyCtx::default()
    }




    async fn upstream_peer(&self, _session: &mut Session, ctx: &mut Self::CTX) -> Result<Box<HttpPeer>> {
        let peer = if let Some(ref s) = ctx.service {
            Box::new(HttpPeer::new((s.backend.as_str(),s.port as u16), !ctx.sni.is_empty(), ctx.sni.clone()))
        }else{
            return Error::err(ErrorType::HTTPStatus(404));
        };

        // let peer = Box::new(HttpPeer::new(("1.1.1.1",80u16), true, "one.one.one.one".to_string()));
        Ok(peer)
    }

    async fn request_filter(&self, session: &mut Session, ctx: &mut Self::CTX) -> Result<bool> where Self::CTX: Send + Sync {
        let mut host = if let Some(s) = session.req_header().headers.get("Host") {
            if let Ok(s) = s.to_str(){
                s
            }else{
                return Error::err(ErrorType::InvalidHTTPHeader)
            }
        }else {
            return Error::err(ErrorType::InvalidHTTPHeader)
        };
        if host.contains(":") {
            let list:Vec<&str> = host.split(":").collect();
            host = list[0];
        }
        let path = session.req_header().uri.path();

        wd_log::log_debug_ln!("request host[{}] path[{}]",host,path);

        let routers = self.router.share();
        for i in 0..100{
            if i != 0 {
                let list:Vec<&str> = host.splitn(1,'.').collect();
                if list.len() == 1 || list.is_empty() {
                    break
                }
                host = list[1];
            }
            if let Some(r) = routers.get(host) {
                ctx.sni = r.sni.clone();
                if let Some(s) = r.exact.get(path) {
                    ctx.service = Some(s.clone());
                }else if let Some(s) = r.prefix.find_by_path(path){
                    ctx.service = Some(s);
                }
                break
            }
        }
        //尝试兜底
        if ctx.service.is_none() {
            if let Some(r) = routers.get("*") {
                ctx.sni = r.sni.clone();
                ctx.service = r.default_backend.clone();
            }
        }
        //如果没找到
        if ctx.service.is_none(){
            return Error::err(ErrorType::HTTPStatus(404));
        }
        Ok(false)
    }
}