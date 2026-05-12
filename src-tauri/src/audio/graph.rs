use std::collections::{HashMap, HashSet};

use serde::Deserialize;

use crate::error::{AppError, AppResult};

/// Payload received from the frontend. Mirrors `pipeline/types.ts`.
#[derive(Debug, Deserialize)]
pub struct GraphSpec {
    pub nodes: Vec<NodeSpec>,
    pub edges: Vec<EdgeSpec>,
}

#[derive(Debug, Deserialize)]
pub struct NodeSpec {
    pub id: String,
    pub kind: NodeKind,
    pub data: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct EdgeSpec {
    #[allow(dead_code)]
    pub id: String,
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    Microphone,
    SystemAudio,
    AppAudio,
    Speaker,
    FileRecording,
    Gain,
    Mute,
    ChannelBalance,
    Limiter,
    LevelMeter,
}

impl NodeKind {
    pub fn category(self) -> NodeCategory {
        match self {
            NodeKind::Microphone | NodeKind::SystemAudio | NodeKind::AppAudio => NodeCategory::Input,
            NodeKind::Speaker | NodeKind::FileRecording => NodeCategory::Output,
            NodeKind::Gain
            | NodeKind::Mute
            | NodeKind::ChannelBalance
            | NodeKind::Limiter
            | NodeKind::LevelMeter => NodeCategory::Effect,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeCategory {
    Input,
    Output,
    Effect,
}

// Per-kind typed data (snake_case after camelCase conversion via serde rename).

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneData {
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemAudioData {
    #[serde(default = "default_true")]
    pub exclude_current_app: bool,
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppAudioData {
    pub bundle_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerData {
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRecordingData {
    pub file_path: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GainData {
    pub gain_db: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MuteData {
    pub muted: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelBalanceData {
    pub left_gain_db: f32,
    pub right_gain_db: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimiterData {
    pub threshold_db: f32,
    pub drive_db: f32,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LevelMeterData {}

/// Typed input variant after validation.
#[derive(Debug, Clone)]
pub enum InputSpec {
    Microphone { device_id: String },
    SystemAudio { exclude_current_app: bool },
    AppAudio { bundle_id: String },
}

/// Typed output variant after validation.
#[derive(Debug, Clone)]
pub enum OutputSpec {
    Speaker { device_id: String },
    FileRecording { file_path: String },
}

/// Typed effect variant after validation. Effects are 1→1.
#[derive(Debug, Clone)]
pub enum EffectSpec {
    Gain(GainData),
    Mute(MuteData),
    ChannelBalance(ChannelBalanceData),
    Limiter(LimiterData),
    LevelMeter(LevelMeterData),
}

/// One effect occurrence in a chain: keeps the originating node id so live
/// parameter updates from the UI can be routed back to the right runtime
/// effect instance.
#[derive(Debug, Clone)]
pub struct EffectInstance {
    pub node_id: String,
    pub spec: EffectSpec,
}

/// A linear chain of effects between an input and an output. Order is from input → output.
#[derive(Debug, Clone)]
pub struct EffectChain(pub Vec<EffectInstance>);

#[derive(Debug, Clone)]
pub struct ValidInput {
    pub id: String,
    pub spec: InputSpec,
}

#[derive(Debug, Clone)]
pub struct ValidOutput {
    pub id: String,
    pub spec: OutputSpec,
}

/// One realized routing: a single input node feeds a single output node through
/// the linear chain of effects between them. Multiple bridges may share the
/// same input or the same output (mixing happens at the output).
#[derive(Debug, Clone)]
pub struct Bridge {
    pub input_id: String,
    pub output_id: String,
    pub effects: EffectChain,
}

#[derive(Debug)]
pub struct ValidGraph {
    pub inputs: Vec<ValidInput>,
    pub outputs: Vec<ValidOutput>,
    pub bridges: Vec<Bridge>,
}

impl GraphSpec {
    /// Validate the graph and resolve typed inputs/outputs/bridges.
    ///
    /// Rules:
    /// - Any number of inputs and outputs are allowed.
    /// - Disconnected inputs/outputs (no path to/from anything) are dropped, not rejected.
    /// - Effects must have at most one incoming and at most one outgoing edge (1→1).
    ///   Disconnected effects are simply dropped.
    /// - Cycles are rejected.
    pub fn validate(&self) -> AppResult<ValidGraph> {
        let nodes_by_id: HashMap<&str, &NodeSpec> =
            self.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

        // Build adjacency: outgoing and incoming.
        let mut outgoing: HashMap<&str, Vec<&str>> = HashMap::new();
        let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
        for edge in &self.edges {
            if !nodes_by_id.contains_key(edge.source.as_str())
                || !nodes_by_id.contains_key(edge.target.as_str())
            {
                return Err(AppError::Validation(format!(
                    "edge {} references unknown node",
                    edge.id
                )));
            }
            outgoing
                .entry(edge.source.as_str())
                .or_default()
                .push(edge.target.as_str());
            incoming
                .entry(edge.target.as_str())
                .or_default()
                .push(edge.source.as_str());
        }

        // Effects are 1→1: enforce.
        for n in &self.nodes {
            if n.kind.category() != NodeCategory::Effect {
                continue;
            }
            let inc = incoming.get(n.id.as_str()).map(Vec::len).unwrap_or(0);
            let out = outgoing.get(n.id.as_str()).map(Vec::len).unwrap_or(0);
            if inc > 1 {
                return Err(AppError::Validation(format!(
                    "effect {:?} has multiple incoming edges (must be 1→1)",
                    n.kind
                )));
            }
            if out > 1 {
                return Err(AppError::Validation(format!(
                    "effect {:?} has multiple outgoing edges (must be 1→1)",
                    n.kind
                )));
            }
        }

        check_acyclic(&self.nodes, &outgoing)?;

        // Resolve typed inputs & outputs that actually have connectivity.
        let inputs = self.resolve_inputs(&outgoing)?;
        let outputs = self.resolve_outputs(&incoming)?;

        // Build bridges: walk forward from each input through 1→1 effect chains
        // until reaching an output (or dead-end). Drop dead-ends silently.
        let mut bridges = Vec::new();
        for input in &inputs {
            walk_forward(
                input.id.as_str(),
                &nodes_by_id,
                &outgoing,
                &mut Vec::new(),
                &mut bridges,
            )?;
        }

        // Bridges may target outputs we kept; verify each bridge end is one of our outputs.
        let output_ids: HashSet<&str> = outputs.iter().map(|o| o.id.as_str()).collect();
        bridges.retain(|b| output_ids.contains(b.output_id.as_str()));

        // Sanity: if user has inputs + outputs but no bridges connect them, that's just
        // "nothing to do" — we don't error, we let the user keep building. The engine
        // will start with whatever bridges exist (potentially zero).

        Ok(ValidGraph {
            inputs,
            outputs,
            bridges,
        })
    }

    fn resolve_inputs(
        &self,
        outgoing: &HashMap<&str, Vec<&str>>,
    ) -> AppResult<Vec<ValidInput>> {
        let mut result = Vec::new();
        for n in &self.nodes {
            if n.kind.category() != NodeCategory::Input {
                continue;
            }
            // Drop fully disconnected inputs — not an error.
            if outgoing.get(n.id.as_str()).map(Vec::is_empty).unwrap_or(true)
                && !outgoing.contains_key(n.id.as_str())
            {
                continue;
            }
            let spec = match n.kind {
                NodeKind::Microphone => {
                    let data: MicrophoneData = parse(&n.data, "Microphone")?;
                    InputSpec::Microphone {
                        device_id: data
                            .device_id
                            .ok_or_else(|| miss(&n.id, "Microphone has no device selected"))?,
                    }
                }
                NodeKind::SystemAudio => {
                    let data: SystemAudioData = parse(&n.data, "SystemAudio")?;
                    InputSpec::SystemAudio {
                        exclude_current_app: data.exclude_current_app,
                    }
                }
                NodeKind::AppAudio => {
                    let data: AppAudioData = parse(&n.data, "AppAudio")?;
                    InputSpec::AppAudio {
                        bundle_id: data
                            .bundle_id
                            .ok_or_else(|| miss(&n.id, "App Audio has no application selected"))?,
                    }
                }
                _ => unreachable!(),
            };
            result.push(ValidInput {
                id: n.id.clone(),
                spec,
            });
        }
        Ok(result)
    }

    fn resolve_outputs(
        &self,
        incoming: &HashMap<&str, Vec<&str>>,
    ) -> AppResult<Vec<ValidOutput>> {
        let mut result = Vec::new();
        for n in &self.nodes {
            if n.kind.category() != NodeCategory::Output {
                continue;
            }
            if incoming.get(n.id.as_str()).map(Vec::is_empty).unwrap_or(true)
                && !incoming.contains_key(n.id.as_str())
            {
                continue;
            }
            let spec = match n.kind {
                NodeKind::Speaker => {
                    let data: SpeakerData = parse(&n.data, "Speaker")?;
                    OutputSpec::Speaker {
                        device_id: data
                            .device_id
                            .ok_or_else(|| miss(&n.id, "Speaker has no device selected"))?,
                    }
                }
                NodeKind::FileRecording => {
                    let data: FileRecordingData = parse(&n.data, "FileRecording")?;
                    OutputSpec::FileRecording {
                        file_path: data
                            .file_path
                            .ok_or_else(|| miss(&n.id, "File Recording has no path"))?,
                    }
                }
                _ => unreachable!(),
            };
            result.push(ValidOutput {
                id: n.id.clone(),
                spec,
            });
        }
        Ok(result)
    }
}

/// DFS forward from `current`, collecting effects, terminating at output nodes.
fn walk_forward<'a>(
    current: &'a str,
    nodes_by_id: &HashMap<&'a str, &'a NodeSpec>,
    outgoing: &HashMap<&'a str, Vec<&'a str>>,
    chain: &mut Vec<EffectInstance>,
    out: &mut Vec<Bridge>,
) -> AppResult<()> {
    let kids = match outgoing.get(current) {
        Some(v) => v,
        None => return Ok(()),
    };
    for &child in kids {
        let child_node = nodes_by_id[child];
        match child_node.kind.category() {
            NodeCategory::Output => {
                out.push(Bridge {
                    input_id: find_chain_root(current, nodes_by_id, outgoing).to_string(),
                    output_id: child.to_string(),
                    effects: EffectChain(chain.clone()),
                });
            }
            NodeCategory::Effect => {
                let spec = effect_from_node(child_node)?;
                chain.push(EffectInstance {
                    node_id: child.to_string(),
                    spec,
                });
                walk_forward(child, nodes_by_id, outgoing, chain, out)?;
                chain.pop();
            }
            NodeCategory::Input => {
                return Err(AppError::Validation(format!(
                    "edge points into an input node: {}",
                    child_node.id
                )));
            }
        }
    }
    Ok(())
}

/// Walk back through 1→1 effects to find the input node that started this path.
/// `from` must be reachable backwards-uniquely from an input (effects are 1→1).
fn find_chain_root<'a>(
    from: &'a str,
    nodes_by_id: &HashMap<&'a str, &'a NodeSpec>,
    outgoing: &HashMap<&'a str, Vec<&'a str>>,
) -> &'a str {
    let mut incoming: HashMap<&'a str, &'a str> = HashMap::new();
    for (&src, targets) in outgoing {
        for &t in targets {
            incoming.insert(t, src);
        }
    }
    let mut cur: &'a str = from;
    while let Some(&prev) = incoming.get(cur) {
        if nodes_by_id[prev].kind.category() == NodeCategory::Input {
            return prev;
        }
        cur = prev;
    }
    from
}

fn effect_from_node(n: &NodeSpec) -> AppResult<EffectSpec> {
    Ok(match n.kind {
        NodeKind::Gain => EffectSpec::Gain(parse(&n.data, "Gain")?),
        NodeKind::Mute => EffectSpec::Mute(parse(&n.data, "Mute")?),
        NodeKind::ChannelBalance => EffectSpec::ChannelBalance(parse(&n.data, "ChannelBalance")?),
        NodeKind::Limiter => EffectSpec::Limiter(parse(&n.data, "Limiter")?),
        NodeKind::LevelMeter => EffectSpec::LevelMeter(parse(&n.data, "LevelMeter")?),
        _ => unreachable!("non-effect kind passed to effect_from_node"),
    })
}

fn parse<T: for<'de> Deserialize<'de>>(value: &serde_json::Value, ctx: &str) -> AppResult<T> {
    serde_json::from_value::<T>(value.clone())
        .map_err(|e| AppError::Validation(format!("invalid {ctx} data: {e}")))
}

fn miss(node_id: &str, msg: &str) -> AppError {
    AppError::Validation(format!("{msg} (node {node_id})"))
}

fn check_acyclic(nodes: &[NodeSpec], outgoing: &HashMap<&str, Vec<&str>>) -> AppResult<()> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mark {
        Unseen,
        InProgress,
        Done,
    }
    let mut marks: HashMap<&str, Mark> = nodes.iter().map(|n| (n.id.as_str(), Mark::Unseen)).collect();
    for n in nodes {
        if marks[n.id.as_str()] == Mark::Unseen {
            visit(n.id.as_str(), outgoing, &mut marks)?;
        }
    }
    return Ok(());

    fn visit<'a>(
        cur: &'a str,
        outgoing: &HashMap<&str, Vec<&'a str>>,
        marks: &mut HashMap<&'a str, Mark>,
    ) -> AppResult<()> {
        match marks.get(cur).copied().unwrap_or(Mark::Unseen) {
            Mark::Done => return Ok(()),
            Mark::InProgress => {
                return Err(AppError::Validation(format!("cycle detected at node {cur}")));
            }
            Mark::Unseen => {}
        }
        marks.insert(cur, Mark::InProgress);
        if let Some(kids) = outgoing.get(cur) {
            for &k in kids {
                visit(k, outgoing, marks)?;
            }
        }
        marks.insert(cur, Mark::Done);
        Ok(())
    }
}
