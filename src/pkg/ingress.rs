use k8s_openapi::api::networking::v1::Ingress;
use kube::{Api, Client};
use kube::api::WatchParams;
use futures::prelude::*;
use kube::runtime::{watcher, WatchStreamExt};

#[derive(Default,Debug,Clone)]
pub struct WatchIngress{
    namespace:Option<String>,
    selector_labels:Option<String>,
}

impl WatchIngress {
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
    pub async fn start_watch(&self)-> anyhow::Result<()> {
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
        watcher(api,wc)
            .applied_objects()
            .default_backoff()
            .try_for_each(|ing|async move{
                wd_log::log_info_ln!("ingress->{:?}",ing);
                Ok(())
            }).await?;
        Ok(())
    }

}