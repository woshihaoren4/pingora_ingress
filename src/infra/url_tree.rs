use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Node<T> {
    path: String,
    data: Option<Arc<T>>,
    next: HashMap<String, Node<T>>,
}

impl<T> Default for Node<T> {
    fn default() -> Self {
        Self {
            path: String::new(),
            data: None,
            next: HashMap::new(),
        }
    }
}

impl<T> Node<T> {
    pub fn insert_path(&mut self, path: &str, data: Arc<T>) {
        if path.is_empty() {
            self.data = Some(data);
            return;
        }
        let ps = path
            .split('/')
            .map(|x| if x.is_empty() { "*" } else { x })
            .rev()
            .collect::<Vec<&str>>();
        self.insert(ps, data);
    }
    pub fn find_by_path(&self, path: &str) -> Option<Arc<T>> {
        let ps = path.split('/').rev().collect::<Vec<&str>>();
        self.find(ps)
    }
    pub fn insert(&mut self, mut ps: Vec<&str>, data: Arc<T>) {
        let path = if let Some(s) = ps.pop() {
            s
        } else {
            return;
        };
        //先判断路径是否匹配
        if self.path.is_empty() {
            self.path = path.to_string();
        } else if self.path.as_str() == path {
        } else {
            panic!("Url.Tree.Node.insert unknown path:{}", path);
        }
        //看下一个
        let next = if let Some(p) = ps.last() {
            self.next.get_mut(*p)
        } else {
            //不存在下一个，则修改当前
            self.data = Some(data);
            return;
        };
        if let Some(next) = next {
            //存在下一个则继续判断
            next.insert(ps, data);
        } else {
            //不存在则新建
            let mut next = Node::<T>::default();
            next.insert(ps, data);
            self.next.insert(next.path.clone(), next);
        }
    }
    pub fn find(&self, mut ps: Vec<&str>) -> Option<Arc<T>> {
        let path = ps.pop()?;

        if self.path == "*" {
        } else if path != self.path {
            return self.data.clone();
        }

        if let Some(p) = ps.last() {
            if let Some(next) = self.next.get(*p) {
                let res = next.find(ps);
                if res.is_some() {
                    return res;
                }
            } else if let Some(next) = self.next.get("*") {
                let res = next.find(ps);
                if res.is_some() {
                    return res;
                }
            }
        }
        self.data.clone()
    }
}

#[cfg(test)]
mod test {
    use crate::infra::url_tree::Node;
    use std::sync::Arc;

    #[test]
    fn test_node() {
        let mut root = Node::default();

        root.insert_path("/api/v1/task/create", Arc::new("avtc"));
        root.insert_path("/api/v1/task/delete", Arc::new("avtd"));
        root.insert_path("/api/v2/update", Arc::new("avu"));
        root.insert_path("/api/v2/", Arc::new("av2"));
        root.insert_path("/api", Arc::new("api"));
        root.insert_path("/", Arc::new("default"));
        root.insert_path("", Arc::new("null"));

        let res = root.find_by_path("/api/v1/task/create").unwrap();
        assert_eq!(*res, "avtc");

        let res = root.find_by_path("/api/v1/task/create/1234").unwrap();
        assert_eq!(*res, "avtc");

        let res = root.find_by_path("/api/v1/task").unwrap();
        assert_eq!(*res, "api");

        let res = root.find_by_path("/api/v2/task/update").unwrap();
        assert_eq!(*res, "av2");

        let res = root.find_by_path("/api/v2").unwrap();
        assert_eq!(*res, "api");

        let res = root.find_by_path("/api").unwrap();
        assert_eq!(*res, "api");

        let res = root.find_by_path("/").unwrap();
        assert_eq!(*res, "default");

        let res = root.find_by_path("").unwrap();
        assert_eq!(*res, "null");

        root.insert_path("/api/v2/", Arc::new("api2"));
        let res = root.find_by_path("/api/v2/xxx").unwrap();
        assert_eq!(*res, "api2");
    }
}
