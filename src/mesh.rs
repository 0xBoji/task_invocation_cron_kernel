use std::{
    net::{IpAddr, SocketAddr, UdpSocket},
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result};
use coding_agent_mesh_presence::{
    NetworkInterface, SharedSecretMode, ZeroConfMesh, DEFAULT_MDNS_PORT, DEFAULT_SERVICE_TYPE,
};
use serde::Deserialize;
use tokio::time;
use uuid::Uuid;

use crate::{
    engine::{BoxFuture, MeshProvider},
    models::MeshAgent,
};

#[derive(Debug)]
pub struct CampMeshProvider {
    mesh: ZeroConfMesh,
    local_agent_id: Option<String>,
    discover_delay: Duration,
}

impl CampMeshProvider {
    pub async fn from_env() -> Result<Self> {
        let config_path = std::env::var_os("TICK_CAMP_CONFIG")
            .map(PathBuf::from)
            .or_else(|| {
                let candidate = PathBuf::from(".camp.toml");
                candidate.exists().then_some(candidate)
            });
        let config = match config_path.as_ref() {
            Some(path) => Some(read_camp_config(path)?),
            None => None,
        };

        let discover_ms = std::env::var("TICK_CAMP_DISCOVER_MS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(250);

        let local_agent_id = std::env::var("TICK_LOCAL_AGENT_ID")
            .ok()
            .or_else(|| config.as_ref().map(|cfg| cfg.agent.id.clone()));

        let service_type = config
            .as_ref()
            .map(|cfg| cfg.discovery.service_type.clone())
            .unwrap_or_else(|| DEFAULT_SERVICE_TYPE.to_owned());
        let mdns_port = config
            .as_ref()
            .map(|cfg| cfg.discovery.mdns_port)
            .unwrap_or(DEFAULT_MDNS_PORT);

        let mut builder = ZeroConfMesh::builder()
            .agent_id(format!("tick-observer-{}", Uuid::new_v4()))
            .role("observer")
            .project("tick")
            .branch("daemon")
            .port(ephemeral_udp_port())
            .mdns_port(mdns_port)
            .service_type(service_type)
            .discover_only()
            .heartbeat_interval(Duration::from_millis(200))
            .ttl(Duration::from_secs(2));

        if let Some(config) = &config {
            builder = apply_shared_secret(builder, &config.discovery)?;
            builder = apply_interfaces(builder, &config.discovery)?;
        }

        Ok(Self {
            mesh: builder.build().await?,
            local_agent_id,
            discover_delay: Duration::from_millis(discover_ms),
        })
    }
}

impl MeshProvider for CampMeshProvider {
    fn local_agent_id(&self) -> Option<&str> {
        self.local_agent_id.as_deref()
    }

    fn list_agents<'a>(&'a self) -> BoxFuture<'a, Result<Vec<MeshAgent>>> {
        Box::pin(async move {
            time::sleep(self.discover_delay).await;
            Ok(self
                .mesh
                .agents()
                .await
                .into_iter()
                .map(|agent| MeshAgent::new(agent.id(), agent.role(), agent.status().as_str()))
                .collect())
        })
    }

    fn shutdown<'a>(&'a self) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.mesh.shutdown().await?;
            Ok(())
        })
    }
}

#[derive(Debug, Deserialize)]
struct CampConfigFile {
    agent: CampAgentConfig,
    discovery: CampDiscoveryConfig,
}

#[derive(Debug, Deserialize)]
struct CampAgentConfig {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CampDiscoveryConfig {
    service_type: String,
    mdns_port: u16,
    #[serde(default)]
    shared_secret: Option<String>,
    #[serde(default)]
    shared_secret_accept: Vec<String>,
    #[serde(default)]
    shared_secret_mode: Option<String>,
    #[serde(default)]
    enable_interface: Vec<String>,
    #[serde(default)]
    disable_interface: Vec<String>,
}

fn read_camp_config(path: &PathBuf) -> Result<CampConfigFile> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read `{}`", path.display()))?;
    toml::from_str(&raw).with_context(|| format!("failed to parse `{}`", path.display()))
}

fn apply_shared_secret(
    builder: coding_agent_mesh_presence::ZeroConfMeshBuilder,
    discovery: &CampDiscoveryConfig,
) -> Result<coding_agent_mesh_presence::ZeroConfMeshBuilder> {
    let mode = parse_shared_secret_mode(
        discovery
            .shared_secret_mode
            .as_deref()
            .unwrap_or("sign-and-verify"),
    )?;
    let builder = match &discovery.shared_secret {
        Some(secret) if discovery.shared_secret_accept.is_empty() => {
            builder.shared_secret_with_mode(secret.clone(), mode)
        }
        Some(secret) => builder.shared_secret_rotation_with_mode(
            secret.clone(),
            discovery.shared_secret_accept.clone(),
            mode,
        ),
        None => builder,
    };
    Ok(builder)
}

fn apply_interfaces(
    mut builder: coding_agent_mesh_presence::ZeroConfMeshBuilder,
    discovery: &CampDiscoveryConfig,
) -> Result<coding_agent_mesh_presence::ZeroConfMeshBuilder> {
    for interface in &discovery.enable_interface {
        builder = builder.enable_interface(parse_network_interface(interface)?);
    }
    for interface in &discovery.disable_interface {
        builder = builder.disable_interface(parse_network_interface(interface)?);
    }
    Ok(builder)
}

fn parse_shared_secret_mode(value: &str) -> Result<SharedSecretMode> {
    match value {
        "sign-only" => Ok(SharedSecretMode::SignOnly),
        "sign-and-verify" => Ok(SharedSecretMode::SignAndVerify),
        other => anyhow::bail!("unsupported shared secret mode `{other}`"),
    }
}

fn parse_network_interface(value: &str) -> Result<NetworkInterface> {
    match value {
        "all" => Ok(NetworkInterface::All),
        "ipv4" => Ok(NetworkInterface::IPv4),
        "ipv6" => Ok(NetworkInterface::IPv6),
        "loopback-v4" => Ok(NetworkInterface::LoopbackV4),
        "loopback-v6" => Ok(NetworkInterface::LoopbackV6),
        _ => {
            let Some((kind, raw)) = value.split_once(':') else {
                return Ok(NetworkInterface::Name(value.to_owned()));
            };
            match kind {
                "name" => Ok(NetworkInterface::Name(raw.to_owned())),
                "addr" => raw
                    .parse::<IpAddr>()
                    .map(NetworkInterface::Addr)
                    .map_err(anyhow::Error::from),
                _ => anyhow::bail!("unsupported network interface selector `{value}`"),
            }
        }
    }
}

fn ephemeral_udp_port() -> u16 {
    UdpSocket::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .and_then(|socket| socket.local_addr())
        .map(|address| address.port())
        .unwrap_or(0)
}
