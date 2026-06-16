//! Argument parsing for the `marlin` harness/debug CLI.

use std::path::PathBuf;

use crate::protocol::GraphQueryFamily;

pub(super) const DEFAULT_RUN_STORE: &str = ".marlin/runs";
pub(super) const DEFAULT_GERBIL_PACKAGE_ROOT: &str = "crates/marlin-gerbil-scheme/gerbil";
pub(super) const DEFAULT_GERBIL_POLICY_RECEIPT_ENTRYPOINT: &str =
    "src/marlin/deck-runtime-policy-receipt-gate-cli.ss";

#[derive(Clone, Debug)]
pub(super) struct ArgCursor {
    args: Vec<String>,
    index: usize,
}

impl ArgCursor {
    pub(super) fn new(args: Vec<String>) -> Self {
        Self { args, index: 0 }
    }

    pub(super) fn next(&mut self) -> Option<String> {
        let arg = self.args.get(self.index).cloned();
        if arg.is_some() {
            self.index += 1;
        }
        arg
    }

    fn required_value(&mut self, option: &str) -> Result<String, String> {
        self.next()
            .ok_or_else(|| format!("{option} requires a value"))
    }

    fn required_path(&mut self, option: &str) -> Result<PathBuf, String> {
        self.required_value(option).map(PathBuf::from)
    }
}

#[derive(Clone, Debug)]
pub(super) struct CommonOptions {
    pub(super) input: Option<PathBuf>,
}

impl CommonOptions {
    pub(super) fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let mut input = None;
        while let Some(arg) = cursor.next() {
            match arg.as_str() {
                "--input" | "-i" => input = Some(cursor.required_path(&arg)?),
                "-h" | "--help" => return Err(super::usage()),
                unknown => return Err(format!("unknown option `{unknown}`")),
            }
        }
        Ok(Self { input })
    }
}

#[derive(Clone, Debug)]
pub(super) struct GerbilPolicyReceiptOptions {
    pub(super) entrypoint: PathBuf,
    pub(super) gxi: PathBuf,
    pub(super) iterations: u64,
    pub(super) loadpath: Option<String>,
    pub(super) package_root: PathBuf,
}

impl GerbilPolicyReceiptOptions {
    pub(super) fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let mut gxi = std::env::var_os("GERBIL")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/usr/local/bin/gxi"));
        let mut entrypoint = std::env::var_os("MARLIN_GERBIL_POLICY_RECEIPT_ENTRYPOINT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_POLICY_RECEIPT_ENTRYPOINT));
        let mut iterations = 1;
        let mut loadpath = None;
        let mut package_root = std::env::var_os("MARLIN_GERBIL_PACKAGE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_GERBIL_PACKAGE_ROOT));
        while let Some(arg) = cursor.next() {
            match arg.as_str() {
                "--entrypoint" => entrypoint = cursor.required_path(&arg)?,
                "--gxi" => gxi = cursor.required_path(&arg)?,
                "--iterations" => {
                    iterations = parse_positive_u64(&arg, &cursor.required_value(&arg)?)?
                }
                "--loadpath" => loadpath = Some(cursor.required_value(&arg)?),
                "--package-root" => package_root = cursor.required_path(&arg)?,
                "-h" | "--help" => return Err(super::usage()),
                unknown => return Err(format!("unknown option `{unknown}`")),
            }
        }
        Ok(Self {
            entrypoint,
            gxi,
            iterations,
            loadpath,
            package_root,
        })
    }
}

fn parse_positive_u64(option: &str, value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{option} requires a positive unsigned integer"))?;
    if parsed == 0 {
        return Err(format!("{option} requires a positive unsigned integer"));
    }
    Ok(parsed)
}

#[derive(Clone, Debug)]
pub(super) struct GraphQueryOptions {
    pub(super) family: Option<GraphQueryFamily>,
    pub(super) input: Option<PathBuf>,
    pub(super) org_memory_fixtures: Vec<PathBuf>,
    pub(super) org_memory_roots: Vec<String>,
    pub(super) org_memory_store_root: Option<PathBuf>,
    pub(super) org_tool_roots: Vec<String>,
    pub(super) org_tool_store_root: Option<PathBuf>,
    pub(super) org_topology_roots: Vec<String>,
    pub(super) org_topology_store_root: Option<PathBuf>,
    pub(super) receipt_id: String,
}

impl GraphQueryOptions {
    pub(super) fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let mut family = None;
        let mut input = None;
        let mut org_memory_fixtures = Vec::new();
        let mut org_memory_roots = Vec::new();
        let mut org_memory_store_root = None;
        let mut org_tool_roots = Vec::new();
        let mut org_tool_store_root = None;
        let mut org_topology_roots = Vec::new();
        let mut org_topology_store_root = None;
        let mut receipt_id = "debug-project-memory-query".to_owned();
        while let Some(arg) = cursor.next() {
            match arg.as_str() {
                "--family" => {
                    family = Some(parse_graph_query_family(&cursor.required_value(&arg)?)?)
                }
                "--input" | "-i" => input = Some(cursor.required_path(&arg)?),
                "--org-memory-fixture" => org_memory_fixtures.push(cursor.required_path(&arg)?),
                "--org-memory-root" => org_memory_roots.push(cursor.required_value(&arg)?),
                "--org-memory-store-root" => {
                    org_memory_store_root = Some(cursor.required_path(&arg)?)
                }
                "--org-tool-root" => org_tool_roots.push(cursor.required_value(&arg)?),
                "--org-tool-store-root" => org_tool_store_root = Some(cursor.required_path(&arg)?),
                "--org-topology-root" => org_topology_roots.push(cursor.required_value(&arg)?),
                "--org-topology-store-root" => {
                    org_topology_store_root = Some(cursor.required_path(&arg)?)
                }
                "--receipt-id" => receipt_id = cursor.required_value(&arg)?,
                "-h" | "--help" => return Err(super::usage()),
                unknown => return Err(format!("unknown option `{unknown}`")),
            }
        }
        Ok(Self {
            family,
            input,
            org_memory_fixtures,
            org_memory_roots,
            org_memory_store_root,
            org_tool_roots,
            org_tool_store_root,
            org_topology_roots,
            org_topology_store_root,
            receipt_id,
        })
    }
}

fn parse_graph_query_family(value: &str) -> Result<GraphQueryFamily, String> {
    match value.to_ascii_lowercase().as_str() {
        "org" => Ok(GraphQueryFamily::Org),
        "memory" => Ok(GraphQueryFamily::Memory),
        "tool" => Ok(GraphQueryFamily::Tool),
        "session" => Ok(GraphQueryFamily::Session),
        "content" => Ok(GraphQueryFamily::Content),
        "topology" => Ok(GraphQueryFamily::Topology),
        "evidence" => Ok(GraphQueryFamily::Evidence),
        "failure" | "failures" => Ok(GraphQueryFamily::Failure),
        unknown => Err(format!(
            "unsupported graph query family `{unknown}`; expected org, memory, tool, session, content, topology, evidence, or failure"
        )),
    }
}

#[derive(Clone, Debug)]
pub(super) struct GraphProposeOptions {
    pub(super) strategy: String,
    pub(super) strategy_id: String,
    pub(super) version: String,
    pub(super) input_digest: String,
    pub(super) output_digest: String,
    pub(super) input: Option<PathBuf>,
}

impl GraphProposeOptions {
    pub(super) fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let mut options = Self {
            strategy: "static".to_owned(),
            strategy_id: "static-debug".to_owned(),
            version: "v1".to_owned(),
            input_digest: "debug:input".to_owned(),
            output_digest: "debug:output".to_owned(),
            input: None,
        };
        while let Some(arg) = cursor.next() {
            match arg.as_str() {
                "--strategy" => options.strategy = cursor.required_value(&arg)?,
                "--strategy-id" => options.strategy_id = cursor.required_value(&arg)?,
                "--version" => options.version = cursor.required_value(&arg)?,
                "--input-digest" => options.input_digest = cursor.required_value(&arg)?,
                "--output-digest" => options.output_digest = cursor.required_value(&arg)?,
                "--input" | "-i" => options.input = Some(cursor.required_path(&arg)?),
                unknown => return Err(format!("unknown option `{unknown}`")),
            }
        }
        Ok(options)
    }
}

#[derive(Clone, Debug)]
pub(super) struct GraphRunOptions {
    pub(super) input: Option<PathBuf>,
    pub(super) run_id: String,
    pub(super) catalog: Option<PathBuf>,
}

impl GraphRunOptions {
    pub(super) fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let mut input = None;
        let mut run_id = "marlin-graph-run".to_owned();
        let mut catalog = None;
        while let Some(arg) = cursor.next() {
            match arg.as_str() {
                "--input" | "-i" => input = Some(cursor.required_path(&arg)?),
                "--run-id" => run_id = cursor.required_value(&arg)?,
                "--catalog" => catalog = Some(cursor.required_path(&arg)?),
                unknown => return Err(format!("unknown option `{unknown}`")),
            }
        }
        Ok(Self {
            input,
            run_id,
            catalog,
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct LoopRunOptions {
    pub(super) input: Option<PathBuf>,
    pub(super) max_iterations: Option<u64>,
    pub(super) store: Option<PathBuf>,
    pub(super) catalog: Option<PathBuf>,
}

impl LoopRunOptions {
    pub(super) fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let mut input = None;
        let mut max_iterations = None;
        let mut store = Some(PathBuf::from(DEFAULT_RUN_STORE));
        let mut catalog = None;
        while let Some(arg) = cursor.next() {
            match arg.as_str() {
                "--input" | "-i" => input = Some(cursor.required_path(&arg)?),
                "--max-iterations" => {
                    let value = cursor.required_value(&arg)?;
                    max_iterations = Some(
                        value
                            .parse::<u64>()
                            .map_err(|_| format!("{arg} requires an unsigned integer"))?,
                    );
                }
                "--store" => store = Some(cursor.required_path(&arg)?),
                "--no-store" => store = None,
                "--catalog" => catalog = Some(cursor.required_path(&arg)?),
                unknown => return Err(format!("unknown option `{unknown}`")),
            }
        }
        Ok(Self {
            input,
            max_iterations,
            store,
            catalog,
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct LoopReplayOptions {
    pub(super) trace_or_report: PathBuf,
}

impl LoopReplayOptions {
    pub(super) fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let Some(path) = cursor.next() else {
            return Err("loop replay requires <trace-or-report>".to_owned());
        };
        if cursor.next().is_some() {
            return Err("loop replay accepts exactly one <trace-or-report>".to_owned());
        }
        Ok(Self {
            trace_or_report: PathBuf::from(path),
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct LoopInspectOptions {
    pub(super) run_id: String,
    pub(super) store: PathBuf,
}

impl LoopInspectOptions {
    pub(super) fn parse(cursor: &mut ArgCursor) -> Result<Self, String> {
        let Some(run_id) = cursor.next() else {
            return Err("loop inspect requires <run-id>".to_owned());
        };
        let mut store = PathBuf::from(DEFAULT_RUN_STORE);
        while let Some(arg) = cursor.next() {
            match arg.as_str() {
                "--store" => store = cursor.required_path(&arg)?,
                unknown => return Err(format!("unknown option `{unknown}`")),
            }
        }
        Ok(Self { run_id, store })
    }
}
