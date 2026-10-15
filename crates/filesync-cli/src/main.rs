//! `filesync`: the Deevnet FileSync command-line interface (prototype).
//!
//! Configuration lives in `$XDG_CONFIG_HOME/deevnet-filesync/` and per-mount state in
//! `$XDG_STATE_HOME/deevnet-filesync/`, never inside a synced folder (ROOT-6).
//! `FILESYNC_CONFIG_DIR` and `FILESYNC_STATE_DIR` override them, which is how one machine can
//! act as several devices for testing.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use filesync_core::backend::{Backend, DirBackend};
use filesync_core::crypto::Keys;
use filesync_core::engine::{self, MountState, Options, Report};
use filesync_core::hub::Hub;
use filesync_core::s3::{S3Backend, S3Config};
use filesync_core::util;

#[derive(Parser)]
#[command(name = "filesync", version, about = "Deevnet FileSync (prototype)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Set up this device: name it, point it at a hub, and create or import the hub key.
    Init {
        /// This device's name, shown in history and conflict copies.
        #[arg(long)]
        device: String,
        /// Use a directory as the hub (for testing, or a mounted NAS).
        #[arg(long, conflicts_with_all = ["s3_endpoint", "bucket"])]
        hub_dir: Option<PathBuf>,
        /// S3-compatible endpoint, e.g. https://s3.us-east-1.amazonaws.com or a MinIO URL.
        #[arg(long, requires = "bucket")]
        s3_endpoint: Option<String>,
        #[arg(long)]
        bucket: Option<String>,
        #[arg(long, default_value = "us-east-1")]
        region: String,
        /// Create the bucket if it does not exist.
        #[arg(long)]
        create_bucket: bool,
        /// Join an existing hub using a copy of its key file, instead of creating a new key.
        #[arg(long)]
        join: Option<PathBuf>,
    },
    /// Manage roots: the named collections of files that sync.
    #[command(subcommand)]
    Root(RootCmd),
    /// Mount a root at a local folder on this device.
    Mount {
        root: String,
        path: PathBuf,
        /// Download changes but never upload (ROOT-9).
        #[arg(long)]
        receive_only: bool,
    },
    /// Stop syncing a root on this device. Local files are left in place.
    Unmount { root: String },
    /// List this device's mounts.
    Mounts,
    /// Show what a sync would do, without doing it.
    Status { root: Option<String> },
    /// Upload local changes and download remote ones ("push now").
    Sync {
        root: Option<String>,
        #[arg(long)]
        allow_mass_delete: bool,
    },
    /// Download remote changes only ("pull now").
    Pull {
        root: Option<String>,
        #[arg(long)]
        allow_mass_delete: bool,
    },
    /// List a root's snapshots: its version history.
    History { root: String },
    /// Restore a root, or part of it, as it was at a snapshot, into a new folder.
    Restore {
        root: String,
        #[arg(long)]
        seq: u64,
        /// Only this file or folder within the root.
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        to: PathBuf,
    },
    /// Sync every mount, then repeat every INTERVAL seconds.
    Run {
        #[arg(long, default_value_t = 120)]
        interval: u64,
    },
}

#[derive(Subcommand)]
enum RootCmd {
    Create { name: String },
    List,
}

#[derive(Serialize, Deserialize)]
struct Config {
    device: String,
    hub: HubConfig,
    #[serde(default)]
    limits: Limits,
    #[serde(default, rename = "mount")]
    mounts: Vec<Mount>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum HubConfig {
    Dir {
        path: PathBuf,
    },
    S3 {
        endpoint: String,
        bucket: String,
        region: String,
    },
}

#[derive(Serialize, Deserialize)]
struct Limits {
    max_file_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        // WKL-2: sized for documents and spreadsheets, not media libraries.
        Self {
            max_file_bytes: 100 * 1024 * 1024,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct Mount {
    root_id: String,
    name: String,
    path: PathBuf,
    #[serde(default)]
    receive_only: bool,
}

#[derive(Deserialize)]
struct Credentials {
    access_key_id: String,
    secret_access_key: String,
}

fn xdg_dir(override_var: &str, xdg_var: &str, fallback: &str) -> Result<PathBuf> {
    if let Some(p) = std::env::var_os(override_var) {
        return Ok(PathBuf::from(p));
    }
    let base = match std::env::var_os(xdg_var) {
        Some(p) => PathBuf::from(p),
        None => PathBuf::from(std::env::var_os("HOME").context("HOME is not set")?).join(fallback),
    };
    Ok(base.join("deevnet-filesync"))
}

fn config_dir() -> Result<PathBuf> {
    xdg_dir("FILESYNC_CONFIG_DIR", "XDG_CONFIG_HOME", ".config")
}

fn state_dir() -> Result<PathBuf> {
    xdg_dir("FILESYNC_STATE_DIR", "XDG_STATE_HOME", ".local/state")
}

/// Writes a file readable only by its owner.
fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    util::write_atomic(path, data)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn load_config() -> Result<Config> {
    let path = config_dir()?.join("config.toml");
    let text = fs::read_to_string(&path)
        .with_context(|| format!("reading {} (run `filesync init` first)", path.display()))?;
    Ok(toml::from_str(&text)?)
}

fn save_config(cfg: &Config) -> Result<()> {
    write_private(
        &config_dir()?.join("config.toml"),
        toml::to_string_pretty(cfg)?.as_bytes(),
    )
}

fn load_key() -> Result<[u8; 32]> {
    let path = config_dir()?.join("hub.key");
    read_key(&path)
}

fn read_key(path: &Path) -> Result<[u8; 32]> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let bytes = hex::decode(text.trim()).context("hub key is not hex")?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("hub key must be 32 bytes"))
}

fn credentials() -> Result<Credentials> {
    if let (Ok(id), Ok(secret)) = (
        std::env::var("FILESYNC_S3_ACCESS_KEY_ID"),
        std::env::var("FILESYNC_S3_SECRET_ACCESS_KEY"),
    ) {
        return Ok(Credentials {
            access_key_id: id,
            secret_access_key: secret,
        });
    }
    let path = config_dir()?.join("credentials.toml");
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "no S3 credentials: set FILESYNC_S3_ACCESS_KEY_ID and FILESYNC_S3_SECRET_ACCESS_KEY, \
             or create {} with access_key_id and secret_access_key",
            path.display()
        )
    })?;
    Ok(toml::from_str(&text)?)
}

fn backend(hub: &HubConfig) -> Result<Box<dyn Backend>> {
    Ok(match hub {
        HubConfig::Dir { path } => Box::new(DirBackend::new(path)?),
        HubConfig::S3 {
            endpoint,
            bucket,
            region,
        } => {
            let c = credentials()?;
            Box::new(S3Backend::new(S3Config {
                endpoint: endpoint.clone(),
                bucket: bucket.clone(),
                region: region.clone(),
                access_key_id: c.access_key_id,
                secret_access_key: c.secret_access_key,
            })?)
        }
    })
}

fn open_hub(cfg: &Config) -> Result<Hub> {
    let hub = Hub::new(backend(&cfg.hub)?, Keys::from_master(&load_key()?));
    hub.check_format()?;
    Ok(hub)
}

fn find_root(hub: &Hub, name_or_id: &str) -> Result<(String, String)> {
    let roots = hub.list_roots()?;
    let found: Vec<_> = roots
        .iter()
        .filter(|r| r.meta.name == name_or_id || r.id == name_or_id)
        .collect();
    match found.as_slice() {
        [r] => Ok((r.id.clone(), r.meta.name.clone())),
        [] => bail!("no root named {name_or_id:?}; see `filesync root list`"),
        _ => bail!("several roots are named {name_or_id:?}; use its id"),
    }
}

fn select_mounts<'a>(cfg: &'a Config, root: Option<&str>) -> Result<Vec<&'a Mount>> {
    let ms: Vec<&Mount> = cfg
        .mounts
        .iter()
        .filter(|m| root.is_none_or(|r| m.name == r || m.root_id == r))
        .collect();
    if ms.is_empty() {
        match root {
            Some(r) => bail!("{r:?} is not mounted on this device"),
            None => bail!("nothing is mounted on this device; see `filesync mount`"),
        }
    }
    Ok(ms)
}

fn state_path(m: &Mount) -> Result<PathBuf> {
    Ok(state_dir()?
        .join("mounts")
        .join(format!("{}.json", m.root_id)))
}

fn load_state(m: &Mount) -> Result<MountState> {
    match fs::read(state_path(m)?) {
        Ok(d) => Ok(serde_json::from_slice(&d)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(MountState::default()),
        Err(e) => Err(e.into()),
    }
}

fn save_state(m: &Mount, s: &MountState) -> Result<()> {
    write_private(&state_path(m)?, &serde_json::to_vec(s)?)
}

/// One sync at a time per device: a second CLI or agent waits its turn by failing fast.
struct Lock(PathBuf);

impl Lock {
    fn take() -> Result<Self> {
        let path = state_dir()?.join("lock");
        fs::create_dir_all(path.parent().unwrap())?;
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(_) => Ok(Lock(path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => bail!(
                "another filesync is running (lock file {}). If none is, delete the file",
                path.display()
            ),
            Err(e) => Err(e.into()),
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn print_report(m: &Mount, r: &Report, dry_run: bool) {
    let mode = if m.receive_only {
        " (receive-only)"
    } else {
        ""
    };
    println!("{} → {}{mode}", m.name, m.path.display());
    let lines: [(&str, &Vec<String>); 7] = [
        ("download", &r.downloaded),
        ("delete locally", &r.removed_local),
        ("upload", &r.uploaded),
        ("delete in hub", &r.removed_remote),
        ("CONFLICT, kept as", &r.conflicts),
        ("pending", &r.pending),
        ("warning", &r.warnings),
    ];
    let mut any = false;
    for (label, items) in lines {
        for item in items {
            println!("  {label}: {item}");
            any = true;
        }
    }
    let seq = |s: Option<u64>| s.map_or("none".to_string(), |s| s.to_string());
    match (r.committed_seq, dry_run) {
        (Some(s), _) => println!("  committed snapshot {s}"),
        (None, true) if !any => println!("  up to date (hub snapshot {})", seq(r.remote_seq)),
        (None, true) => println!("  (dry run; hub snapshot {})", seq(r.remote_seq)),
        (None, false) if !any => println!("  up to date (hub snapshot {})", seq(r.remote_seq)),
        (None, false) => println!("  at hub snapshot {}", seq(r.remote_seq)),
    }
}

fn run_mounts(
    cfg: &Config,
    root: Option<&str>,
    push: bool,
    dry_run: bool,
    allow_mass_delete: bool,
) -> Result<()> {
    let hub = open_hub(cfg)?;
    let _lock = if dry_run { None } else { Some(Lock::take()?) };
    let mut failed = false;
    for m in select_mounts(cfg, root)? {
        let mut state = load_state(m)?;
        let opt = Options {
            device: cfg.device.clone(),
            push: push && !m.receive_only,
            dry_run,
            allow_mass_delete,
            max_file_bytes: cfg.limits.max_file_bytes,
        };
        match engine::sync(&hub, &m.root_id, &m.path, &mut state, &opt) {
            Ok(report) => {
                if !dry_run {
                    save_state(m, &state)?;
                }
                print_report(m, &report, dry_run);
            }
            Err(e) => {
                // Save what was applied before the failure, so the next run starts from there.
                if !dry_run {
                    save_state(m, &state)?;
                }
                eprintln!("{}: error: {e:#}", m.name);
                failed = true;
            }
        }
    }
    if failed {
        bail!("one or more roots failed to sync");
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Init {
            device,
            hub_dir,
            s3_endpoint,
            bucket,
            region,
            create_bucket,
            join,
        } => {
            let cfg_path = config_dir()?.join("config.toml");
            if cfg_path.exists() {
                bail!(
                    "{} already exists; this device is already set up",
                    cfg_path.display()
                );
            }
            let hub_cfg = match (hub_dir, s3_endpoint, bucket) {
                (Some(path), None, None) => HubConfig::Dir {
                    path: fs::canonicalize(&path).or_else(|_| {
                        fs::create_dir_all(&path)?;
                        fs::canonicalize(&path)
                    })?,
                },
                (None, Some(endpoint), Some(bucket)) => HubConfig::S3 {
                    endpoint,
                    bucket,
                    region,
                },
                _ => bail!("give either --hub-dir, or --s3-endpoint with --bucket"),
            };
            if create_bucket {
                match &hub_cfg {
                    HubConfig::S3 {
                        endpoint,
                        bucket,
                        region,
                    } => {
                        let c = credentials()?;
                        S3Backend::new(S3Config {
                            endpoint: endpoint.clone(),
                            bucket: bucket.clone(),
                            region: region.clone(),
                            access_key_id: c.access_key_id,
                            secret_access_key: c.secret_access_key,
                        })?
                        .create_bucket()?;
                    }
                    HubConfig::Dir { .. } => bail!("--create-bucket only applies to S3"),
                }
            }
            let master = match &join {
                Some(p) => read_key(p)?,
                None => Keys::generate_master(),
            };
            let hub = Hub::new(backend(&hub_cfg)?, Keys::from_master(&master));
            if join.is_some() {
                hub.check_format()?;
                // Proves the key is right before anything is saved.
                hub.list_roots()
                    .context("this key does not open the hub's roots")?;
            } else {
                hub.init()?;
                if !hub.list_roots()?.is_empty() {
                    bail!("this hub already has roots; use --join with its key file");
                }
            }
            write_private(
                &config_dir()?.join("hub.key"),
                hex::encode(master).as_bytes(),
            )?;
            let cfg = Config {
                device,
                hub: hub_cfg,
                limits: Limits::default(),
                mounts: Vec::new(),
            };
            save_config(&cfg)?;
            println!("device {} set up with hub: {}", cfg.device, hub.describe());
            if join.is_none() {
                println!(
                    "hub key: {}\n  Back it up. Without it, the hub's contents cannot be decrypted.\n  \
                     Other devices join with: filesync init --device <name> ... --join <copy of this file>",
                    config_dir()?.join("hub.key").display()
                );
            }
        }
        Cmd::Root(RootCmd::Create { name }) => {
            let cfg = load_config()?;
            let hub = open_hub(&cfg)?;
            if hub.list_roots()?.iter().any(|r| r.meta.name == name) {
                bail!("a root named {name:?} already exists");
            }
            let id = hub.create_root(&name)?;
            println!("created root {name} ({id})");
        }
        Cmd::Root(RootCmd::List) => {
            let cfg = load_config()?;
            let hub = open_hub(&cfg)?;
            for r in hub.list_roots()? {
                let mounted = cfg
                    .mounts
                    .iter()
                    .find(|m| m.root_id == r.id)
                    .map(|m| format!("mounted at {}", m.path.display()))
                    .unwrap_or_else(|| "not mounted here".into());
                println!("{:<20} {}  {}", r.meta.name, r.id, mounted);
            }
        }
        Cmd::Mount {
            root,
            path,
            receive_only,
        } => {
            let mut cfg = load_config()?;
            let hub = open_hub(&cfg)?;
            let (id, name) = find_root(&hub, &root)?;
            if cfg.mounts.iter().any(|m| m.root_id == id) {
                bail!("{name} is already mounted on this device");
            }
            fs::create_dir_all(&path)?;
            let path = fs::canonicalize(&path)?;
            // ROOT-7: no nested or overlapping roots.
            for m in &cfg.mounts {
                if path.starts_with(&m.path) || m.path.starts_with(&path) {
                    bail!(
                        "{} overlaps {} (mounted at {})",
                        path.display(),
                        m.name,
                        m.path.display()
                    );
                }
            }
            if let Ok(sd) = state_dir() {
                if sd.starts_with(&path) || config_dir()?.starts_with(&path) {
                    bail!(
                        "{} contains FileSync's own configuration or state",
                        path.display()
                    );
                }
            }
            cfg.mounts.push(Mount {
                root_id: id,
                name: name.clone(),
                path: path.clone(),
                receive_only,
            });
            save_config(&cfg)?;
            println!(
                "mounted {name} at {}; run `filesync sync {name}`",
                path.display()
            );
        }
        Cmd::Unmount { root } => {
            let mut cfg = load_config()?;
            let before = cfg.mounts.len();
            let removed: Vec<Mount> = cfg
                .mounts
                .iter()
                .filter(|m| m.name == root || m.root_id == root)
                .cloned()
                .collect();
            cfg.mounts.retain(|m| m.name != root && m.root_id != root);
            if cfg.mounts.len() == before {
                bail!("{root:?} is not mounted on this device");
            }
            save_config(&cfg)?;
            for m in removed {
                let _ = fs::remove_file(state_path(&m)?);
                println!(
                    "unmounted {}; files in {} were left in place",
                    m.name,
                    m.path.display()
                );
            }
        }
        Cmd::Mounts => {
            let cfg = load_config()?;
            for m in &cfg.mounts {
                let st = load_state(m)?;
                println!(
                    "{:<20} {}{}  last synced snapshot {}",
                    m.name,
                    m.path.display(),
                    if m.receive_only {
                        " (receive-only)"
                    } else {
                        ""
                    },
                    st.base_seq.map_or("none".into(), |s| s.to_string())
                );
            }
        }
        Cmd::Status { root } => {
            let cfg = load_config()?;
            run_mounts(&cfg, root.as_deref(), true, true, false)?;
        }
        Cmd::Sync {
            root,
            allow_mass_delete,
        } => {
            let cfg = load_config()?;
            run_mounts(&cfg, root.as_deref(), true, false, allow_mass_delete)?;
        }
        Cmd::Pull {
            root,
            allow_mass_delete,
        } => {
            let cfg = load_config()?;
            run_mounts(&cfg, root.as_deref(), false, false, allow_mass_delete)?;
        }
        Cmd::History { root } => {
            let cfg = load_config()?;
            let hub = open_hub(&cfg)?;
            let (id, name) = find_root(&hub, &root)?;
            println!("{name}:");
            for seq in hub.snapshot_seqs(&id, None)? {
                let s = hub.get_snapshot(&id, seq)?;
                let files = s.entries.values().filter(|e| !e.is_dir()).count();
                println!(
                    "  {:>5}  {}  {:<16} {files} files",
                    s.seq,
                    util::human_utc(s.created_unix),
                    s.device
                );
            }
        }
        Cmd::Restore {
            root,
            seq,
            path,
            to,
        } => {
            let cfg = load_config()?;
            let hub = open_hub(&cfg)?;
            let (id, name) = find_root(&hub, &root)?;
            let n = engine::restore(&hub, &id, seq, path.as_deref(), &to)?;
            println!(
                "restored {n} files of {name} at snapshot {seq} into {}",
                to.display()
            );
        }
        Cmd::Run { interval } => loop {
            let cfg = load_config()?;
            if let Err(e) = run_mounts(&cfg, None, true, false, false) {
                eprintln!("error: {e:#}");
            }
            std::thread::sleep(Duration::from_secs(interval));
        },
    }
    Ok(())
}
