//! Standing paired-run determinism regression gate.
//!
//! The parent process executes the same frozen harness binary twice with
//! isolated data and recorder directories, then compares the recorder's raw
//! ordered JSONL streams. Infrastructure failures are INVALID, never a green
//! determinism result.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const VERDICT_SCHEMA: &str = "bastion.determinism-regression.verdict/v1";
const MAX_RECORDER_SAMPLES: usize = 2_000_000;
const MAX_RECORDER_EVENTS: usize = 2_000_000;

#[derive(Clone, Debug)]
pub struct Config {
    pub scenario: String,
    pub seed: u32,
    pub ticks: u64,
    pub tps: f64,
    pub ladder_episode: Option<String>,
    pub output_dir: Option<PathBuf>,
    pub save_tree: Option<PathBuf>,
    pub normalizations: Vec<String>,
    pub timeout: Duration,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
struct NormalizationRecord {
    name: String,
    field: String,
    rationale: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ChildEvidence {
    label: String,
    command: Vec<String>,
    exit_code: Option<i32>,
    functional_pass: Option<bool>,
    functional_outcome_verified: bool,
    timed_out: bool,
    stdout: String,
    stderr: String,
    input_data_tree_sha256: Option<String>,
    recorder_metadata: String,
    artifact_verified: bool,
    seed_verified: bool,
    trajectory: TapeEvidence,
    events: TapeEvidence,
    authoritative_observation: TapeEvidence,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct TapeEvidence {
    path: String,
    raw_sha256: String,
    normalized_sha256: String,
    records: usize,
    truncated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct FirstDivergence {
    record_kind: String,
    record_index: usize,
    tick: Option<u64>,
    uid: Option<u64>,
    entity: Option<u64>,
    episode: Option<u64>,
    field: String,
    a: Value,
    b: Value,
    observed_writer: String,
    nearby_observed_writers: Vec<String>,
    writer_order_proven: bool,
    cross_stream_order_proven: bool,
    same_tick_alternates: Vec<SameTickAlternate>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct SameTickAlternate {
    record_kind: String,
    record_index: usize,
    field: String,
    a: Value,
    b: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Verdict {
    schema: &'static str,
    deterministic: bool,
    valid: bool,
    scenario: String,
    seed: u32,
    input_save_tree_sha256: Option<String>,
    artifact_sha256: String,
    comparison_scope: &'static str,
    normalizations: Vec<NormalizationRecord>,
    children: Vec<ChildEvidence>,
    functional_outcomes_match: bool,
    functional_children_ok: bool,
    gate_pass: bool,
    first_divergence: Option<FirstDivergence>,
    invalid_reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    B55Deep,
    B58LadderIntegration,
    Class7ItemIdentity,
    Class7AgentRoundtrip,
}

impl Scenario {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "b55-deep" => Ok(Self::B55Deep),
            "b58-ladder-integration-fixture" => Ok(Self::B58LadderIntegration),
            "class7-item-identity" => Ok(Self::Class7ItemIdentity),
            "class7-agent-roundtrip" => Ok(Self::Class7AgentRoundtrip),
            _ => Err(format!(
                "unknown determinism scenario {value:?}; expected b55-deep or \
                 b58-ladder-integration-fixture or class7-item-identity or class7-agent-roundtrip"
            )),
        }
    }

    fn child_args(self, config: &Config) -> Result<Vec<String>, String> {
        let mut args = vec![
            "--seed".into(),
            config.seed.to_string(),
            "--ticks".into(),
            config.ticks.to_string(),
            "--tps".into(),
            config.tps.to_string(),
        ];
        match self {
            Self::B55Deep => args.push("--b55-deep-scenario".into()),
            Self::B58LadderIntegration => {
                let episode = config.ladder_episode.as_deref().ok_or_else(|| {
                    "b58-ladder-integration-fixture requires --ladder-episode".to_owned()
                })?;
                args.extend([
                    "--b58-ladder-integration-fixture".into(),
                    "--ladder-episode".into(),
                    episode.into(),
                ]);
            },
            Self::Class7ItemIdentity => args.push("--class7-item-determinism-fixture".into()),
            Self::Class7AgentRoundtrip => args.push("--class7-agent-roundtrip-fixture".into()),
        }
        Ok(args)
    }

    fn outcome_markers(self, config: &Config) -> Result<(String, String), String> {
        match self {
            Self::B55Deep => Ok((
                "B5.5 DEEP SCENARIO: PASS".into(),
                "B5.5 DEEP SCENARIO: FAIL".into(),
            )),
            Self::B58LadderIntegration => {
                let episode = config.ladder_episode.as_deref().ok_or_else(|| {
                    "b58-ladder-integration-fixture requires --ladder-episode".to_owned()
                })?;
                Ok((
                    format!("M2-LADDER-EPISODE {episode}: PASS"),
                    format!("M2-LADDER-EPISODE {episode}: FAIL"),
                ))
            },
            Self::Class7ItemIdentity => Ok((
                "CLASS7 ITEM DETERMINISM FIXTURE: PASS".into(),
                "CLASS7 ITEM DETERMINISM FIXTURE: FAIL".into(),
            )),
            Self::Class7AgentRoundtrip => Ok((
                "CLASS7 AGENT ROUNDTRIP FIXTURE: PASS".into(),
                "CLASS7 AGENT ROUNDTRIP FIXTURE: FAIL".into(),
            )),
        }
    }

    fn uses_recorder(self) -> bool { self != Self::Class7ItemIdentity }

    fn delayed_recorder(self) -> bool { self == Self::B58LadderIntegration }

    fn uses_authoritative_observation(self) -> bool {
        matches!(self, Self::Class7ItemIdentity | Self::Class7AgentRoundtrip)
    }

    fn comparison_scope(self) -> &'static str {
        match (self.uses_recorder(), self.uses_authoritative_observation()) {
            (true, true) => {
                "flight-recorder trajectory/writer events plus authoritative class-7 round-trip \
                 observation"
            },
            (true, false) => "flight-recorder trajectory and writer-event streams",
            (false, true) => "authoritative class-7 inventory and UseItem selection observation",
            (false, false) => "no authoritative comparison stream",
        }
    }
}

#[derive(Clone, Debug)]
struct Normalizations {
    records: Vec<NormalizationRecord>,
    ignored_paths: BTreeSet<String>,
}

impl Normalizations {
    fn parse(names: &[String]) -> Result<Self, String> {
        let mut records = Vec::new();
        let mut ignored_paths = BTreeSet::new();
        for name in names {
            if name != "wall-unix-millis" {
                return Err(format!(
                    "normalization {name:?} is not allowed; only wall-unix-millis is orthogonal"
                ));
            }
            if ignored_paths.insert("$.wall_unix_millis".into()) {
                records.push(NormalizationRecord {
                    name: name.clone(),
                    field: "wall_unix_millis".into(),
                    rationale: "host wall clock is not simulation state or behavior".into(),
                });
            }
        }
        Ok(Self {
            records,
            ignored_paths,
        })
    }

    fn normalize(&self, value: &mut Value) { normalize_value(value, "$", &self.ignored_paths); }
}

pub fn run(config: Config) -> ExitCode {
    let mut config = config;
    let output_dir = config.output_dir.clone().unwrap_or_else(default_output_dir);
    config.output_dir = Some(output_dir.clone());
    let fallback = config.clone();
    match run_inner(config) {
        Ok((verdict, output_dir)) => {
            let code = if !verdict.valid {
                2
            } else if verdict.gate_pass {
                0
            } else if verdict.deterministic {
                3
            } else {
                1
            };
            if let Err(error) = write_verdict(&output_dir, &verdict) {
                eprintln!("DETERMINISM INVALID: failed to write verdict: {error}");
                return ExitCode::from(2);
            }
            print_human_summary(&verdict, &output_dir);
            ExitCode::from(code)
        },
        Err(error) => {
            if output_dir.join(".bastion-determinism-owned").is_file() {
                let artifact_sha256 = std::env::current_exe()
                    .ok()
                    .and_then(|path| hash_file(&path).ok())
                    .unwrap_or_default();
                let verdict = Verdict {
                    schema: VERDICT_SCHEMA,
                    deterministic: false,
                    valid: false,
                    scenario: fallback.scenario,
                    seed: fallback.seed,
                    input_save_tree_sha256: None,
                    artifact_sha256,
                    comparison_scope: "flight-recorder trajectory and writer-event streams",
                    normalizations: Vec::new(),
                    children: Vec::new(),
                    functional_outcomes_match: false,
                    functional_children_ok: false,
                    gate_pass: false,
                    first_divergence: None,
                    invalid_reasons: vec![error.clone()],
                };
                let _ = write_verdict(&output_dir, &verdict);
            }
            eprintln!("DETERMINISM INVALID: {error} ({})", output_dir.display());
            ExitCode::from(2)
        },
    }
}

fn run_inner(config: Config) -> Result<(Verdict, PathBuf), String> {
    let output_dir = config.output_dir.clone().expect("run assigns output dir");
    create_fresh_dir(&output_dir)?;
    let scenario = Scenario::parse(&config.scenario)?;
    let normalizations = Normalizations::parse(&config.normalizations)?;
    if config.save_tree.is_some() {
        return Err(format!(
            "scenario {:?} owns an internal temp data directory; --determinism-save-tree is not \
             consumed and is therefore rejected rather than overclaimed",
            config.scenario
        ));
    }
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let artifact_sha256 = hash_file(&executable)?;

    let input_save_tree_sha256 = None;

    let mut children = Vec::new();
    let mut invalid_reasons = Vec::new();
    for label in ["run-a", "run-b"] {
        let child_root = output_dir.join(label);
        let recorder_dir = child_root.join("recorder");
        fs::create_dir_all(&child_root).map_err(|error| error.to_string())?;
        let args = scenario.child_args(&config)?;
        let outcome_markers = scenario.outcome_markers(&config)?;
        let evidence = run_child(
            label,
            &executable,
            &artifact_sha256,
            &args,
            &child_root,
            &recorder_dir,
            config.seed,
            config.timeout,
            &normalizations,
            None,
            scenario,
            &outcome_markers,
        )?;
        invalid_reasons.extend(child_invalid_reasons(label, &evidence, scenario));
        children.push(evidence);
    }
    let artifact_after_children = hash_file(&executable)?;
    if artifact_after_children != artifact_sha256 {
        invalid_reasons.push(format!(
            "executable changed during paired run: {artifact_sha256} -> {artifact_after_children}"
        ));
    }

    let functional_outcomes_match = children
        .first()
        .zip(children.get(1))
        .is_some_and(|(a, b)| a.functional_pass == b.functional_pass && a.exit_code == b.exit_code);
    let first_divergence = if invalid_reasons.is_empty() {
        compare_child_tapes(&output_dir, &normalizations, scenario)?
            .or_else(|| (!functional_outcomes_match).then(|| child_outcome_divergence(&children)))
    } else {
        None
    };
    let valid = invalid_reasons.is_empty();
    let deterministic = valid && first_divergence.is_none();
    let functional_children_ok = children
        .iter()
        .all(|child| child.functional_pass == Some(true));
    let gate_pass = deterministic && functional_children_ok;
    Ok((
        Verdict {
            schema: VERDICT_SCHEMA,
            deterministic,
            valid,
            scenario: config.scenario,
            seed: config.seed,
            input_save_tree_sha256,
            artifact_sha256,
            comparison_scope: scenario.comparison_scope(),
            normalizations: normalizations.records,
            children,
            functional_outcomes_match,
            functional_children_ok,
            gate_pass,
            first_divergence,
            invalid_reasons,
        },
        output_dir,
    ))
}

fn child_invalid_reasons(label: &str, evidence: &ChildEvidence, scenario: Scenario) -> Vec<String> {
    let mut reasons = Vec::new();
    if evidence.timed_out {
        reasons.push(format!("{label}: child timed out"));
    }
    if evidence.exit_code.is_none() && !evidence.timed_out {
        reasons.push(format!(
            "{label}: child terminated without a numeric exit code"
        ));
    }
    if !evidence.functional_outcome_verified {
        reasons.push(format!(
            "{label}: missing, ambiguous, or exit-inconsistent structured scenario outcome"
        ));
    }
    if !evidence.artifact_verified {
        reasons.push(format!("{label}: artifact metadata mismatch"));
    }
    if !evidence.seed_verified {
        reasons.push(format!("{label}: seed metadata mismatch"));
    }
    let mut required_tapes = Vec::new();
    if scenario.uses_recorder() {
        required_tapes.extend([
            ("trajectory", &evidence.trajectory),
            ("events", &evidence.events),
        ]);
    }
    if scenario.uses_authoritative_observation() {
        required_tapes.push((
            "authoritative-observation",
            &evidence.authoritative_observation,
        ));
    }
    for (kind, tape) in required_tapes {
        if tape.path.is_empty() {
            reasons.push(format!("{label}: missing {kind} tape"));
        }
        if tape.truncated {
            reasons.push(format!("{label}: {kind} tape truncated"));
        }
        if kind == "authoritative-observation" && tape.records != 1 {
            reasons.push(format!(
                "{label}: authoritative observation must contain exactly one record, got {}",
                tape.records
            ));
        }
    }
    reasons
}

#[allow(clippy::too_many_arguments)]
fn run_child(
    label: &str,
    executable: &Path,
    artifact_sha256: &str,
    args: &[String],
    child_root: &Path,
    recorder_dir: &Path,
    seed: u32,
    timeout: Duration,
    normalizations: &Normalizations,
    input_data_tree_sha256: Option<String>,
    scenario: Scenario,
    outcome_markers: &(String, String),
) -> Result<ChildEvidence, String> {
    let stdout_path = child_root.join("stdout.log");
    let stderr_path = child_root.join("stderr.log");
    let observation_path = child_root.join("authoritative-observation.jsonl");
    let stdout = File::create(&stdout_path).map_err(|error| error.to_string())?;
    let stderr = File::create(&stderr_path).map_err(|error| error.to_string())?;
    let command_text = std::iter::once(executable.display().to_string())
        .chain(args.iter().cloned())
        .collect::<Vec<_>>();
    let mut command = Command::new(executable);
    command
        .args(args)
        .env("BASTION_FLIGHT_RECORDER_ARTIFACT_SHA256", artifact_sha256)
        .env("BASTION_FLIGHT_RECORDER_SEED", seed.to_string())
        .env("BASTION_FLIGHT_RECORDER_SESSION_ID", label)
        .env("BASTION_FLIGHT_RECORDER_COMMAND", command_text.join(" "))
        .env("BASTION_DETERMINISM_OBSERVATION_PATH", &observation_path)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    if scenario.delayed_recorder() {
        command.env("M2_RECORDER_DIR", recorder_dir);
    } else if scenario.uses_recorder() {
        command
            .env("BASTION_FLIGHT_RECORDER_DIR", recorder_dir)
            .env("BASTION_FLIGHT_RECORDER_SAMPLE_EVERY", "1")
            .env(
                "BASTION_FLIGHT_RECORDER_MAX_SAMPLES",
                MAX_RECORDER_SAMPLES.to_string(),
            )
            .env(
                "BASTION_FLIGHT_RECORDER_MAX_EVENTS",
                MAX_RECORDER_EVENTS.to_string(),
            );
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("{label}: spawn failed: {error}"))?;
    let start = Instant::now();
    let (exit_code, timed_out) = loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break (status.code(), false);
        }
        if start.elapsed() >= timeout {
            child.kill().map_err(|error| error.to_string())?;
            let _ = child.wait();
            break (None, true);
        }
        thread::sleep(Duration::from_millis(20));
    };
    let (functional_pass, functional_outcome_verified) = if timed_out {
        (None, false)
    } else {
        parse_functional_outcome(&stdout_path, exit_code, outcome_markers)?
    };
    let (
        trajectory,
        events,
        authoritative_observation,
        metadata_path,
        artifact_verified,
        seed_verified,
    ) = if scenario.uses_recorder() {
        let trajectory = inspect_tape(
            &recorder_dir.join("trajectory.jsonl"),
            &recorder_dir.join("summary.json"),
            "samples_written",
            "truncated_samples",
            normalizations,
        )?;
        let events = inspect_tape(
            &recorder_dir.join("events.jsonl"),
            &recorder_dir.join("summary.json"),
            "events_written",
            "truncated_events",
            normalizations,
        )?;
        let metadata_path = recorder_dir.join("metadata.json");
        let (recorder_artifact_verified, recorder_seed_verified) =
            verify_metadata(&metadata_path, executable, artifact_sha256, seed)?;
        let observation = if scenario.uses_authoritative_observation() {
            inspect_jsonl(&observation_path, normalizations)?
        } else {
            TapeEvidence::default()
        };
        let (observation_artifact_verified, observation_seed_verified) =
            if scenario.uses_authoritative_observation() {
                verify_observation(&observation_path, artifact_sha256, seed)?
            } else {
                (true, true)
            };
        (
            trajectory,
            events,
            observation,
            metadata_path,
            recorder_artifact_verified && observation_artifact_verified,
            recorder_seed_verified && observation_seed_verified,
        )
    } else {
        let observation = inspect_jsonl(&observation_path, normalizations)?;
        let (artifact_verified, seed_verified) =
            verify_observation(&observation_path, artifact_sha256, seed)?;
        (
            TapeEvidence::default(),
            TapeEvidence::default(),
            observation,
            observation_path.clone(),
            artifact_verified,
            seed_verified,
        )
    };
    Ok(ChildEvidence {
        label: label.into(),
        command: command_text,
        exit_code,
        functional_pass,
        functional_outcome_verified,
        timed_out,
        stdout: stdout_path.display().to_string(),
        stderr: stderr_path.display().to_string(),
        input_data_tree_sha256,
        recorder_metadata: metadata_path.display().to_string(),
        artifact_verified,
        seed_verified,
        trajectory,
        events,
        authoritative_observation,
    })
}

fn parse_functional_outcome(
    stdout_path: &Path,
    exit_code: Option<i32>,
    markers: &(String, String),
) -> Result<(Option<bool>, bool), String> {
    let stdout = fs::read_to_string(stdout_path).map_err(|error| error.to_string())?;
    let pass = stdout.lines().any(|line| line.trim() == markers.0);
    let fail = stdout.lines().any(|line| line.trim() == markers.1);
    let outcome = match (pass, fail) {
        (true, false) => Some(true),
        (false, true) => Some(false),
        _ => None,
    };
    let exit_consistent = match outcome {
        Some(true) => exit_code == Some(0),
        Some(false) => exit_code.is_some_and(|code| code != 0 && code != 101),
        None => false,
    };
    Ok((outcome, exit_consistent))
}

fn verify_metadata(
    path: &Path,
    executable: &Path,
    artifact_sha256: &str,
    seed: u32,
) -> Result<(bool, bool), String> {
    if !path.is_file() {
        return Ok((false, false));
    }
    let metadata: Value =
        serde_json::from_reader(File::open(path).map_err(|error| error.to_string())?)
            .map_err(|error| format!("{}: {error}", path.display()))?;
    let recorded_executable = metadata["executable"].as_str().map(PathBuf::from);
    let executable_matches = recorded_executable
        .as_deref()
        .is_some_and(|recorded| paths_equal(recorded, executable));
    let hash_matches = metadata["artifact_sha256"].as_str() == Some(artifact_sha256);
    let seed_text = seed.to_string();
    let seed_matches = metadata["seed"].as_str() == Some(seed_text.as_str());
    Ok((executable_matches && hash_matches, seed_matches))
}

fn child_outcome_divergence(children: &[ChildEvidence]) -> FirstDivergence {
    let a = children.first().and_then(|child| child.exit_code);
    let b = children.get(1).and_then(|child| child.exit_code);
    FirstDivergence {
        record_kind: "child-outcome".into(),
        record_index: 0,
        tick: None,
        uid: None,
        entity: None,
        episode: None,
        field: "$.exit_code".into(),
        a: a.map(Value::from).unwrap_or(Value::Null),
        b: b.map(Value::from).unwrap_or(Value::Null),
        observed_writer: "not-observable-from-recorder".into(),
        nearby_observed_writers: Vec::new(),
        writer_order_proven: false,
        cross_stream_order_proven: true,
        same_tick_alternates: Vec::new(),
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn inspect_tape(
    path: &Path,
    summary_path: &Path,
    count_key: &str,
    truncated_key: &str,
    normalizations: &Normalizations,
) -> Result<TapeEvidence, String> {
    if !path.is_file() || !summary_path.is_file() {
        return Ok(TapeEvidence::default());
    }
    let summary: Value =
        serde_json::from_reader(File::open(summary_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    let expected_count = summary[count_key]
        .as_u64()
        .ok_or_else(|| format!("missing {count_key} in {}", summary_path.display()))?
        as usize;
    let truncated = summary[truncated_key]
        .as_bool()
        .ok_or_else(|| format!("missing {truncated_key} in {}", summary_path.display()))?;
    let records = read_tape(path)?;
    if records.len() != expected_count {
        return Err(format!(
            "{} contains {} records but summary reports {}",
            path.display(),
            records.len(),
            expected_count
        ));
    }
    let mut normalized_hasher = Sha256::new();
    for mut value in records {
        normalizations.normalize(&mut value);
        normalized_hasher.update(serde_json::to_vec(&value).map_err(|error| error.to_string())?);
        normalized_hasher.update(b"\n");
    }
    Ok(TapeEvidence {
        path: path.display().to_string(),
        raw_sha256: hash_file(path)?,
        normalized_sha256: hex::encode(normalized_hasher.finalize()),
        records: expected_count,
        truncated,
    })
}

fn inspect_jsonl(path: &Path, normalizations: &Normalizations) -> Result<TapeEvidence, String> {
    if !path.is_file() {
        return Ok(TapeEvidence::default());
    }
    let records = read_tape(path)?;
    let mut normalized_hasher = Sha256::new();
    for mut value in records.iter().cloned() {
        normalizations.normalize(&mut value);
        normalized_hasher.update(serde_json::to_vec(&value).map_err(|error| error.to_string())?);
        normalized_hasher.update(b"\n");
    }
    Ok(TapeEvidence {
        path: path.display().to_string(),
        raw_sha256: hash_file(path)?,
        normalized_sha256: hex::encode(normalized_hasher.finalize()),
        records: records.len(),
        truncated: false,
    })
}

fn verify_observation(
    path: &Path,
    artifact_sha256: &str,
    seed: u32,
) -> Result<(bool, bool), String> {
    if !path.is_file() {
        return Ok((false, false));
    }
    let records = read_tape(path)?;
    let Some(value) = records.first() else {
        return Ok((false, false));
    };
    let artifact_matches = value["artifact_sha256"].as_str() == Some(artifact_sha256);
    let seed_text = seed.to_string();
    let seed_matches = value["seed"].as_str() == Some(seed_text.as_str());
    Ok((artifact_matches, seed_matches))
}

fn compare_child_tapes(
    output_dir: &Path,
    normalizations: &Normalizations,
    scenario: Scenario,
) -> Result<Option<FirstDivergence>, String> {
    let mut candidates = Vec::new();
    let mut streams = Vec::new();
    if scenario.uses_recorder() {
        streams.extend([
            ("sample", "trajectory.jsonl", true),
            ("writer-event", "events.jsonl", true),
        ]);
    }
    if scenario.uses_authoritative_observation() {
        streams.push((
            "authoritative-observation",
            "authoritative-observation.jsonl",
            false,
        ));
    }
    for (kind, file, recorder) in streams {
        let a_path = if recorder {
            output_dir.join("run-a/recorder").join(file)
        } else {
            output_dir.join("run-a").join(file)
        };
        let b_path = if recorder {
            output_dir.join("run-b/recorder").join(file)
        } else {
            output_dir.join("run-b").join(file)
        };
        if let Some(divergence) = compare_stream(kind, &a_path, &b_path, normalizations)? {
            candidates.push(divergence);
        }
    }
    candidates.sort_by_key(|divergence| {
        (
            divergence.tick.unwrap_or(u64::MAX),
            divergence.record_kind.clone(),
            divergence.record_index,
        )
    });
    if candidates.is_empty() {
        return Ok(None);
    }
    let mut first = candidates.remove(0);
    let earliest_tick = first.tick;
    let same_tick = candidates
        .into_iter()
        .filter(|candidate| candidate.tick == earliest_tick)
        .collect::<Vec<_>>();
    first.cross_stream_order_proven = same_tick.is_empty();
    first.same_tick_alternates = same_tick
        .into_iter()
        .map(|candidate| SameTickAlternate {
            record_kind: candidate.record_kind,
            record_index: candidate.record_index,
            field: candidate.field,
            a: candidate.a,
            b: candidate.b,
        })
        .collect();
    let nearby = nearby_writers(output_dir, first.tick, first.uid)?;
    first.nearby_observed_writers = nearby;
    Ok(Some(first))
}

fn compare_stream(
    kind: &str,
    a_path: &Path,
    b_path: &Path,
    normalizations: &Normalizations,
) -> Result<Option<FirstDivergence>, String> {
    let a = read_tape(a_path)?;
    let b = read_tape(b_path)?;
    for index in 0..a.len().max(b.len()) {
        let (Some(mut av), Some(mut bv)) = (a.get(index).cloned(), b.get(index).cloned()) else {
            let av = a.get(index).cloned().unwrap_or(Value::Null);
            let bv = b.get(index).cloned().unwrap_or(Value::Null);
            let source = if av.is_object() { &av } else { &bv };
            let result = divergence(kind, index, "$length", av.clone(), bv.clone(), source);
            return Ok(Some(result));
        };
        normalizations.normalize(&mut av);
        normalizations.normalize(&mut bv);
        if av != bv {
            let (field, left, right) = first_json_difference(&av, &bv, "$")
                .unwrap_or_else(|| ("$".into(), av.clone(), bv.clone()));
            let mut result = divergence(kind, index, &field, left, right, &av);
            populate_writer_observation(&mut result, &av, &bv);
            return Ok(Some(result));
        }
    }
    Ok(None)
}

fn divergence(
    kind: &str,
    index: usize,
    field: &str,
    a: Value,
    b: Value,
    source: &Value,
) -> FirstDivergence {
    FirstDivergence {
        record_kind: kind.into(),
        record_index: index,
        tick: source.get("tick").and_then(Value::as_u64),
        uid: source.get("uid").and_then(Value::as_u64),
        entity: source.get("entity").and_then(Value::as_u64),
        episode: source.get("episode").and_then(Value::as_u64),
        field: field.into(),
        a,
        b,
        observed_writer: "unknown".into(),
        nearby_observed_writers: Vec::new(),
        writer_order_proven: false,
        cross_stream_order_proven: false,
        same_tick_alternates: Vec::new(),
    }
}

fn populate_writer_observation(result: &mut FirstDivergence, a: &Value, b: &Value) {
    if result.record_kind != "writer-event" {
        return;
    }
    let a_proven = a["dispatcher_dependency_proven"].as_bool() == Some(true);
    let b_proven = b["dispatcher_dependency_proven"].as_bool() == Some(true);
    let a_writer = a["writer"].as_str();
    let b_writer = b["writer"].as_str();
    if a_proven && b_proven && a_writer.is_some() && a_writer == b_writer {
        result.observed_writer = a_writer.unwrap_or("unknown").to_owned();
        result.writer_order_proven = true;
    }
}

fn first_json_difference(a: &Value, b: &Value, path: &str) -> Option<(String, Value, Value)> {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            let keys: BTreeSet<_> = a.keys().chain(b.keys()).collect();
            for key in keys {
                let child_path = format!("{path}.{key}");
                match (a.get(key), b.get(key)) {
                    (Some(left), Some(right)) => {
                        if let Some(diff) = first_json_difference(left, right, &child_path) {
                            return Some(diff);
                        }
                    },
                    (left, right) => {
                        return Some((
                            child_path,
                            left.cloned().unwrap_or(Value::Null),
                            right.cloned().unwrap_or(Value::Null),
                        ));
                    },
                }
            }
            None
        },
        (Value::Array(a), Value::Array(b)) => {
            for index in 0..a.len().max(b.len()) {
                let child_path = format!("{path}[{index}]");
                match (a.get(index), b.get(index)) {
                    (Some(left), Some(right)) => {
                        if let Some(diff) = first_json_difference(left, right, &child_path) {
                            return Some(diff);
                        }
                    },
                    (left, right) => {
                        return Some((
                            child_path,
                            left.cloned().unwrap_or(Value::Null),
                            right.cloned().unwrap_or(Value::Null),
                        ));
                    },
                }
            }
            None
        },
        _ if a == b => None,
        _ => Some((path.into(), a.clone(), b.clone())),
    }
}

fn normalize_value(value: &mut Value, path: &str, ignored: &BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                if ignored.contains(&child_path) {
                    *child = Value::String("<normalized>".into());
                } else {
                    normalize_value(child, &child_path, ignored);
                }
            }
        },
        Value::Array(values) => {
            for (index, child) in values.iter_mut().enumerate() {
                normalize_value(child, &format!("{path}[{index}]"), ignored);
            }
        },
        _ => {},
    }
}

fn nearby_writers(
    output_dir: &Path,
    tick: Option<u64>,
    uid: Option<u64>,
) -> Result<Vec<String>, String> {
    let (Some(tick), Some(uid)) = (tick, uid) else {
        return Ok(Vec::new());
    };
    let mut writers = BTreeSet::new();
    for label in ["run-a", "run-b"] {
        let path = output_dir.join(label).join("recorder/events.jsonl");
        for event in read_tape(&path)? {
            let event_tick = event["tick"].as_u64();
            let event_uid = event["uid"].as_u64();
            if event_uid == Some(uid)
                && event_tick.is_some_and(|candidate| candidate.abs_diff(tick) <= 1)
                && let Some(writer) = event["writer"].as_str()
            {
                writers.insert(writer.to_owned());
            }
        }
    }
    Ok(writers.into_iter().collect())
}

fn read_tape(path: &Path) -> Result<Vec<Value>, String> {
    let file = File::open(path).map_err(|error| format!("{}: {error}", path.display()))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let line = line.map_err(|error| error.to_string())?;
            serde_json::from_str(&line)
                .map_err(|error| format!("{} line {}: {error}", path.display(), index + 1))
        })
        .collect()
}

fn write_verdict(output_dir: &Path, verdict: &Verdict) -> Result<(), String> {
    let verdict_path = output_dir.join("verdict.json");
    serde_json::to_writer_pretty(
        File::create(&verdict_path).map_err(|error| error.to_string())?,
        verdict,
    )
    .map_err(|error| error.to_string())?;
    let mut summary = File::create(output_dir.join("summary.txt")).map_err(|e| e.to_string())?;
    writeln!(summary, "valid={}", verdict.valid).map_err(|e| e.to_string())?;
    writeln!(summary, "deterministic={}", verdict.deterministic).map_err(|e| e.to_string())?;
    writeln!(summary, "scenario={}", verdict.scenario).map_err(|e| e.to_string())?;
    writeln!(summary, "seed={}", verdict.seed).map_err(|e| e.to_string())?;
    writeln!(
        summary,
        "functional_outcomes_match={}",
        verdict.functional_outcomes_match
    )
    .map_err(|e| e.to_string())?;
    writeln!(
        summary,
        "functional_children_ok={}",
        verdict.functional_children_ok
    )
    .map_err(|e| e.to_string())?;
    writeln!(summary, "gate_pass={}", verdict.gate_pass).map_err(|e| e.to_string())?;
    for child in &verdict.children {
        writeln!(
            summary,
            "{}: exit={:?} functional_pass={:?} outcome_verified={} timed_out={}",
            child.label,
            child.exit_code,
            child.functional_pass,
            child.functional_outcome_verified,
            child.timed_out
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(first) = &verdict.first_divergence {
        writeln!(
            summary,
            "first_divergence=tick={:?} uid={:?} kind={} index={} field={} observed_writer={} \
             order_proven={} cross_stream_order_proven={}",
            first.tick,
            first.uid,
            first.record_kind,
            first.record_index,
            first.field,
            first.observed_writer,
            first.writer_order_proven,
            first.cross_stream_order_proven
        )
        .map_err(|e| e.to_string())?;
        writeln!(summary, "lhs={}", first.a).map_err(|e| e.to_string())?;
        writeln!(summary, "rhs={}", first.b).map_err(|e| e.to_string())?;
        for alternate in &first.same_tick_alternates {
            writeln!(
                summary,
                "same_tick_alternate=kind={} index={} field={} lhs={} rhs={}",
                alternate.record_kind,
                alternate.record_index,
                alternate.field,
                alternate.a,
                alternate.b
            )
            .map_err(|e| e.to_string())?;
        }
    }
    for reason in &verdict.invalid_reasons {
        writeln!(summary, "invalid_reason={reason}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn print_human_summary(verdict: &Verdict, output_dir: &Path) {
    if !verdict.valid {
        eprintln!(
            "DETERMINISM INVALID: {} ({})",
            verdict.invalid_reasons.join("; "),
            output_dir.display()
        );
    } else if let Some(first) = &verdict.first_divergence {
        eprintln!(
            "DETERMINISM DIVERGED: tick={:?} uid={:?} {}[{}] field={} observed_writer={} \
             cross_stream_order_proven={} ({})",
            first.tick,
            first.uid,
            first.record_kind,
            first.record_index,
            first.field,
            first.observed_writer,
            first.cross_stream_order_proven,
            output_dir.display()
        );
    } else if verdict.functional_children_ok {
        eprintln!(
            "DETERMINISM OK: scenario={} seed={} ({})",
            verdict.scenario,
            verdict.seed,
            output_dir.display()
        );
    } else {
        eprintln!(
            "DETERMINISM OK; FUNCTIONAL SCENARIO FAIL: scenario={} seed={} exits={:?} ({})",
            verdict.scenario,
            verdict.seed,
            verdict
                .children
                .iter()
                .map(|child| child.exit_code)
                .collect::<Vec<_>>(),
            output_dir.display()
        );
    }
}

fn create_fresh_dir(path: &Path) -> Result<(), String> {
    if path.exists() {
        return Err(format!(
            "output directory {} already exists; evidence overwrite is forbidden",
            path.display()
        ));
    }
    fs::create_dir_all(path).map_err(|error| error.to_string())?;
    File::create(path.join(".bastion-determinism-owned")).map_err(|error| error.to_string())?;
    Ok(())
}

fn default_output_dir() -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    std::env::temp_dir().join(format!(
        "bastion-determinism-{}-{millis}",
        std::process::id()
    ))
}

#[cfg(test)]
fn hash_tree(root: &Path) -> Result<String, String> {
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (relative, path) in files {
        let relative = relative.to_string_lossy().replace('\\', "/");
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        let mut file = File::open(path).map_err(|error| error.to_string())?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(PathBuf, PathBuf)>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_dir() {
            collect_files(root, &path, files)?;
        } else if file_type.is_file() {
            files.push((
                path.strip_prefix(root)
                    .map_err(|error| error.to_string())?
                    .to_path_buf(),
                path,
            ));
        } else {
            return Err(format!("unsupported tree entry {}", path.display()));
        }
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(tick: u64, x: f64) -> Value {
        serde_json::json!({
            "schema": "bastion.flight-recorder.sample/v1",
            "tick": tick,
            "uid": 7,
            "entity": 3,
            "episode": 1,
            "position": [x, 2.0, 3.0],
            "wall_unix_millis": 1000 + tick,
            "movement_writer": "agent"
        })
    }

    fn write_tape(path: &Path, values: &[Value]) {
        let mut file = File::create(path).unwrap();
        for value in values {
            serde_json::to_writer(&mut file, value).unwrap();
            writeln!(file).unwrap();
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "bastion-determinism-test-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn identical_and_wall_clock_normalized_tapes_are_deterministic() {
        let dir = temp_dir("identical");
        let a = dir.join("a.jsonl");
        let b = dir.join("b.jsonl");
        let mut right = sample(10, 1.0);
        right["wall_unix_millis"] = Value::from(9999_u64);
        write_tape(&a, &[sample(10, 1.0)]);
        write_tape(&b, &[right]);
        let normalizations = Normalizations::parse(&["wall-unix-millis".into()]).unwrap();
        assert!(
            compare_stream("sample", &a, &b, &normalizations)
                .unwrap()
                .is_none()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn first_field_length_and_order_divergences_are_reported() {
        let dir = temp_dir("divergence");
        let a = dir.join("a.jsonl");
        let b = dir.join("b.jsonl");
        let none = Normalizations::parse(&[]).unwrap();

        write_tape(&a, &[sample(10, 1.0)]);
        write_tape(&b, &[sample(10, 2.0)]);
        let diff = compare_stream("sample", &a, &b, &none).unwrap().unwrap();
        assert_eq!(diff.tick, Some(10));
        assert_eq!(diff.field, "$.position[0]");

        write_tape(&a, &[sample(10, 1.0), sample(11, 2.0)]);
        write_tape(&b, &[sample(10, 1.0)]);
        assert_eq!(
            compare_stream("sample", &a, &b, &none)
                .unwrap()
                .unwrap()
                .field,
            "$length"
        );

        write_tape(&a, &[sample(10, 1.0), sample(11, 2.0)]);
        write_tape(&b, &[sample(11, 2.0), sample(10, 1.0)]);
        let order = compare_stream("sample", &a, &b, &none).unwrap().unwrap();
        assert_eq!(order.record_index, 0);
        assert_eq!(order.field, "$.position[0]");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn behavioral_and_unknown_normalizations_are_rejected() {
        for name in ["position", "item_hash", "inv_slot", "anything-else"] {
            assert!(Normalizations::parse(&[name.into()]).is_err());
        }
    }

    #[test]
    fn tree_hash_is_order_stable_and_content_sensitive() {
        let a = temp_dir("tree-a");
        let b = temp_dir("tree-b");
        fs::write(a.join("z"), b"two").unwrap();
        fs::write(a.join("a"), b"one").unwrap();
        fs::write(b.join("a"), b"one").unwrap();
        fs::write(b.join("z"), b"two").unwrap();
        assert_eq!(hash_tree(&a).unwrap(), hash_tree(&b).unwrap());
        fs::write(b.join("z"), b"changed").unwrap();
        assert_ne!(hash_tree(&a).unwrap(), hash_tree(&b).unwrap());
        fs::remove_dir_all(a).unwrap();
        fs::remove_dir_all(b).unwrap();
    }

    #[test]
    fn output_directory_must_be_fresh() {
        let dir = temp_dir("fresh");
        assert!(create_fresh_dir(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn structured_outcome_rejects_panics_and_accepts_explicit_failure() {
        let dir = temp_dir("outcome");
        let stdout = dir.join("stdout.log");
        let markers = ("SCENARIO: PASS".into(), "SCENARIO: FAIL".into());
        fs::write(&stdout, "SCENARIO: FAIL\n").unwrap();
        assert_eq!(
            parse_functional_outcome(&stdout, Some(1), &markers).unwrap(),
            (Some(false), true)
        );
        fs::write(&stdout, "thread panicked at example\n").unwrap();
        assert_eq!(
            parse_functional_outcome(&stdout, Some(101), &markers).unwrap(),
            (None, false)
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn timeout_missing_and_truncated_evidence_are_invalid() {
        let full = TapeEvidence {
            path: "present".into(),
            raw_sha256: "raw".into(),
            normalized_sha256: "normalized".into(),
            records: 1,
            truncated: false,
        };
        let evidence = ChildEvidence {
            label: "run-a".into(),
            command: Vec::new(),
            exit_code: None,
            functional_pass: None,
            functional_outcome_verified: false,
            timed_out: true,
            stdout: String::new(),
            stderr: String::new(),
            input_data_tree_sha256: None,
            recorder_metadata: String::new(),
            artifact_verified: true,
            seed_verified: true,
            trajectory: TapeEvidence {
                truncated: true,
                ..full.clone()
            },
            events: TapeEvidence::default(),
            authoritative_observation: TapeEvidence::default(),
        };
        let reasons = child_invalid_reasons("run-a", &evidence, Scenario::B55Deep);
        assert!(reasons.iter().any(|reason| reason.contains("timed out")));
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("trajectory tape truncated"))
        );
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("missing events tape"))
        );
    }

    #[test]
    fn agent_roundtrip_requires_recorder_and_authoritative_observation() {
        let full = TapeEvidence {
            path: "present".into(),
            raw_sha256: "raw".into(),
            normalized_sha256: "normalized".into(),
            records: 1,
            truncated: false,
        };
        let evidence = ChildEvidence {
            label: "run-a".into(),
            command: Vec::new(),
            exit_code: Some(0),
            functional_pass: Some(true),
            functional_outcome_verified: true,
            timed_out: false,
            stdout: String::new(),
            stderr: String::new(),
            input_data_tree_sha256: None,
            recorder_metadata: String::new(),
            artifact_verified: true,
            seed_verified: true,
            trajectory: full.clone(),
            events: full,
            authoritative_observation: TapeEvidence::default(),
        };
        let reasons = child_invalid_reasons("run-a", &evidence, Scenario::Class7AgentRoundtrip);
        assert_eq!(reasons.len(), 2);
        assert!(
            reasons
                .iter()
                .any(|reason| reason.contains("missing authoritative-observation tape"))
        );
        assert!(reasons.iter().any(|reason| {
            reason.contains("authoritative observation must contain exactly one record")
        }));
    }

    #[test]
    fn normalization_is_exact_not_recursive() {
        let dir = temp_dir("normalization-path");
        let a = dir.join("a.jsonl");
        let b = dir.join("b.jsonl");
        let mut left = sample(1, 1.0);
        let mut right = left.clone();
        left["nested"] = serde_json::json!({"wall_unix_millis": 10});
        right["nested"] = serde_json::json!({"wall_unix_millis": 11});
        write_tape(&a, &[left]);
        write_tape(&b, &[right]);
        let normalizations = Normalizations::parse(&["wall-unix-millis".into()]).unwrap();
        assert_eq!(
            compare_stream("sample", &a, &b, &normalizations)
                .unwrap()
                .unwrap()
                .field,
            "$.nested.wall_unix_millis"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn same_tick_cross_stream_divergence_is_reported_as_ambiguous() {
        let dir = temp_dir("same-tick");
        for label in ["run-a", "run-b"] {
            fs::create_dir_all(dir.join(label).join("recorder")).unwrap();
        }
        write_tape(&dir.join("run-a/recorder/trajectory.jsonl"), &[sample(
            10, 1.0,
        )]);
        write_tape(&dir.join("run-b/recorder/trajectory.jsonl"), &[sample(
            10, 2.0,
        )]);
        let writer = |value: i64| {
            serde_json::json!({
                "tick": 10,
                "uid": 7,
                "writer": "agent",
                "dispatcher_dependency_proven": true,
                "input": value,
            })
        };
        write_tape(&dir.join("run-a/recorder/events.jsonl"), &[writer(1)]);
        write_tape(&dir.join("run-b/recorder/events.jsonl"), &[writer(2)]);
        let first = compare_child_tapes(
            &dir,
            &Normalizations::parse(&[]).unwrap(),
            Scenario::B55Deep,
        )
        .unwrap()
        .unwrap();
        assert!(!first.cross_stream_order_proven);
        assert_eq!(first.same_tick_alternates.len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn authoritative_observation_metadata_mismatch_is_rejected() {
        let dir = temp_dir("observation-metadata");
        let path = dir.join("observation.jsonl");
        write_tape(&path, &[serde_json::json!({
            "artifact_sha256": "wrong",
            "seed": "9",
            "result": {"inventory": []}
        })]);
        assert_eq!(
            verify_observation(&path, "expected", 9).unwrap(),
            (false, true)
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
