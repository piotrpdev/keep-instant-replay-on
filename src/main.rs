use anyhow::{Context, bail};
use argh::FromArgs;
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::str::FromStr;
use std::time::Duration;
use std::{env, panic, slice, thread};
use std::{ffi::CStr, fmt};
use tracing::level_filters::LevelFilter;
use tracing::{debug, error, info};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use windows::{
    Win32::{
        Foundation::CloseHandle,
        System::Memory::{
            FILE_MAP_READ, MEMORY_BASIC_INFORMATION, MapViewOfFile, OpenFileMappingA,
            UnmapViewOfFile, VirtualQuery,
        },
    },
    core::PCSTR,
};

#[cfg(debug_assertions)]
const DEFAULT_LOG_FILTER: LevelFilter = LevelFilter::DEBUG;
#[cfg(not(debug_assertions))]
const DEFAULT_LOG_FILTER: LevelFilter = LevelFilter::INFO;

#[allow(clippy::doc_markdown)]
#[derive(Clone, Debug, FromArgs)]
/// Enable Nvidia ShadowPlay Instant Replay when it isn't.
struct CliArgs {
    /// how many seconds to wait between checking if Instant Replay is enabled (default: 5)
    #[argh(
        option,
        short = 's',
        default = "Duration::from_secs(5)",
        from_str_fn(seconds_string_to_duration)
    )]
    seconds_between_checks: Duration,

    /// UUID of the memory mapped file with the Nvidia HTTP server port and secret (default: {8BA1E16C-FC54-4595-9782-E370A5FBE8DA})
    #[argh(
        option,
        default = "\"{8BA1E16C-FC54-4595-9782-E370A5FBE8DA}\".to_string()"
    )]
    file_mapping_uuid: String,

    /// maximum allowed region usize of the view of the memory mapped file (default: 4096)
    #[argh(option, default = "4096")]
    max_region_size: usize,

    /// expected usize of the memory mapped file's contents (default: 58)
    #[argh(option, default = "58")]
    expected_contents_size: usize,

    /// endpoint for enabling Instant Replay (default: "/ShadowPlay/v.1.0/InstantReplay/Enable")
    #[argh(
        option,
        default = "\"/ShadowPlay/v.1.0/InstantReplay/Enable\".to_string()"
    )]
    enable_endpoint: String,

    /// path to the log file (default: "{CARGO_CRATE_NAME}.log")
    #[argh(
        option,
        default = "concat!(env!(\"CARGO_CRATE_NAME\"), \".log\").to_string()"
    )]
    log_path: String,
}

#[derive(Clone, Deserialize)]
pub struct LpContents {
    pub port: u16,
    pub secret: String,
}

impl fmt::Debug for LpContents {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "LpContents {{ port: {}, secret: [REDACTED] }}",
            self.port
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StatusBody {
    pub status: bool,
}

fn seconds_string_to_duration(s: &str) -> Result<Duration, String> {
    let seconds = match s.parse::<u64>() {
        Ok(seconds) => seconds,
        Err(e) => return Err(format!("Invalid duration: {s}. Error: {e}")),
    };
    Ok(Duration::from_secs(seconds))
}

fn get_nvidia_http_server_info(
    file_mapping_uuid: &str,
    max_region_size: usize,
    expected_contents_size: usize,
) -> anyhow::Result<String> {
    info!(
        "Attempting to retrieve Nvidia HTTP server port and secret from: {file_mapping_uuid:?}..."
    );

    unsafe {
        debug!("Attempting to open file mapping...");
        let lp_name_cstring =
            CString::from_str(file_mapping_uuid).context("Failed to convert LP name to CString")?;
        let map_handle = OpenFileMappingA(
            FILE_MAP_READ.0,
            false,
            PCSTR(lp_name_cstring.as_bytes_with_nul().as_ptr()),
        )
        .context("Failed to open file mapping")?;
        let _map_handle_guard = scopeguard::guard(map_handle, |handle| {
            if !handle.is_invalid() {
                debug!("Attempting to close file map handle...");
                if let Err(e) = CloseHandle(handle).context("Failed to close file map handle") {
                    error!("{e}");
                }
            }
        });
        if map_handle.is_invalid() {
            bail!("Failed to open file mapping, handle is invalid");
        }

        debug!("Attempting to map view of file...");
        let map_view = MapViewOfFile(map_handle, FILE_MAP_READ, 0, 0, 0);
        let _map_view_guard = scopeguard::guard(map_view, |view| {
            if !view.Value.is_null() {
                debug!("Attempting to unmap view of file...");
                if let Err(e) = UnmapViewOfFile(view).context("Failed to unmap view of file") {
                    error!("{e}");
                }
            }
        });
        if map_view.Value.is_null() {
            bail!("Failed to map view of file, returned null pointer");
        }

        debug!("Attempting to query map view memory information...");
        let mut map_view_memory_info: MEMORY_BASIC_INFORMATION =
            MEMORY_BASIC_INFORMATION::default();
        let query_result = VirtualQuery(
            Some(map_view.Value),
            &mut map_view_memory_info,
            std::mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        );
        if query_result == 0 {
            bail!("Failed to query map view memory information");
        }
        if !(1..=max_region_size).contains(&map_view_memory_info.RegionSize) {
            bail!("Map view memory information region size is not within valid range");
        }

        debug!("Attempting to read map view contents...");
        let p_mapping = map_view.Value as *const u8;
        let raw_slice = slice::from_raw_parts(p_mapping, expected_contents_size + 1);

        debug!("Attempting to convert map view contents to CStr...");
        let c_string = CStr::from_bytes_with_nul(raw_slice)
            .context("Failed to convert map view bytes to CStr")?;
        let parsed_c_string = c_string
            .to_str()
            .context("Failed to convert map view CStr to str")?;

        Ok(parsed_c_string.to_string())
    }
}

fn is_instant_replay_enabled(
    enable_endpoint: &str,
    lp_contents: &LpContents,
) -> anyhow::Result<bool> {
    info!("Attempting to get Instant Replay status...");

    debug!("Attempting to send HTTP GET request to Instant Replay status endpoint...");
    let response = minreq::get(format!(
        "http://localhost:{}{enable_endpoint}",
        lp_contents.port
    ))
    .with_header("X_LOCAL_SECURITY_COOKIE", &lp_contents.secret)
    .send()
    .context("HTTP request to Instant Replay status endpoint failed")?;

    if response.status_code != 200 {
        debug!("Response: {response:?}");
        match response.as_str() {
            Ok(response_body) => {
                debug!("Response body: {:?}", response_body);
            }
            Err(_) => {
                error!("Failed to read response body as string");
            }
        }
        bail!(
            "Failed to get Instant Replay status, status code: {}",
            response.status_code
        );
    }

    let status_body: StatusBody = response
        .json()
        .context("Failed to deserialize Instant Replay status")?;

    Ok(status_body.status)
}

fn enable_instant_replay(enable_endpoint: &str, lp_contents: &LpContents) -> anyhow::Result<()> {
    info!("Attempting to enable Instant Replay...");

    let enable_body = StatusBody { status: true };
    let enable_body_json = serde_json::to_string(&enable_body)
        .context("Failed to serialize payload for Instant Replay endpoint")?;

    debug!("Attempting to send HTTP POST request to Instant Replay endpoint...");
    let response = minreq::post(format!(
        "http://localhost:{}{enable_endpoint}",
        lp_contents.port
    ))
    .with_header("Content-Type", "application/json")
    .with_header("X_LOCAL_SECURITY_COOKIE", &lp_contents.secret)
    .with_body(enable_body_json)
    .send()
    .context("HTTP request to Instant Replay endpoint failed")?;

    if response.status_code != 200 {
        debug!("Response: {response:?}");
        match response.as_str() {
            Ok(response_body) => {
                debug!("Response body: {:?}", response_body);
            }
            Err(_) => {
                error!("Failed to read response body as string");
            }
        }
        bail!(
            "Failed to enable Instant Replay, status code: {}",
            response.status_code
        );
    }

    Ok(())
}

fn setup_logger(log_path: String) -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    println!("Attempting to set up logger...");

    #[cfg(debug_assertions)]
    println!("Attempting to open log file...");
    let log_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_path)
        .context("Failed to open log file")?;

    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer().with_filter(
                LevelFilter::from_str(
                    &env::var("RUST_LOG").unwrap_or_else(|_| "bad_var".to_string()),
                )
                .unwrap_or(DEFAULT_LOG_FILTER),
            ),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(log_file),
        );

    #[cfg(debug_assertions)]
    println!("Attempting to set global default logger...");
    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set default global logger")?;

    debug!("Attempting to set panic hook...");
    panic::set_hook(Box::new(|panic| {
        if let Some(location) = panic.location() {
            error!(
                message = %panic,
                panic.file = location.file(),
                panic.line = location.line(),
                panic.column = location.column(),
            );
        } else {
            error!(message = %panic);
        }
    }));

    Ok(())
}

fn run() -> anyhow::Result<()> {
    let args: CliArgs = argh::from_env();

    #[cfg(debug_assertions)]
    dbg!(&args);

    setup_logger(args.log_path).context("Failed to set up logger")?;

    let lp_contents_string = get_nvidia_http_server_info(
        &args.file_mapping_uuid,
        args.max_region_size,
        args.expected_contents_size,
    )
    .context(format!(
        "Failed to get Nvidia HTTP server info from {:?}",
        args.file_mapping_uuid
    ))?;

    let lp_contents: LpContents =
        serde_json::from_str(&lp_contents_string).context("Failed to deserialize LP JSON")?;

    debug!("{lp_contents:?}");

    loop {
        let is_instant_replay_enabled =
            is_instant_replay_enabled(&args.enable_endpoint, &lp_contents)
                .context("Failed to get Instant Replay status")?;

        if is_instant_replay_enabled {
            info!("Instant Replay is already enabled");
        } else {
            enable_instant_replay(&args.enable_endpoint, &lp_contents)
                .context("Failed to enable Instant Replay")?;

            info!("Instant Replay has been enabled");
        }

        debug!(
            "Sleeping for {:?} before checking status again...",
            args.seconds_between_checks
        );
        thread::sleep(args.seconds_between_checks);
    }
}

fn main() -> anyhow::Result<()> {
    if let Err(e) = run() {
        error!("{e}");
        bail!(e);
    }

    Ok(())
}
