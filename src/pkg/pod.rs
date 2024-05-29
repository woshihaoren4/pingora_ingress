use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client};
use std::env;
use std::fs::read_to_string;
use wd_tools::PFErr;

pub struct PodApi {}

impl PodApi {
    pub async fn get_self_pod_info() -> anyhow::Result<Pod> {
        let pod_name = PodApi::pod_name();
        if pod_name.is_empty() {
            return anyhow::anyhow!("pod name not found").err();
        } else {
            wd_log::log_debug_ln!("pod name[{}]", pod_name);
        }
        let namespace = PodApi::namespace();
        if namespace.is_empty() {
            return anyhow::anyhow!("namespace not found").err();
        } else {
            wd_log::log_debug_ln!("namespace name[{}]", namespace);
        }

        let client = Client::try_default().await?;
        let api = Api::<Pod>::namespaced(client, namespace.as_str());

        let pod = api.get(pod_name.as_str()).await?;

        Ok(pod)
    }

    pub fn pod_name() -> String {
        if let Ok(n) = env::var("HOSTNAME") {
            return n;
        }
        if let Ok(n) = read_to_string("/etc/hostname") {
            return n.replace('\n', "");
        }
        "".into()
    }
    pub fn namespace() -> String {
        if let Ok(n) = env::var("NAMESPACE") {
            return n;
        }
        if let Ok(n) = read_to_string("/run/secrets/kubernetes.io/serviceaccount/namespace") {
            return n.replace('\n', "");
        }
        "".into()
    }
}

#[cfg(test)]
mod test {
    use k8s_openapi::api::core::v1::Pod;
    use kube::{Api, Client};

    #[tokio::test]
    pub async fn test_pod_info() {
        let client = Client::try_default().await.unwrap();

        let api = Api::<Pod>::namespaced(client, "qa");

        let pod = api
            .get("pingora-ingress-ctl-f4bd748c8-p7khx")
            .await
            .unwrap();

        println!("pod=>{:?}", pod);
    }
}
