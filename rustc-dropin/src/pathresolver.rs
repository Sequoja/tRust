use std::collections::HashMap;
use syntax::{ast, visit};

/// Used for path resolution
pub struct PathResolver {
    resolv_paths: HashMap<String, String>,
    // code_2_monitor: HashSet<String>,
}

impl PathResolver {
    /// Consrtucts the PathResolver struct
    pub fn new() -> PathResolver {
        PathResolver {
            resolv_paths: HashMap::new(),
            // code_2_monitor: c_2_m,
        }
    }

    /// Return HashMap for path resolution
    pub fn find_resolv_paths(&mut self, krate: &ast::Crate) -> HashMap<String, String> {
        visit::walk_crate(self, krate);
        self.resolv_paths.clone()
    }

    /// Add path to the HashMap
    fn add_path(&mut self, key: String, val: String) {
        if key.is_empty() && !val.is_empty() {
            let new_key = val
                .split("::")
                .collect::<Vec<_>>()
                .pop()
                .unwrap()
                .to_string();
            if self.resolv_paths.get(&new_key).is_none() {
                self.resolv_paths.insert(new_key, val);
            } else {
                eprint!("Path {} allready exists", val)
            }
        } else if !val.is_empty() && val.ends_with('*') {
            let mut segments = val.split("::").collect::<Vec<_>>();
            let last = segments.pop().unwrap_or("");
            let sec_2_last = segments.pop().unwrap_or("");
            let new_key = format!("{}{}", sec_2_last, last);
            if self.resolv_paths.get(&new_key).is_none() {
                self.resolv_paths.insert(new_key, val);
            } else {
                eprint!("Path {} allready exists", val)
            }
        } else if self.resolv_paths.get(&key).is_none() && !val.is_empty() {
            self.resolv_paths.insert(key, val);
        } else {
            eprint!("Path {} allready exists", val)
        }
    }
}

impl<'p> visit::Visitor<'p> for PathResolver {
    fn visit_item(&mut self, val: &'p ast::Item) {
        match val.node {
            ast::ItemKind::Fn(..) => {
                self.add_path(
                    val.ident.to_string(),
                    ast::Path::from_ident(val.ident).to_string(),
                );
            }
            ast::ItemKind::ExternCrate(Some(name)) => {
                self.add_path(val.ident.to_string(), name.to_string());
            }
            ast::ItemKind::Use(ref use_tree) => {
                self.add_path(val.ident.to_string(), use_tree.prefix.to_string());
            }
            _ => (),
        }

        visit::walk_item(self, val);
    }

    fn visit_impl_item(&mut self, val: &'p ast::ImplItem) {
        if let ast::ImplItemKind::Method(..) = val.node {
            self.resolv_paths.insert(
                val.ident.to_string(),
                ast::Path::from_ident(val.ident).to_string(),
            );
        }

        visit::walk_impl_item(self, val);
    }

    fn visit_mac(&mut self, _mac: &'p ast::Mac) {
        // panic!("visit_mac disabled by default");
        // N.B., see note about macros above.
        // if you really want a visitor that
        // works on macros, use this
        // definition in your trait impl:
        visit::walk_mac(self, _mac)
    }
}
