use std::{collections::HashMap, fmt::Display, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use bimap::BiHashMap;
use log::info;
use petgraph::{Direction::Incoming, Graph, algo::toposort, graph::NodeIndex, visit::EdgeRef};

use crate::{factory::Factory, node::Node};

#[derive(Debug)]
pub struct FileGraph<T: Factory> {
    inner: Graph<(), ()>,
    idx_by_path: BiHashMap<NodeIndex, PathBuf>,
    path_by_node: HashMap<PathBuf, Node>,
    factory: T,
}

impl<T: Factory> Display for FileGraph<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node_idx in self.inner.node_indices() {
            let path = self
                .idx_by_path
                .get_by_left(&node_idx)
                .expect("corrupted graph state");
            write!(f, "{} -> ", path.display())?;

            let mut neighbors = Vec::new();
            for neighbor_idx in self.inner.neighbors(node_idx) {
                let neighbor = self
                    .idx_by_path
                    .get_by_left(&neighbor_idx)
                    .expect("corrupted graph state");
                neighbors.push(format!("{}", neighbor.display()));
            }
            writeln!(f, "[{}]", neighbors.join(", "))?;
        }
        Ok(())
    }
}

impl<T: Factory> FileGraph<T> {
    pub fn new(factory: T) -> Self {
        Self {
            inner: Graph::new(),
            idx_by_path: BiHashMap::new(),
            path_by_node: HashMap::new(),
            factory,
        }
    }

    pub fn len(&self) -> usize {
        self.inner.node_count()
    }

    /// simply add a node to the graph, this is a plain operation
    /// No dependency analysis or edge making is done
    /// use `add_tree` for that
    pub fn add_node(&mut self, node: Node, replace: bool) -> NodeIndex {
        let idx = self.idx_by_path.get_by_right(&node.path);
        match idx {
            Some(idx) => {
                if replace {
                    self.path_by_node.insert(node.path.clone(), node);
                }
                *idx
            }
            None => {
                let idx = self.inner.add_node(());
                self.raw_add_node(idx, node);
                idx
            }
        }
    }

    fn raw_add_node(&mut self, idx: NodeIndex, node: Node) {
        self.idx_by_path.insert(idx, node.path.clone());
        self.path_by_node.insert(node.path.clone(), node);
    }

    pub fn iter_nodes(&self) -> impl Iterator<Item = &Node> {
        self.path_by_node.values()
    }

    /// Take a node, and recursively add its dependencies to the graph
    /// An edge is created from the dependency to the node
    /// If the node is already present in the graph, this node is not inserted
    /// And the currently present node is used for all functions
    /// Else we insert the node
    pub fn add_tree(
        &mut self,
        node: Node,
        known_libs: &HashMap<String, PathBuf>,
        replace: bool,
        search_paths: &Vec<PathBuf>,
    ) -> Result<NodeIndex> {
        let deps = node.deps.find()?;

        let extra_search_paths = node.deps.paths_to_add_for_next_search();
        let mut search_paths = search_paths.clone();
        search_paths.extend(extra_search_paths);

        let mut all_parent_idx = Vec::new();
        for p in deps {
            if let Some(parent_idx) = self.idx_by_path.get_by_right(&p) {
                all_parent_idx.push(*parent_idx);
                continue;
            }
            let parent_node = self.factory.make(&p, known_libs, &search_paths)?;
            if let Some(parent_node) = parent_node {
                info!("adding node recursively in graph, path={}", p.display());
                let parent_idx = self
                    .add_tree(parent_node, known_libs, false, &search_paths)
                    .context(anyhow!("file: {}", p.display()))?;
                all_parent_idx.push(parent_idx);
            }
        }

        let idx = self.add_node(node, replace);
        for parent_idx in all_parent_idx {
            self.inner.add_edge(parent_idx, idx, ());
            if !self.inner.contains_edge(parent_idx, idx) {
                self.inner.add_edge(parent_idx, idx, ());
            }
        }
        Ok(idx)
    }


    pub fn get_node_by_path(&self, path: &PathBuf) -> Option<&Node> {
        self.path_by_node.get(path)
    }

    pub fn toposort(&self) -> Result<impl Iterator<Item = Node>> {
        let node_indices = toposort(&self.inner, None)
            .map_err(|e| anyhow!("{:#?}", e))
            .context("dependency analysis failed")?;

        Ok(node_indices.into_iter().map(|idx| {
            self.idx_by_path
                .get_by_left(&idx)
                .expect(&format!("fatal: failure in topological sort, found an index which we could not find in our registry: index={:?}", idx))
                .clone()
        }).map(|path| {
            self.path_by_node
                .get(&path)
                .expect(&format!("fatal: failure in topological sort, found a path which we could not find in our registry: path={:?}", path))
                .clone()
        }))
    }

    pub fn get_node_dependencies(&self, node: &Node) -> Vec<Node> {
        // given a node, return all the dependencies of the node
        self.idx_by_path
            .get_by_right(&node.path)
            .map(|idx| {
                self.inner
                    .edges_directed(*idx, Incoming)
                    .map(|e| self.get_node_by_index_or_panic(e.source()))
                    .collect::<Vec<Node>>()
            })
            .unwrap_or(vec![])
    }

    fn get_node_by_index_or_panic(&self, idx: NodeIndex) -> Node {
        let path = self.idx_by_path.get_by_left(&idx).expect(&format!(
            "corrupted graph state: could not find path for idx in edge, idx={:?}",
            idx
        ));
        self.path_by_node
            .get(path)
            .expect(&format!(
                "corrupted graph state: could not find node for path in edge, path={:}",
                path.display()
            ))
            .clone()
    }
}

#[cfg(test)]
mod test {

    use super::*;

    use std::path::PathBuf;

    use std::collections::HashMap;

    struct MockFactory {
        pub path_by_deps: HashMap<PathBuf, Vec<PathBuf>>,
    }

    impl MockFactory {
        pub fn new(path_by_deps: HashMap<PathBuf, Vec<PathBuf>>) -> Self {
            MockFactory { path_by_deps }
        }
    }

    impl Factory for MockFactory {
        fn make(
            &self,
            path: &PathBuf,
            _known_libs: &HashMap<String, PathBuf>,
            _extra_search_paths: &Vec<PathBuf>,
        ) -> Result<Option<Node>> {
            let deps = self
                .path_by_deps
                .get(path)
                .cloned()
                .unwrap_or_else(Vec::new);
            Ok(Some(Node::mock(path.clone(), deps)?))
        }

        fn make_py_executable(&self, path: &PathBuf) -> Result<Node> {
            self.make(path, &HashMap::new(), &Vec::new())
                .map(|n| n.unwrap())
        }

        fn make_with_symlinks(
            &self,
            path: &PathBuf,
            _symlinks: &Vec<String>,
            _known_libs: &HashMap<String, PathBuf>,
            _extra_search_paths: &Vec<PathBuf>,
        ) -> Result<Option<Node>> {
            self.make(path, &HashMap::new(), &Vec::new())
        }
    }
    fn create_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    fn get_graph(path_by_deps: HashMap<PathBuf, Vec<PathBuf>>) -> FileGraph<MockFactory> {
        FileGraph::new(MockFactory::new(path_by_deps))
    }

    #[test]
    fn test_add_node_single() {
        let tmp = create_temp_dir();
        let mut graph = get_graph(HashMap::new());
        let path = touch_path(&tmp, "python");
        let node = Node::mock(path, vec![]).unwrap();
        let idx = graph
            .add_tree(node, &HashMap::new(), false, &Vec::new())
            .unwrap();
        assert_eq!(graph.inner.node_count(), 1);
        assert!(graph.idx_by_path.contains_left(&idx));
    }

    #[test]
    fn test_add_node_with_dependencies() {
        let tmp = create_temp_dir();

        let p_lib_test = touch_path(&tmp, "libtest");
        let p_python = touch_path(&tmp, "python");
        let lib_test = Node::mock(p_lib_test.clone(), vec![]).unwrap();
        let py_node = Node::mock(p_python.clone(), vec![p_lib_test.clone()]).unwrap();

        let mut graph = get_graph(HashMap::from([(p_python, vec![p_lib_test])]));

        graph
            .add_tree(py_node.clone(), &HashMap::new(), false, &Vec::new())
            .unwrap();
        assert_eq!(graph.inner.node_count(), 2);
        assert_eq!(graph.inner.edge_count(), 1);
        assert!(graph.idx_by_path.contains_right(&lib_test.path));
        assert!(graph.idx_by_path.contains_right(&py_node.path));
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut graph = get_graph(HashMap::new());

        let tmp = create_temp_dir();
        let p_python = touch_path(&tmp, "python");
        let node = Node::mock(p_python, vec![]).unwrap();

        graph
            .add_tree(node.clone(), &HashMap::new(), false, &Vec::new())
            .unwrap();
        assert_eq!(graph.inner.node_count(), 1);

        graph
            .add_tree(node.clone(), &HashMap::new(), false, &Vec::new())
            .unwrap();
        assert_eq!(graph.inner.node_count(), 1); // Should not add duplicate
    }

    #[test]
    fn test_add_node_complex_dependencies() {
        println!("*************start complex test**********************");
        let tmp = create_temp_dir();


        let dep2_path = touch_path(&tmp, "dep2.py");
        let dep2 = Node::mock(dep2_path.clone(), vec![]).unwrap();

        let dep1_path = touch_path(&tmp, "libdep1");
        let dep1 = Node::mock(dep1_path.clone(), vec![dep2_path.clone()]).unwrap();

        let dep3_path = touch_path(&tmp, "libdep3");
        let dep3 = Node::mock(dep3_path.clone(), vec![dep2_path.clone()]).unwrap();

        let main_path = touch_path(&tmp, "python");
        let main = Node::mock(main_path.clone(), vec![dep1_path.clone(), dep3_path.clone()]).unwrap();

        let path_by_deps = HashMap::from([
            (main_path, vec![dep1_path.clone(), dep3_path.clone()]),
            (dep1_path, vec![dep2_path.clone()]),
            (dep3_path, vec![dep2_path.clone()]),
            (dep2_path, vec![]),
        ]);
        let mut graph = get_graph(path_by_deps);

        let result = graph.add_tree(main.clone(), &HashMap::new(), false, &Vec::new());
        println!("*************end complex adding**********************");
        assert!(result.is_ok());
        assert_eq!(graph.inner.node_count(), 4);
        assert_eq!(graph.inner.edge_count(), 4);
        assert!(graph.idx_by_path.contains_right(&main.path));
        assert!(graph.idx_by_path.contains_right(&dep1.path));
        assert!(graph.idx_by_path.contains_right(&dep2.path));

        // Test topological sort
        let nodes: Vec<Node> = graph.toposort().unwrap().collect();
        assert_eq!(nodes.len(), 4);

        assert_before(&nodes, &dep2, &dep1);
        assert_before(&nodes, &dep2, &dep3);
        assert_before(&nodes, &dep1, &main);
        assert_before(&nodes, &dep3, &main);
    }

    fn assert_before(vec: &Vec<Node>, first: &Node, second: &Node) {
        let first_pos = vec
            .iter()
            .position(|x| *x == *first)
            .expect("first element not found in vec");
        let second_pos = vec
            .iter()
            .position(|x| *x == *second)
            .expect("second element not found in vec");

        assert!(
            first_pos < second_pos,
            "Expected first element to come before second element in vec, first={:?} second={:?}",
            first,
            second
        );
    }

    fn touch_path(tmp: &tempfile::TempDir, name: &str) -> PathBuf {
        let path = tmp.path().join(name);
        std::fs::File::create(&path).expect("Failed to create file");
        path
    }
}
