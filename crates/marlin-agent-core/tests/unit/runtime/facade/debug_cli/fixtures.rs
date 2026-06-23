use marlin_agent_core::{LoopEdgeSpec, LoopGraph, LoopNodeSpec};

pub(super) fn single_node_graph() -> LoopGraph {
    single_node_graph_with_executor("debug.echo")
}

pub(super) fn single_node_graph_with_executor(executor: &str) -> LoopGraph {
    LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![LoopNodeSpec {
            id: "plan".to_owned(),
            executor: executor.to_owned(),
            config: Default::default(),
        }],
        edges: Vec::new(),
    }
}

pub(super) fn two_step_graph() -> LoopGraph {
    LoopGraph {
        graph_id: "graph".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "plan".to_owned(),
                executor: "debug.echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "apply".to_owned(),
                executor: "debug.echo".to_owned(),
                config: Default::default(),
            },
        ],
        edges: vec![LoopEdgeSpec {
            from: "plan".to_owned(),
            to: "apply".to_owned(),
            condition: None,
        }],
    }
}

pub(super) fn adapter_registration_graph() -> LoopGraph {
    LoopGraph {
        graph_id: "adapter-graph".to_owned(),
        nodes: vec![
            LoopNodeSpec {
                id: "tool".to_owned(),
                executor: "debug.echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "provider".to_owned(),
                executor: "debug.provider.echo".to_owned(),
                config: Default::default(),
            },
            LoopNodeSpec {
                id: "subagent".to_owned(),
                executor: "debug.subagent.echo".to_owned(),
                config: Default::default(),
            },
        ],
        edges: Vec::new(),
    }
}

pub(super) fn catalog_toml(executor: &str, adapter: &str) -> String {
    format!(
        r#"[[executors]]
executor = "{executor}"
adapter = "{adapter}"
runtime = "debug-echo"
"#
    )
}

pub(super) fn process_command_catalog_toml(executor: &str, command: &str, args: &[&str]) -> String {
    let args = args
        .iter()
        .map(|arg| format!("{arg:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        r#"[[executors]]
executor = "{executor}"
adapter = "tool"
runtime = "process-command"
command = "{command}"
args = [{args}]
"#
    )
}
