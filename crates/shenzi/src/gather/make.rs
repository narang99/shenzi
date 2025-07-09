use std::collections::HashMap;

use log::info;
use rayon::current_thread_index;

use crate::{
    gather::{Factory, NodeFactory},
    node::Node,
};

pub fn mk_nodes_parallel(
    node_specs: &Vec<NodeSpec>,
    factory: &NodeFactory,
) -> (Vec<Node>, Vec<NodeSpec>) {
    use rayon::prelude::*;

    let empty_known_libs = HashMap::new();
    let mut res = Vec::new();
    let mut failures = Vec::new();

    let num_threads = rayon::current_num_threads();
    let chunk_size = (node_specs.len() + num_threads - 1) / num_threads;
    info!(
        "gather: creating nodes, chunk_size={} threads={}",
        chunk_size, num_threads
    );

    let results: Vec<(Vec<Node>, Vec<NodeSpec>)> = node_specs
        .par_chunks(chunk_size)
        .map(|chunk| {
            let thread_idx = current_thread_index().unwrap_or(0);
            let mut local_res = Vec::new();
            let mut local_failures = Vec::new();
            let mut i = 0;
            let total = chunk.len();
            for payload in chunk {
                let node = factory.make_from_spec(payload, &empty_known_libs);
                match node {
                    Ok(node) => {
                        local_res.push(node);
                    }
                    Err(_) => {
                        local_failures.push(payload.clone());
                    }
                }
                i += 1;
                if total / 10 != 0 && i % (total / 10) == 0 {
                    info!("thread: {} exported {}/{} files", thread_idx, i, total);
                }
            }
            (local_res, local_failures)
        })
        .collect();

    for (local_res, local_failures) in results {
        res.extend(local_res);
        failures.extend(local_failures);
    }

    (res, failures)
}
